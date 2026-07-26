//! Desktop `.pen` / `.op` Save and Open dialog flow.
//!
//! Owns native pickers, [`run_action`] routing, and error dialogs.

use std::path::PathBuf;

use op_editor_core::EditorState;
use op_host_native::WidgetHostNative;
#[cfg(test)]
use op_host_services::doc_io::active_page_bbox;
use op_host_services::doc_io::{
    load_editor_state_with_report, preserve_app_preferences, save_to_path, set_file_name_display,
    ActionOutcome, ErrorKind,
};

/// Document extensions for the native file-dialog filter (`.op` is the
/// canonical format, `.pen` the legacy alias). Order is cosmetic — save
/// dialogs always seed an explicit `.op` file name.
pub(crate) const DOCUMENT_EXTENSIONS: &[&str] = &["op", "pen"];

/// Pop a Save dialog (rfd native) and write the current document to
/// the chosen path. `Ok(Some(path))` on success, `Ok(None)` on user
/// cancel, `Err` on IO / encode failure.
pub fn save_as_dialog(state: &EditorState) -> Result<Option<PathBuf>, String> {
    let Some(path) = pick_save_as_path(state) else {
        return Ok(None);
    };
    save_to_path(state, &path)?;
    Ok(Some(path))
}

/// Pop only the native Save-As picker. The ordinary desktop path uses this
/// before handing serialization to [`crate::save_session`]; synchronous
/// close/reload confirmation keeps using [`save_as_dialog`].
pub fn pick_save_as_path(state: &EditorState) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title(op_i18n::translate(
            state.editor_ui.locale,
            "dialog.pickerSaveTitle",
        ))
        .add_filter(op_editor_ui::PRODUCT_NAME, DOCUMENT_EXTENSIONS)
        .set_file_name("untitled.op")
        .save_file()
}

/// Cmd+S — save to `current_path` if known, else fall through to
/// Save As. Updates `current_path` + window title on success.
/// Returns `true` when the document was written to disk, `false` on
/// an IO error or a cancelled Save-As dialog — so the caller can
/// tell a real save from a no-op (e.g. the unsaved-changes prompt).
pub fn handle_save(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    if let Some(path) = current_path.clone() {
        match save_to_path(host.editor_state(), &path) {
            Err(e) => {
                eprintln!("[save] {e}");
                show_error_dialog(host, ErrorKind::Save, Some(&path), &e);
                return false;
            }
            Ok(()) => {
                crate::settings_io::touch_recent(host, &path);
                set_display_name(host, Some(&path));
                host.editor_state_mut().mark_saved_revision();
                return true;
            }
        }
    }
    handle_save_as(host, current_path, window)
}

fn set_display_name(host: &mut WidgetHostNative, path: Option<&std::path::Path>) {
    set_file_name_display(host.editor_state_mut(), path);
}

fn viewport_size_for_window(window: Option<&winit::window::Window>) -> (f32, f32) {
    window
        .map(|w| {
            let size = w.inner_size();
            let scale = w.scale_factor() as f32;
            (size.width as f32 / scale, size.height as f32 / scale)
        })
        .unwrap_or((super::INITIAL_VIEWPORT_W, super::INITIAL_VIEWPORT_H))
}

fn fit_loaded_document(host: &mut WidgetHostNative, window: Option<&winit::window::Window>) {
    let (vw, vh) = viewport_size_for_window(window);
    host.fit_content_to_viewport(vw, vh);
    host.mark_editor_state_dirty();
}

/// Cmd+Shift+S — always pop the Save dialog.
pub fn handle_save_as(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    match save_as_dialog(host.editor_state()) {
        Ok(Some(path)) => {
            crate::settings_io::touch_recent(host, &path);
            // Mirror handle_save: refresh the in-chrome file name too, not
            // just the OS window title — without this, first Save As writes
            // the file but the TopBar keeps showing "Untitled".
            set_display_name(host, Some(&path));
            host.editor_state_mut().mark_saved_revision();
            *current_path = Some(path);
            refresh_title(current_path, window);
            true
        }
        Ok(None) => false,
        Err(e) => {
            eprintln!("[save as] {e}");
            show_error_dialog(host, ErrorKind::Save, None, &e);
            false
        }
    }
}

fn load_into_host(host: &mut WidgetHostNative, path: &std::path::Path) -> Result<PathBuf, String> {
    let loaded_source_state = crate::figma_import_session::capture_output_state(path)?;
    let locale = host.editor_state().editor_ui.locale;
    let loaded = load_editor_state_with_report(path, locale);
    crate::heap_pressure::schedule_relief("document load parse");
    let loaded = loaded?;
    let mut state = loaded.state;
    preserve_app_preferences(host.editor_state(), &mut state);
    let bound_path = crate::legacy_op_upgrade::prompt_and_save(
        &mut state,
        path,
        &loaded.report,
        loaded_source_state,
    )?;
    set_file_name_display(&mut state, Some(&bound_path));
    state.clear_selection();
    eprintln!(
        "[open] {} active-page top-level nodes",
        state.active_children().len()
    );
    host.replace_editor_state(state);
    host.editor_state_mut().mark_saved_revision();
    host.force_rotate_layer_panel_owner();
    host.mark_editor_state_dirty();
    host.arm_missing_fonts_detection();
    Ok(bound_path)
}

/// Cmd+O — pop the Open dialog and replace the current document.
pub fn handle_open(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    let path = match rfd::FileDialog::new()
        .set_title(op_i18n::translate(
            host.editor_state().editor_ui.locale,
            "dialog.pickerOpenTitle",
        ))
        .add_filter(op_editor_ui::PRODUCT_NAME, DOCUMENT_EXTENSIONS)
        .pick_file()
    {
        Some(p) => p,
        None => return false,
    };
    match load_into_host(host, &path) {
        Ok(bound_path) => {
            fit_loaded_document(host, window);
            crate::settings_io::touch_recent(host, &bound_path);
            *current_path = Some(bound_path);
            refresh_title(current_path, window);
            true
        }
        Err(e) => {
            eprintln!("[open] {e}");
            show_error_dialog(host, ErrorKind::Open, Some(&path), &e);
            false
        }
    }
}

/// Open a drag/drop or file-association path without a picker.
pub fn open_path(
    host: &mut WidgetHostNative,
    path: PathBuf,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    match load_into_host(host, &path) {
        Ok(bound_path) => {
            fit_loaded_document(host, window);
            crate::settings_io::touch_recent(host, &bound_path);
            *current_path = Some(bound_path);
            refresh_title(current_path, window);
            true
        }
        Err(e) => {
            eprintln!("[open] {e}");
            show_error_dialog(host, ErrorKind::Open, Some(&path), &e);
            false
        }
    }
}

/// Build the layout-resolved scene from the live editor state and
/// dispatch its configured export format to `path`.
fn export_editor_state_to_path(state: &EditorState, path: &std::path::Path) -> Result<(), String> {
    use op_editor_core::editor_ui_state::ExportFormat as Fmt;

    let fmt = state.editor_ui.export_format;
    let scale = state.editor_ui.export_scale;
    // A single selected node scopes raster and SVG exports to that
    // subtree. PDF remains page-level.
    let single_node = if state.selection_count() == 1 && state.selection.anchor.is_real() {
        Some(state.selection.anchor.as_str())
    } else {
        None
    };
    if fmt == Fmt::Pdf {
        // PDF is intentionally multi-page; keep the full builder for it.
        let scene = op_pen_loader::editor_state_to_layout_scene(state);
        return op_host_services::export_pdf::export_pdf(&scene, path);
    }

    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let raster = |rf: op_host_services::export::RasterFormat| match single_node {
        Some(id) => op_host_services::export::export_node_raster(&scene, id, path, rf, scale),
        None => op_host_services::export::export_raster(&scene, path, rf, scale),
    };
    match fmt {
        Fmt::Png => raster(op_host_services::export::RasterFormat::Png),
        Fmt::Jpeg => raster(op_host_services::export::RasterFormat::Jpeg),
        Fmt::Webp => raster(op_host_services::export::RasterFormat::Webp),
        Fmt::Svg => match single_node {
            Some(id) => op_host_services::export::export_node_svg(&scene, id, path),
            None => op_host_services::export::export_svg(&scene, path),
        },
        Fmt::Pdf => unreachable!("PDF returned before active-page scene construction"),
    }
}

/// Route a `FileAction` raised by the file-menu dispatcher to the
/// matching dialog flow. The returned [`ActionOutcome`] tells the
/// runner which post-action bookkeeping to run — see its variant
/// docs.
pub fn run_action(
    action: op_editor_core::editor_ui_state::FileAction,
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> ActionOutcome {
    use op_editor_core::editor_ui_state::FileAction;
    match action {
        FileAction::New => {
            let mut state = EditorState::starter();
            preserve_app_preferences(host.editor_state(), &mut state);
            host.replace_editor_state(state);
            let (vw, vh) = viewport_size_for_window(window);
            host.fit_content_to_viewport(vw, vh);
            host.editor_state_mut().mark_saved_revision();
            // Fresh starter document restarts at revision 0 / page 0 — rotate the
            // LayerPanel cache owner so the next paint rebuilds (same aliasing as
            // the Open path).
            host.force_rotate_layer_panel_owner();
            host.mark_editor_state_dirty();
            *current_path = None;
            refresh_title(current_path, window);
            ActionOutcome::Saved
        }
        FileAction::Open => ActionOutcome::saved_or_noop(handle_open(host, current_path, window)),
        FileAction::Save => ActionOutcome::saved_or_noop(handle_save(host, current_path, window)),
        FileAction::SaveAs => {
            ActionOutcome::saved_or_noop(handle_save_as(host, current_path, window))
        }
        FileAction::ExportImage => {
            // main.rs intercepts ExportImage to open the picker; this
            // fallback keeps external callers working.
            let ui = &mut host.editor_state_mut().editor_ui;
            ui.image_panel.close_popovers();
            ui.export_dialog_open = true;
            host.mark_editor_state_dirty();
            ActionOutcome::Noop
        }
        FileAction::ExportImageConfirm => {
            use op_editor_core::editor_ui_state::ExportFormat as Fmt;
            let fmt = host.editor_state().editor_ui.export_format;
            let (filter_label, filter_exts): (&str, &[&str]) = match fmt {
                Fmt::Png => ("PNG", &["png"]),
                Fmt::Jpeg => ("JPEG", &["jpg", "jpeg"]),
                Fmt::Webp => ("WEBP", &["webp"]),
                Fmt::Svg => ("SVG", &["svg"]),
                Fmt::Pdf => ("PDF", &["pdf"]),
            };
            let default_name = format!("openpencil-export.{}", fmt.extension());
            if let Some(path) = rfd::FileDialog::new()
                .set_title(op_i18n::translate(
                    host.editor_state().editor_ui.locale,
                    "dialog.pickerExportTitle",
                ))
                .add_filter(filter_label, filter_exts)
                .set_file_name(&default_name)
                .save_file()
            {
                let result = export_editor_state_to_path(host.editor_state(), &path);
                if let Err(e) = result {
                    eprintln!("[export-image] {e}");
                    show_error_dialog(host, ErrorKind::Export, Some(&path), &e);
                }
            }
            ActionOutcome::Noop
        }
        FileAction::OpenRecent(i) => {
            let Some(entry) = host.editor_state().editor_ui.recent_files.get(i).cloned() else {
                return ActionOutcome::Noop;
            };
            let path = std::path::PathBuf::from(&entry.path);
            match load_into_host(host, &path) {
                Ok(bound_path) => {
                    fit_loaded_document(host, window);
                    crate::settings_io::touch_recent(host, &bound_path);
                    *current_path = Some(bound_path);
                    refresh_title(current_path, window);
                    ActionOutcome::Saved
                }
                Err(e) => {
                    // File missing / parse failure → tell the user and
                    // drop the stale entry from recents.
                    eprintln!("[open-recent] {e}; pruning {}", entry.path);
                    show_error_dialog(host, ErrorKind::Open, Some(&path), &e);
                    host.editor_state_mut()
                        .editor_ui
                        .recent_files
                        .retain(|r| r.path != entry.path);
                    host.mark_editor_state_dirty();
                    ActionOutcome::Noop
                }
            }
        }
        FileAction::ClearRecent => {
            host.editor_state_mut().editor_ui.recent_files.clear();
            host.mark_editor_state_dirty();
            ActionOutcome::Noop
        }
        FileAction::ImportHtml => {
            let path = match rfd::FileDialog::new()
                .set_title(op_i18n::translate(
                    host.editor_state().editor_ui.locale,
                    "dialog.pickerOpenTitle",
                ))
                .add_filter("HTML / ZIP", &["html", "htm", "zip"])
                .pick_file()
            {
                Some(p) => p,
                None => return ActionOutcome::Noop,
            };
            // Same worker-thread rationale as the Figma branch: the CSS
            // cascade + resource fetch takes seconds on a real page.
            ActionOutcome::HtmlImportStarted(path)
        }
        FileAction::ImportFigma => {
            let path = match rfd::FileDialog::new()
                .set_title(op_i18n::translate(
                    host.editor_state().editor_ui.locale,
                    "dialog.pickerOpenTitle",
                ))
                .add_filter("Figma", &["fig"])
                .pick_file()
            {
                Some(p) => p,
                None => return ActionOutcome::Noop,
            };
            // Spawn the parse on a worker thread so the UI keeps
            // repainting (a 2–3 MB .fig with hundreds of nodes takes
            // multiple seconds; running it on the main thread freezes
            // the window). The desktop runner picks up the session in
            // the next `RedrawRequested` pump and applies the result
            // when it lands.
            ActionOutcome::FigmaImportStarted(path)
        }
        FileAction::FinishFigmaImport(selection) => ActionOutcome::FigmaImportSelection(selection),
        FileAction::ImportImageOrSvg => {
            crate::persistence_image::handle_import_image_or_svg(host);
            ActionOutcome::Noop
        }
        FileAction::PickFillImage => {
            crate::persistence_image::handle_pick_fill_image(host);
            ActionOutcome::Noop
        }
        FileAction::RelinkImage => {
            crate::persistence_image::handle_relink_image(host);
            ActionOutcome::Noop
        }
    }
}

// `import_figma_into_host` (synchronous parse) was retired in favour
// of `figma_import_session::spawn_approved`, which moves the parse to a worker
// thread and pumps the result back through a channel each frame.

fn refresh_title(current_path: &Option<PathBuf>, window: Option<&winit::window::Window>) {
    let Some(window) = window else { return };
    let title = match current_path.as_ref().and_then(|p| p.file_name()) {
        Some(name) => format!(
            "{} — {}",
            name.to_string_lossy(),
            op_editor_ui::PRODUCT_NAME
        ),
        None => op_editor_ui::PRODUCT_NAME.to_string(),
    };
    window.set_title(&title);
}

/// Pop a native error dialog. Used by Open / Save / Export when the
/// underlying IO or parse step fails.
fn show_error_dialog(
    host: &WidgetHostNative,
    kind: ErrorKind,
    path: Option<&std::path::Path>,
    detail: &str,
) {
    let locale = host.editor_state().editor_ui.locale;
    let (title_key, lead_key) = match kind {
        ErrorKind::Open => ("dialog.openErrorTitle", "dialog.openErrorLead"),
        ErrorKind::Save => ("dialog.saveErrorTitle", "dialog.saveErrorLead"),
        ErrorKind::Export => ("dialog.exportErrorTitle", "dialog.exportErrorLead"),
    };
    let mut body = op_i18n::translate(locale, lead_key).to_string();
    if let Some(p) = path {
        body.push_str("\n\n");
        body.push_str(&p.display().to_string());
    }
    body.push_str("\n\n");
    body.push_str(detail);
    rfd::MessageDialog::new()
        .set_title(op_i18n::translate(locale, title_key))
        .set_description(&body)
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// Public re-export of the native error dialog — used by the
/// background Figma import session (`figma_import_session::pump`) to
/// pop the same OS dialog the synchronous error path uses.
pub fn show_error_dialog_public(
    host: &WidgetHostNative,
    kind: ErrorKind,
    path: Option<&std::path::Path>,
    detail: &str,
) {
    show_error_dialog(host, kind, path, detail)
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod tests;
