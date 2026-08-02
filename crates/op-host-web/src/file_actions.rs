// Browser boundary: these pure helpers are unit-tested; DOM picker/download
// behavior lives in dom_io and still needs browser smoke.
//! Pure (DOM-free) document IO helpers behind the browser file-IO
//! glue (`dom_io`). Everything here is plain Rust over
//! `op_editor_core` / `op_pen_loader` / `op_figma`, so the unit tests
//! exercise the exact serialize / ingest paths the browser uses
//! without a DOM.
//!
//! Mirrors the desktop's `persistence.rs` split: Save serializes
//! `editor_state.doc` straight to canonical `.op` JSON. When the
//! web-canvas daemon has a bound local path it performs the real
//! desktop write, including the `.opmeta` active-page sidecar; the
//! browser fallback downloads the same JSON as a Blob. Open parses
//! the canonical schema via `op_pen_loader::load_canonical`, then
//! re-seeds `EditorState` through `EditorState::from_document`.

use base64::Engine as _;
use op_editor_core::editor_ui_state::ExportFormat;
use op_editor_core::{uikit_io, EditorState, UIKit};

mod drop_plan;
mod export_error;
mod image_fill_upload;
mod ingest_error;
mod save_payload;
mod save_queue;

pub use drop_plan::{drop_batch_plan, drop_kind, DropBatchPlan, DropKind};
pub use export_error::DocumentExportError;
pub use image_fill_upload::apply_fill_image_data_url;
pub use ingest_error::DocumentIngestError;
pub use save_payload::{
    acknowledge_browser_download, parse_save_response, save_ack_matches_document, save_file_name,
    save_snapshot_matches_document, serialize_save_payload, SavePayloadTarget,
};
#[cfg(test)]
pub use save_payload::{save_request_body, serialize_document};
pub use save_queue::LatestSaveQueue;

/// An ingested document plus the loader's best-effort schema
/// warnings (the desktop logs these to stderr; the web glue routes
/// them to `console.warn`).
pub struct IngestedDoc {
    pub state: EditorState,
    pub warnings: Vec<String>,
}

pub fn export_svg_document(state: &EditorState) -> Result<String, DocumentExportError> {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    if state.selection_count() == 1 && state.selection.anchor.is_real() {
        Ok(op_editor_ui::svg_export::serialize_node_svg(
            &scene,
            state.selection.anchor.as_str(),
        )?)
    } else {
        Ok(op_editor_ui::svg_export::serialize_active_page_svg(&scene)?)
    }
}

#[derive(Debug)]
pub struct PdfDownload {
    pub file_name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct RasterDownload {
    pub file_name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

fn document_value_with_editor_meta(
    state: &EditorState,
) -> Result<serde_json::Value, DocumentExportError> {
    let mut document = serde_json::to_value(&state.doc)
        .map_err(|e| DocumentExportError::SerializeDocument(e.to_string()))?;
    let Some(object) = document.as_object_mut() else {
        return Err(DocumentExportError::DocumentNotObject);
    };
    object.insert(
        "editorMeta".to_string(),
        serde_json::to_value(op_pen_loader::EditorMeta::from_state(state))
            .map_err(|e| DocumentExportError::SerializeEditorMeta(e.to_string()))?,
    );
    Ok(document)
}

pub fn export_pdf_request_body(state: &EditorState) -> Result<String, DocumentExportError> {
    let document = document_value_with_editor_meta(state)?;
    serde_json::to_string(&serde_json::json!({
        "document": document,
        "activePageIndex": state.ui.active_page_index,
    }))
    .map_err(|e| DocumentExportError::SerializeRequest(e.to_string()))
}

pub fn parse_pdf_download_response(response: &str) -> Result<PdfDownload, DocumentExportError> {
    let parsed: serde_json::Value = serde_json::from_str(response)
        .map_err(|e| DocumentExportError::ResponseParse(e.to_string()))?;
    if !parsed.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        let message = parsed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("PDF export failed");
        return Err(DocumentExportError::Daemon(message.to_string()));
    }
    let file_name = parsed
        .get("fileName")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("openpencil-export.pdf")
        .to_string();
    let mime = parsed
        .get("mime")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("application/pdf")
        .to_string();
    let data = parsed
        .get("dataBase64")
        .and_then(|v| v.as_str())
        .ok_or(DocumentExportError::PdfMissingData)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| DocumentExportError::PdfDecode(e.to_string()))?;
    Ok(PdfDownload {
        file_name,
        mime,
        bytes,
    })
}

pub fn export_raster_request_body(state: &EditorState) -> Result<String, DocumentExportError> {
    let format = match state.editor_ui.export_format {
        ExportFormat::Png => "png",
        ExportFormat::Jpeg => "jpeg",
        ExportFormat::Webp => "webp",
        ExportFormat::Svg | ExportFormat::Pdf => {
            return Err(DocumentExportError::RasterFormatUnsupported);
        }
    };
    let document = document_value_with_editor_meta(state)?;
    let mut body = serde_json::json!({
        "document": document,
        "activePageIndex": state.ui.active_page_index,
        "format": format,
        "scale": state.editor_ui.export_scale,
    });
    if state.selection_count() == 1 && state.selection.anchor.is_real() {
        body["selectedNodeId"] =
            serde_json::Value::String(state.selection.anchor.as_str().to_string());
    }
    serde_json::to_string(&body).map_err(|e| DocumentExportError::SerializeRequest(e.to_string()))
}

pub fn parse_raster_download_response(
    response: &str,
) -> Result<RasterDownload, DocumentExportError> {
    let parsed: serde_json::Value = serde_json::from_str(response)
        .map_err(|e| DocumentExportError::ResponseParse(e.to_string()))?;
    if !parsed.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        let message = parsed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Raster export failed");
        return Err(DocumentExportError::Daemon(message.to_string()));
    }
    let file_name = parsed
        .get("fileName")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("openpencil-export.png")
        .to_string();
    let mime = parsed
        .get("mime")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("image/png")
        .to_string();
    let data = parsed
        .get("dataBase64")
        .and_then(|v| v.as_str())
        .ok_or(DocumentExportError::RasterMissingData)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| DocumentExportError::RasterDecode(e.to_string()))?;
    Ok(RasterDownload {
        file_name,
        mime,
        bytes,
    })
}

pub fn apply_open_recent_response(
    state: &mut EditorState,
    path: &str,
    response: &str,
    now_secs: u64,
) -> bool {
    let parsed: Option<serde_json::Value> = serde_json::from_str(response).ok();
    let ok = parsed
        .as_ref()
        .and_then(|v| v.get("ok"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if ok {
        state.editor_ui.file_name_display = Some(path_file_name(path).to_string());
        state
            .editor_ui
            .touch_recent_file(path.to_string(), now_secs);
        return true;
    }
    let pruned = parsed
        .as_ref()
        .and_then(|v| v.get("pruned"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    pruned && state.editor_ui.remove_recent_file(path)
}

fn path_file_name(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or(path)
}

pub struct KitExport {
    pub file_name: String,
    pub json: String,
}

pub fn export_kit_document(state: &EditorState) -> Result<Option<KitExport>, DocumentExportError> {
    let kit_name = state
        .doc
        .name
        .clone()
        .unwrap_or_else(|| "My Kit".to_string());
    let Some(kit_doc) = uikit_io::build_kit_document(&state.doc, &[], &kit_name) else {
        return Ok(None);
    };
    let json = serde_json::to_string_pretty(&kit_doc)
        .map_err(|e| DocumentExportError::SerializeKit(e.to_string()))?;
    Ok(Some(KitExport {
        file_name: uikit_io::kit_export_file_name(&kit_name),
        json,
    }))
}

pub fn import_kit_source(src: &str, kit_id: String) -> Result<Option<UIKit>, DocumentIngestError> {
    let loaded = op_pen_loader::load_canonical(src)
        .map_err(|e| DocumentIngestError::LoadCanonical(e.to_string()))?;
    Ok(uikit_io::import_kit_from_document(&loaded.value, kit_id))
}

/// Parse canonical `.op` / `.pen` source into a fresh `EditorState`,
/// carrying over the app-level preferences from `previous` (the
/// state being replaced). Mirrors the desktop's
/// `persistence::load_editor_state` + `preserve_app_preferences`
/// pair, minus the legacy `.opmeta` sidecar. Embedded `editorMeta`
/// restores the active page and Figma Preserve geometry mode; older files
/// without it open on their first non-empty page, matching desktop.
pub fn ingest_op_source(
    src: &str,
    previous: &EditorState,
) -> Result<IngestedDoc, DocumentIngestError> {
    let editor_meta = op_pen_loader::extract_editor_meta(src);
    let loaded = op_pen_loader::load_canonical(src)
        .map_err(|e| DocumentIngestError::LoadCanonical(e.to_string()))?;
    let warnings = loaded.warnings.iter().map(|w| format!("{w:?}")).collect();
    let mut state = EditorState::from_document(loaded.value);
    op_pen_loader::apply_editor_meta_or_legacy_fallback(&mut state, editor_meta);
    preserve_app_preferences(previous, &mut state);
    Ok(IngestedDoc { state, warnings })
}

/// Parse a binary Figma `.fig` export into a fresh `EditorState`.
/// Mirrors the desktop's `figma_import_session::parse_path` body
/// (Preserve layout mode + `preserve_authored_geometry`); the wasm32
/// build has no worker threads, so the caller runs this on the main
/// thread after the async `FileReader` read completes.
pub fn ingest_figma_bytes(
    bytes: &[u8],
    file_name: &str,
) -> Result<IngestedDoc, DocumentIngestError> {
    let import = op_figma::parse_fig_binary(bytes, file_name, op_figma::FigLayoutMode::Preserve)
        .map_err(|e| DocumentIngestError::FigmaParse(e.to_string()))?;
    let mut state = EditorState::from_document(import.document);
    state.editor_ui.preserve_authored_geometry = true;
    Ok(IngestedDoc {
        state,
        warnings: import.warnings,
    })
}

/// Install payload returned by the isolated Figma import Worker.
///
/// `source` is a complete, ordinary canonical `.op` document (including the
/// shared `images` / `imageThumbs` tables produced in the Worker). Routing it
/// through the compatibility loader preserves the exact same old-schema and
/// image-interning behavior as File → Open; no paged placeholder document is
/// ever exposed to `EditorState` in this first phase.
pub fn ingest_figma_temp_source(
    source: &str,
    worker_warnings_json: &str,
) -> Result<IngestedDoc, DocumentIngestError> {
    let loaded = op_pen_loader::load_canonical(source)
        .map_err(|error| DocumentIngestError::LoadCanonical(error.to_string()))?;
    let mut warnings: Vec<String> = serde_json::from_str(worker_warnings_json)
        .map_err(|error| DocumentIngestError::WorkerWarningsParse(error.to_string()))?;
    warnings.extend(loaded.warnings.iter().map(|warning| format!("{warning:?}")));
    let mut state = EditorState::from_document(loaded.value);
    state.editor_ui.preserve_authored_geometry = true;
    Ok(IngestedDoc { state, warnings })
}

#[cfg(test)]
fn ingest_html_project(
    files: &[op_html::HtmlProjectFile],
) -> Result<IngestedDoc, DocumentIngestError> {
    let imported = op_html::import_html_project_document(files, &Default::default())
        .map_err(|error| DocumentIngestError::HtmlProject(error.to_string()))?;
    if imported.document.children.is_empty() {
        return Err(DocumentIngestError::HtmlProjectEmpty);
    }
    Ok(IngestedDoc {
        state: EditorState::from_document(imported.document),
        warnings: imported.warnings,
    })
}

/// Carry app-level preferences from the state being replaced into a
/// freshly ingested one. Port of the desktop's
/// `persistence::preserve_app_preferences` (private there, so
/// duplicated rather than shared — the desktop crate doesn't compile
/// for wasm32).
pub fn preserve_app_preferences(previous: &EditorState, next: &mut EditorState) {
    let previous_selected_model = previous.chat.selected_model_entry().cloned();
    next.editor_ui.theme_mode = previous.editor_ui.theme_mode;
    next.editor_ui.locale = previous.editor_ui.locale;
    next.editor_ui.recent_files = previous.editor_ui.recent_files.clone();
    next.editor_ui.font_import_supported = previous.editor_ui.font_import_supported;
    next.editor_ui.system_fonts_loaded = previous.editor_ui.system_fonts_loaded;
    next.editor_ui.system_font_families = previous.editor_ui.system_font_families.clone();
    next.editor_ui.bundled_font_families = previous.editor_ui.bundled_font_families.clone();
    next.editor_ui.imported_font_families = previous.editor_ui.imported_font_families.clone();
    next.editor_ui.prompt_center.custom_prompts =
        previous.editor_ui.prompt_center.custom_prompts.clone();
    next.editor_ui.prompt_center.custom_store_writable =
        previous.editor_ui.prompt_center.custom_store_writable;
    next.editor_ui.prompt_center.custom_store_dirty =
        previous.editor_ui.prompt_center.custom_store_dirty;
    next.editor_ui.agent_settings = previous.editor_ui.agent_settings.clone();
    next.editor_ui.chat_selected_agent = previous.editor_ui.chat_selected_agent;
    next.chat.discovered_models = previous.chat.discovered_models.clone();
    next.rebuild_chat_models();
    if let Some(prev) = previous_selected_model {
        if let Some(idx) = next.chat.available_models.iter().position(|m| {
            m.provider == prev.provider
                && m.value == prev.value
                && m.builtin_provider_id == prev.builtin_provider_id
        }) {
            next.select_chat_model(idx);
        }
    }
}

/// What a picked / dropped file is, by extension (case-insensitive
/// — matches the desktop's `is_supported_document` /
/// `is_supported_figma_import` semantics).
/// File name without its last extension — node naming for inserted
/// images / SVGs (mirrors the desktop's `Path::file_stem` usage; the
/// browser only hands us a flat name string).
pub fn file_stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(idx) if idx > 0 => &name[..idx],
        _ => name,
    }
}

/// Best-effort MIME type for a chat attachment picked in the browser.
/// Mirrors the desktop attachment helper's extension table.
pub fn attachment_media_type_for_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    match lower.rsplit('.').next() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Browser `File.name` is already pathless in normal cases, but keep
/// the same separator hardening as desktop temp-file staging.
pub fn attachment_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c == '/' || c == '\\' { '_' } else { c })
        .collect();
    if cleaned.trim().is_empty() {
        "attachment".to_string()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::PenNodeExt;

    #[test]
    fn metadata_free_open_uses_first_nonempty_page_and_legacy_layout_mode() {
        let previous = EditorState::new();
        let source = r#"{
          "version":"1.0.0",
          "children":[],
          "pages":[
            {"id":"empty","name":"Empty","children":[]},
            {"id":"content","name":"Content","children":[
              {"type":"rectangle","id":"visible","x":0,"y":0,"width":10,"height":10}
            ]}
          ]
        }"#;

        let ingested = ingest_op_source(source, &previous).expect("legacy document loads");

        assert_eq!(ingested.state.ui.active_page_index, 1);
        assert!(!ingested.state.editor_ui.preserve_authored_geometry);
    }

    #[test]
    fn figma_worker_canonical_source_installs_all_pages_eagerly() {
        let source = r#"{"version":"1.0","pages":[{"id":"p1","name":"One","children":[{"type":"rectangle","id":"a"}]},{"id":"p2","name":"Two","children":[{"type":"rectangle","id":"b"}]}],"children":[]}"#;
        let ingested = ingest_figma_temp_source(source, r#"["worker warning"]"#)
            .expect("worker canonical source loads");
        let pages = ingested.state.doc.pages.as_ref().expect("pages");
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].children.len(), 1);
        assert_eq!(pages[1].children.len(), 1);
        assert!(ingested.state.editor_ui.preserve_authored_geometry);
        assert_eq!(ingested.warnings, ["worker warning"]);
    }

    #[test]
    fn app_preferences_preserve_runtime_font_availability() {
        let mut previous = EditorState::new();
        previous.editor_ui.font_import_supported = true;
        previous.editor_ui.system_fonts_loaded = true;
        previous.editor_ui.system_font_families = std::sync::Arc::new(vec!["PingFang SC".into()]);
        previous.editor_ui.bundled_font_families = std::sync::Arc::new(vec!["Inter".into()]);
        previous.editor_ui.imported_font_families = std::sync::Arc::new(vec!["Brand Sans".into()]);
        let mut next = EditorState::new();

        preserve_app_preferences(&previous, &mut next);

        assert!(next.editor_ui.font_import_supported);
        assert!(next.editor_ui.system_fonts_loaded);
        assert_eq!(&*next.editor_ui.system_font_families, &["PingFang SC"]);
        assert_eq!(&*next.editor_ui.bundled_font_families, &["Inter"]);
        assert_eq!(&*next.editor_ui.imported_font_families, &["Brand Sans"]);
    }

    #[test]
    fn attachment_media_type_matches_desktop_image_extensions() {
        assert_eq!(attachment_media_type_for_name("a.png"), "image/png");
        assert_eq!(attachment_media_type_for_name("a.JPG"), "image/jpeg");
        assert_eq!(attachment_media_type_for_name("a.jpeg"), "image/jpeg");
        assert_eq!(attachment_media_type_for_name("a.gif"), "image/gif");
        assert_eq!(attachment_media_type_for_name("a.webp"), "image/webp");
        assert_eq!(attachment_media_type_for_name("a.svg"), "image/svg+xml");
        assert_eq!(
            attachment_media_type_for_name("notes.txt"),
            "application/octet-stream"
        );
    }

    #[test]
    fn attachment_file_name_strips_path_separators() {
        assert_eq!(attachment_file_name("../a.png"), ".._a.png");
        assert_eq!(attachment_file_name("folder\\a.png"), "folder_a.png");
        assert_eq!(attachment_file_name(""), "attachment");
    }

    #[test]
    fn export_kit_document_builds_download_name_and_json() {
        let src = r#"{"version":"1.0.0","name":"My Kit!","children":[{"type":"frame","id":"button","name":"Primary Button","reusable":true,"x":0,"y":0,"width":120,"height":40,"children":[]}]}"#;
        let doc = op_pen_loader::load_canonical(src)
            .expect("canonical doc")
            .value;
        let state = EditorState::from_document(doc);

        let export = export_kit_document(&state)
            .expect("export encodes")
            .expect("document has reusable components");

        assert_eq!(export.file_name, "My Kit.op");
        let parsed: jian_ops_schema::PenDocument =
            serde_json::from_str(&export.json).expect("kit json");
        assert_eq!(parsed.name.as_deref(), Some("My Kit!"));
        assert_eq!(parsed.children.len(), 1);
        assert_eq!(parsed.children[0].base().id, "button");
    }

    #[test]
    fn save_request_body_embeds_document_and_active_page_index() {
        let src = r#"{"version":"1.0.0","children":[],"pages":[{"id":"p1","name":"One","children":[]},{"id":"p2","name":"Two","children":[{"type":"rectangle","id":"save-node","name":"Save Node","x":0,"y":0,"width":80,"height":40}]}]}"#;
        let doc = op_pen_loader::load_canonical(src)
            .expect("canonical doc")
            .value;
        let mut state = EditorState::from_document(doc);
        assert!(state.set_active_page(1));

        let body = save_request_body(&state).expect("request body");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("json body");

        assert_eq!(
            parsed["document"]["pages"][1]["children"][0]["id"],
            "save-node"
        );
        assert_eq!(parsed["activePageIndex"], 1);
    }

    #[test]
    fn parse_save_response_accepts_daemon_success() {
        let saved = parse_save_response(r#"{"ok":true,"version":3,"fileName":"design.op"}"#)
            .expect("save response");

        assert_eq!(saved.file_name, "design.op");
        assert_eq!(saved.version, Some(3));
    }

    #[test]
    fn parse_save_response_surfaces_daemon_error() {
        let err = parse_save_response(r#"{"ok":false,"error":"No file path"}"#)
            .expect_err("daemon error should fail");

        assert_eq!(err.to_string(), "No file path");
    }

    #[test]
    fn export_pdf_request_body_embeds_current_document() {
        let src = r#"{"version":"1.0.0","children":[],"pages":[{"id":"p1","name":"One","children":[]},{"id":"p2","name":"Two","children":[{"type":"rectangle","id":"pdf-node","name":"PDF Node","x":0,"y":0,"width":80,"height":40}]}]}"#;
        let doc = op_pen_loader::load_canonical(src)
            .expect("canonical doc")
            .value;
        let mut state = EditorState::from_document(doc);
        assert!(state.set_active_page(1));
        state.editor_ui.preserve_authored_geometry = true;

        let body = export_pdf_request_body(&state).expect("request body");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("json body");

        assert_eq!(
            parsed["document"]["pages"][1]["children"][0]["id"],
            "pdf-node"
        );
        assert_eq!(
            parsed["document"]["pages"][1]["children"][0]["name"],
            "PDF Node"
        );
        assert_eq!(parsed["activePageIndex"], 1);
        assert_eq!(parsed["document"]["editorMeta"]["activePageIndex"], 1);
        assert_eq!(
            parsed["document"]["editorMeta"]["preserveAuthoredGeometry"],
            true
        );
    }

    #[test]
    fn parse_pdf_download_response_decodes_daemon_payload() {
        let response = serde_json::json!({
            "ok": true,
            "fileName": "openpencil-export.pdf",
            "mime": "application/pdf",
            "dataBase64": base64::engine::general_purpose::STANDARD.encode(b"%PDF-test%%EOF"),
        })
        .to_string();

        let download = parse_pdf_download_response(&response).expect("pdf response");

        assert_eq!(download.file_name, "openpencil-export.pdf");
        assert_eq!(download.mime, "application/pdf");
        assert_eq!(download.bytes, b"%PDF-test%%EOF");
    }

    #[test]
    fn parse_pdf_download_response_surfaces_daemon_error() {
        let err = parse_pdf_download_response(r#"{"ok":false,"error":"nothing to export"}"#)
            .expect_err("daemon error should fail");

        assert_eq!(err.to_string(), "nothing to export");
    }

    #[test]
    fn export_raster_request_body_embeds_format_scale_document_and_single_selection() {
        let src = r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"raster-node","name":"Raster Node","x":0,"y":0,"width":80,"height":40}]}"#;
        let doc = op_pen_loader::load_canonical(src)
            .expect("canonical doc")
            .value;
        let mut state = EditorState::from_document(doc);
        state.editor_ui.export_format = ExportFormat::Webp;
        state.editor_ui.export_scale = 3.0;
        state.editor_ui.preserve_authored_geometry = true;
        state.set_single_selection(op_editor_core::NodeId::new("raster-node"));

        let body = export_raster_request_body(&state).expect("request body");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("json body");

        assert_eq!(parsed["format"], "webp");
        assert_eq!(parsed["scale"], 3.0);
        assert_eq!(parsed["selectedNodeId"], "raster-node");
        assert_eq!(parsed["activePageIndex"], 0);
        assert_eq!(parsed["document"]["children"][0]["id"], "raster-node");
        assert_eq!(
            parsed["document"]["editorMeta"]["preserveAuthoredGeometry"],
            true
        );
    }

    #[test]
    fn parse_raster_download_response_decodes_daemon_payload() {
        let response = serde_json::json!({
            "ok": true,
            "fileName": "openpencil-export.png",
            "mime": "image/png",
            "dataBase64": base64::engine::general_purpose::STANDARD.encode(b"\x89PNG\r\n\x1a\n"),
        })
        .to_string();

        let download = parse_raster_download_response(&response).expect("raster response");

        assert_eq!(download.file_name, "openpencil-export.png");
        assert_eq!(download.mime, "image/png");
        assert_eq!(download.bytes, b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn parse_raster_download_response_surfaces_daemon_error() {
        let err = parse_raster_download_response(r#"{"ok":false,"error":"nothing to export"}"#)
            .expect_err("daemon error should fail");

        assert_eq!(err.to_string(), "nothing to export");
    }

    #[test]
    fn import_kit_source_extracts_components_with_supplied_id() {
        let src = r#"{"version":"1.0.0","name":"Imported System","children":[{"type":"frame","id":"card","name":"Profile Card","reusable":true,"x":0,"y":0,"width":240,"height":120,"children":[]}]}"#;

        let kit = import_kit_source(src, "web-kit-1".to_string())
            .expect("import parses")
            .expect("source has reusable components");

        assert_eq!(kit.id, "web-kit-1");
        assert_eq!(kit.name, "Imported System");
        assert_eq!(kit.components.len(), 1);
        assert_eq!(kit.components[0].id, "card");
    }

    #[test]
    fn drop_kind_recognizes_html_and_zip() {
        assert!(matches!(drop_kind("page.html"), DropKind::Html));
        assert!(matches!(drop_kind("PAGE.HTM"), DropKind::Html));
        assert!(matches!(drop_kind("site.CSS"), DropKind::HtmlResource));
        assert!(matches!(drop_kind("ui.WOFF2"), DropKind::HtmlResource));
        assert!(matches!(drop_kind("brand.otf"), DropKind::HtmlResource));
        assert!(matches!(drop_kind("app.mjs"), DropKind::HtmlResource));
        assert!(matches!(
            drop_kind("manifest.webmanifest"),
            DropKind::HtmlResource
        ));
        assert!(matches!(drop_kind("favicon.ico"), DropKind::Image));
        assert!(matches!(drop_kind("photo.avif"), DropKind::Image));
        assert!(matches!(drop_kind("saved-page.ZIP"), DropKind::Zip));
        assert!(matches!(drop_kind("a.svg"), DropKind::Svg));
    }

    #[test]
    fn drop_batch_plan_groups_html_with_explicit_resources() {
        assert_eq!(
            drop_batch_plan(&[
                DropKind::Html,
                DropKind::HtmlResource,
                DropKind::Image,
                DropKind::Svg,
            ]),
            DropBatchPlan::HtmlProject
        );
        assert_eq!(
            drop_batch_plan(&[DropKind::Html, DropKind::Html]),
            DropBatchPlan::HtmlProject
        );
    }

    #[test]
    fn drop_batch_plan_rejects_html_document_figma_and_unknown_mixes() {
        for conflict in [DropKind::Document, DropKind::Figma, DropKind::Unsupported] {
            assert_eq!(
                drop_batch_plan(&[DropKind::Html, conflict]),
                DropBatchPlan::InvalidHtmlMix
            );
        }
    }

    #[test]
    fn drop_batch_plan_rejects_zip_mixes_and_keeps_other_drops_individual() {
        assert_eq!(drop_batch_plan(&[DropKind::Zip]), DropBatchPlan::HtmlZip);
        assert_eq!(
            drop_batch_plan(&[DropKind::Zip, DropKind::Html]),
            DropBatchPlan::InvalidZipMix
        );
        assert_eq!(
            drop_batch_plan(&[DropKind::Image, DropKind::Svg]),
            DropBatchPlan::Individual
        );
        assert_eq!(
            drop_batch_plan(&[DropKind::HtmlResource]),
            DropBatchPlan::Individual
        );
    }

    #[test]
    fn ingest_html_project_resolves_relative_stylesheets_and_images() {
        let files = vec![
            op_html::HtmlProjectFile {
                relative_path: "pages/index.html".into(),
                bytes: br#"<link rel="stylesheet" href="../assets/site.css">
                    <div class="hero"></div>"#
                    .to_vec(),
            },
            op_html::HtmlProjectFile {
                relative_path: "assets/site.css".into(),
                bytes: br#".hero { width: 40px; height: 30px;
                    background-image: url('./hero icon.png?v=1'); }"#
                    .to_vec(),
            },
            op_html::HtmlProjectFile {
                relative_path: "assets/hero icon.png".into(),
                bytes: vec![0x89, 0x50, 0x4e, 0x47, 1, 2, 3],
            },
        ];

        let ingested = ingest_html_project(&files).expect("saved-page project imports");
        let json = serde_json::to_string(&ingested.state.doc).expect("document serializes");

        assert!(json.contains("data:image/png;base64,"));
        assert!(!ingested
            .warnings
            .iter()
            .any(|warning| warning.contains("external stylesheet skipped")));
    }

    #[test]
    fn ingest_html_project_prefers_index_and_rejects_missing_html() {
        let files = vec![
            op_html::HtmlProjectFile {
                relative_path: "other.html".into(),
                bytes: b"<h1>Other</h1>".to_vec(),
            },
            op_html::HtmlProjectFile {
                relative_path: "INDEX.HTML".into(),
                bytes: b"<h1>Index chosen</h1>".to_vec(),
            },
        ];
        let ingested = ingest_html_project(&files).expect("index candidate imports");
        let json = serde_json::to_string(&ingested.state.doc).expect("document serializes");
        assert!(json.contains("Index chosen"));
        assert!(!json.contains("Other"));

        assert!(ingest_html_project(&[op_html::HtmlProjectFile {
            relative_path: "style.css".into(),
            bytes: b"body {}".to_vec(),
        }])
        .is_err());
    }
}
