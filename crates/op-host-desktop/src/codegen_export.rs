//! Desktop file export for the Code panel: Download (code file / zip) and
//! Export AI Bundle (structure-bundle zip). Drained each frame.
//!
//! Both actions are raised as `pending_*` flags on `editor_state.codegen`
//! (set in the native property dispatch); this pass clears the flag, pops a
//! native `rfd` save dialog, and writes the bytes — mirroring the Save / Open
//! flow in `persistence.rs`. The rfd dialogs are modal/blocking, which is
//! acceptable here (the same as `persistence::save_as_dialog`).

use std::io::Write;

use op_codegen::ai::types::AssetFile;
use op_host_native::WidgetHostNative;

use crate::codegen_session::CodegenResults;

/// Drain pending Download / Export-Bundle requests from the Code panel.
/// Returns true when either action ran (the cleared flag changed state, so
/// the caller repaints the button's reset state).
pub fn drain_codegen_file_actions(host: &mut WidgetHostNative, results: &CodegenResults) -> bool {
    let mut ran = false;
    if host.editor_state().codegen.pending_download {
        host.editor_state_mut().codegen.pending_download = false;
        ran = true;
        handle_download(host, results);
    }
    if host.editor_state().codegen.pending_export_bundle {
        host.editor_state_mut().codegen.pending_export_bundle = false;
        ran = true;
        handle_export_bundle(host);
    }
    ran
}

/// Save the generated code: a single file when there are no assets, or a
/// `.zip` (`component.<ext>` + `assets/<...>`) when image assets came back.
fn handle_download(host: &WidgetHostNative, results: &CodegenResults) {
    let Some(result) = results.get(
        crate::codegen_session::document_identity(host),
        host.editor_state().codegen.framework,
    ) else {
        return;
    };
    if result.code.is_empty() {
        return;
    }
    if result.assets.is_empty() {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("component.{}", result.framework_ext))
            .add_filter("code", &[result.framework_ext.as_str()])
            .save_file()
        else {
            return;
        };
        if let Err(e) = std::fs::write(&path, &result.code) {
            show_save_error(host, Some(&path), &e.to_string());
        }
    } else {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("component.zip")
            .add_filter("zip", &["zip"])
            .save_file()
        else {
            return;
        };
        let bytes = build_code_zip(&result.code, &result.framework_ext, &result.assets);
        if let Err(e) = std::fs::write(&path, bytes) {
            show_save_error(host, Some(&path), &e.to_string());
        }
    }
}

/// Save the AI structure bundle (`manifest.json` + raw/sanitized views +
/// assets) as `bundle.zip`. The bundle is built FRESH from the live
/// selection (or the active page when nothing is selected) at click time —
/// TS parity (`handleDownloadStructureBundle` calls `getTargetNodes()`);
/// no completed generation is required and a stale generation-time
/// selection is never exported.
fn handle_export_bundle(host: &WidgetHostNative) {
    let Some(bundle) = build_live_structure_bundle(host.editor_state()) else {
        // Nothing to bundle (empty page / unresolvable selection) — the
        // TS handler returns silently on `nodes.length === 0`.
        return;
    };
    let Some(path) = rfd::FileDialog::new()
        .set_file_name("bundle.zip")
        .add_filter("zip", &["zip"])
        .save_file()
    else {
        return;
    };
    let bytes = build_bundle_zip(&bundle.files);
    if let Err(e) = std::fs::write(&path, bytes) {
        show_save_error(host, Some(&path), &e.to_string());
    }
}

/// Build the structure bundle from the LIVE editor state: the selection's
/// subtrees, else the active page's children (the same input-builder the
/// Generate launch uses), asset-sanitized here because the pipeline isn't
/// involved. `None` when there is nothing to bundle.
fn build_live_structure_bundle(
    state: &op_editor_core::EditorState,
) -> Option<op_codegen::ai::bundle::StructureBundle> {
    let (_input, raw) = crate::codegen_input::build_codegen_input(state)?;
    let scope = if state.selection.is_empty() {
        op_codegen::ai::bundle::BundleScope::Page
    } else {
        op_codegen::ai::bundle::BundleScope::Selection
    };
    let (sanitized, assets) = op_codegen::ai::assets::extract_codegen_assets(&raw);
    Some(op_codegen::ai::bundle::build_structure_bundle(
        &raw, &sanitized, &assets, scope,
    ))
}

/// Pop the shared native Save error dialog.
fn show_save_error(host: &WidgetHostNative, path: Option<&std::path::Path>, detail: &str) {
    crate::persistence::show_error_dialog_public(
        host,
        op_host_services::doc_io::ErrorKind::Save,
        path,
        detail,
    );
}

/// Build a Download zip carrying `component.<ext>` (the generated code) plus
/// one `assets/<...>` entry per asset. Stored (uncompressed) so the bytes are
/// byte-stable and fast to write. Asset paths reuse each asset's `zip_path`,
/// which already lives under `assets/`.
fn build_code_zip(code: &str, ext: &str, assets: &[AssetFile]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    // Best-effort: a zip write failure on an in-memory cursor is effectively
    // impossible, so we ignore the Result rather than thread it back to the UI.
    let _ = writer.start_file(format!("component.{ext}"), opts);
    let _ = writer.write_all(code.as_bytes());
    for asset in assets {
        let _ = writer.start_file(asset.zip_path.clone(), opts);
        let _ = writer.write_all(&asset.bytes);
    }
    writer
        .finish()
        .map(|cursor| cursor.into_inner())
        .unwrap_or_default()
}

/// Build a zip from a structure bundle's `(in-zip path, bytes)` entries.
/// Stored (uncompressed).
fn build_bundle_zip(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (path, bytes) in files {
        let _ = writer.start_file(path.clone(), opts);
        let _ = writer.write_all(bytes);
    }
    writer
        .finish()
        .map(|cursor| cursor.into_inner())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    use op_editor_core::codegen::Framework;

    use crate::codegen_session::{pump, CodegenDelta, CodegenSession};

    fn asset(zip_path: &str, bytes: &[u8]) -> AssetFile {
        AssetFile {
            id: "a".into(),
            relative_path: format!("./{zip_path}"),
            zip_path: zip_path.into(),
            mime_type: "image/png".into(),
            bytes: bytes.to_vec(),
            source_node_id: "n1".into(),
        }
    }

    fn entry_names(bytes: &[u8]) -> Vec<String> {
        let archive = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).expect("valid zip");
        archive.file_names().map(|s| s.to_string()).collect()
    }

    fn entry_bytes(bytes: &[u8], name: &str) -> Vec<u8> {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).expect("valid zip");
        let mut entry = archive.by_name(name).expect("zip entry");
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents).expect("read zip entry");
        contents
    }

    fn completed_run(
        framework: Framework,
        code: &str,
        assets: Vec<AssetFile>,
    ) -> Option<CodegenSession> {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(CodegenDelta::Done {
            code: code.into(),
            degraded: false,
            assets,
        })
        .expect("queue completion");
        drop(tx);
        Some(CodegenSession {
            rx,
            finished: false,
            framework,
            document_identity: (0, 0),
            selection_snapshot: Vec::new(),
            model: None,
            cancel: Arc::new(AtomicBool::new(false)),
            run_epoch: 1,
        })
    }

    #[test]
    fn code_zip_without_assets_has_only_the_component() {
        let bytes = build_code_zip("export default function X(){}", "tsx", &[]);
        let names = entry_names(&bytes);
        assert_eq!(names, vec!["component.tsx".to_string()]);
    }

    #[test]
    fn code_zip_with_assets_includes_each_asset_path() {
        let assets = vec![
            asset("assets/img-1.png", &[1, 2, 3]),
            asset("assets/img-2.png", &[4, 5, 6]),
        ];
        let bytes = build_code_zip("code", "vue", &assets);
        let names = entry_names(&bytes);
        assert!(names.contains(&"component.vue".to_string()));
        assert!(names.contains(&"assets/img-1.png".to_string()));
        assert!(names.contains(&"assets/img-2.png".to_string()));
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn download_uses_html_run_after_generating_react_and_switching_back() {
        let mut host = WidgetHostNative::new();
        let mut results = CodegenResults::default();

        host.editor_state_mut().codegen.framework = Framework::Html;
        let mut current = completed_run(
            Framework::Html,
            "<main>html source</main>",
            vec![asset("assets/html.png", &[1, 8, 7])],
        );
        assert!(pump(&mut host, &mut current, &mut results));

        assert!(host
            .editor_state_mut()
            .codegen
            .select_framework(Framework::React));
        current = completed_run(
            Framework::React,
            "export default function ReactSource() {}",
            vec![asset("assets/react.png", &[9, 9, 9])],
        );
        assert!(pump(&mut host, &mut current, &mut results));

        assert!(host
            .editor_state_mut()
            .codegen
            .select_framework(Framework::Html));
        let result = results
            .get(
                crate::codegen_session::document_identity(&host),
                host.editor_state().codegen.framework,
            )
            .expect("HTML result remains cached");
        assert_eq!(result.code, "<main>html source</main>");
        assert_eq!(result.framework_ext, "html");

        let zip = build_code_zip(&result.code, &result.framework_ext, &result.assets);
        assert_eq!(
            entry_bytes(&zip, "component.html"),
            b"<main>html source</main>"
        );
        assert_eq!(entry_bytes(&zip, "assets/html.png"), [1, 8, 7]);
        assert!(!entry_names(&zip).iter().any(|name| name.contains("react")));
    }

    #[test]
    fn completed_download_cache_is_inaccessible_after_document_replacement() {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().codegen.framework = Framework::Html;
        let mut results = CodegenResults::default();
        let mut current = completed_run(
            Framework::Html,
            "<main>old document</main>",
            vec![asset("assets/old.png", &[1, 2, 3])],
        );
        assert!(pump(&mut host, &mut current, &mut results));
        let old_identity = crate::codegen_session::document_identity(&host);
        assert!(results.get(old_identity, Framework::Html).is_some());

        host.replace_editor_state(op_editor_core::EditorState::new());
        let new_identity = crate::codegen_session::document_identity(&host);
        assert_ne!(new_identity, old_identity);
        assert!(
            results.get(new_identity, Framework::Html).is_none(),
            "a new document must not see the previous document's raw assets"
        );
        assert!(
            host.editor_state().codegen.code.is_empty(),
            "the replacement state cannot fall back to painted old code"
        );
    }

    fn rect_node(id: &str) -> jian_ops_schema::node::PenNode {
        use jian_ops_schema::node::{ContainerProps, PenNode, PenNodeBase, RectangleNode};
        use jian_ops_schema::sizing::SizingBehavior;
        PenNode::Rectangle(RectangleNode {
            base: PenNodeBase {
                id: id.to_string(),
                name: Some(id.to_string()),
                x: Some(0.0),
                y: Some(0.0),
                ..Default::default()
            },
            container: ContainerProps {
                width: Some(SizingBehavior::Number(10.0)),
                height: Some(SizingBehavior::Number(10.0)),
                ..Default::default()
            },
            children: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        })
    }

    /// The bundle bytes for a named in-zip path, as UTF-8.
    fn file_text(bundle: &op_codegen::ai::bundle::StructureBundle, path: &str) -> String {
        let bytes = bundle
            .files
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, b)| b.clone())
            .unwrap_or_else(|| panic!("bundle missing {path}"));
        String::from_utf8(bytes).expect("utf8")
    }

    /// Export AI Bundle works PRE-generation, against the live selection —
    /// no completed run is required (TS parity, gap #40).
    #[test]
    fn live_bundle_builds_from_selection_without_generation() {
        let mut state = op_editor_core::EditorState::new();
        state.doc.children = vec![rect_node("n1"), rect_node("n2")];
        state.set_single_selection(op_editor_core::NodeId::new("n1"));

        let bundle = super::build_live_structure_bundle(&state).expect("bundle");
        let raw = file_text(&bundle, "views/raw.json");
        assert!(raw.contains("n1"), "live selected node in the raw view");
        assert!(!raw.contains("n2"), "unselected sibling stays out");
        assert!(file_text(&bundle, "manifest.json").contains("\"scope\": \"selection\""));
    }

    /// With nothing selected the bundle covers the active page's children
    /// (the same fallback the Generate input-builder uses) at page scope.
    #[test]
    fn live_bundle_falls_back_to_page_children_without_selection() {
        let mut state = op_editor_core::EditorState::new();
        state.doc.children = vec![rect_node("n1"), rect_node("n2")];
        state.clear_selection();

        let bundle = super::build_live_structure_bundle(&state).expect("bundle");
        let raw = file_text(&bundle, "views/raw.json");
        assert!(raw.contains("n1") && raw.contains("n2"));
        assert!(file_text(&bundle, "manifest.json").contains("\"scope\": \"page\""));
    }

    /// An empty page with no selection has nothing to bundle (the TS
    /// handler returns silently on zero nodes).
    #[test]
    fn live_bundle_is_none_on_empty_page() {
        let state = op_editor_core::EditorState::new();
        assert!(super::build_live_structure_bundle(&state).is_none());
    }

    #[test]
    fn bundle_zip_contains_every_file_entry() {
        let files = vec![
            ("manifest.json".to_string(), b"{}".to_vec()),
            ("views/raw.json".to_string(), b"[]".to_vec()),
            ("views/sanitized.json".to_string(), b"[]".to_vec()),
            ("assets/img-1.png".to_string(), vec![9, 9, 9]),
        ];
        let bytes = build_bundle_zip(&files);
        let names = entry_names(&bytes);
        assert!(names.contains(&"manifest.json".to_string()));
        assert!(names.contains(&"views/raw.json".to_string()));
        assert!(names.contains(&"views/sanitized.json".to_string()));
        assert!(names.contains(&"assets/img-1.png".to_string()));
        assert_eq!(names.len(), 4);
    }
}
