//! Desktop arm of the Styles tab's `DESIGN.md` import.
//!
//! The Asset Center never touches the filesystem: it raises requests and this
//! drains them, the same split the template-open path uses. Three of them, and
//! they are genuinely different jobs — open a file dialog, write a guide the
//! shared flow already registered, delete one it already forgot.
//!
//! Persist and delete are deliberately *after the fact*. Memory is the source
//! of truth for what the catalogue contains, so the card list, the pin, and
//! the grid are all correct the instant the user acts, whether or not the disk
//! cooperates. A write that fails costs the guide at next launch and says so;
//! it does not make the button appear broken now.

use op_host_native::widget_host::WidgetHostNative;

use crate::user_style_store;

/// Drain every pending style-import request. Returns whether anything changed
/// on screen.
pub(crate) fn drain_pending_style_import(host: &mut WidgetHostNative) -> bool {
    let mut changed = drain_pending_file_pick(host);
    changed |= drain_pending_persist(host);
    changed |= drain_pending_delete(host);
    changed
}

/// Ask for a `.md` and import it.
fn drain_pending_file_pick(host: &mut WidgetHostNative) -> bool {
    if !host
        .editor_state_mut()
        .editor_ui
        .scene_template_center
        .take_pending_style_import_file()
    {
        return false;
    }
    let locale = host.editor_state().editor_ui.locale;
    let picked = rfd::FileDialog::new()
        .set_title(op_i18n::translate(locale, "assetCenter.style.importTitle"))
        .add_filter("Markdown", &["md", "markdown"])
        .pick_file();
    let Some(path) = picked else {
        return true;
    };
    match user_style_store::import_style_guide_file(&path) {
        Ok(id) => {
            // Pin what was just imported, matching the paste path: the user
            // went and found this guide, and leaving it unpinned would make
            // the next generation ignore it.
            host.editor_state_mut().editor_ui.pinned_style_guide = Some(id);
            host.editor_state_mut()
                .editor_ui
                .scene_template_center
                .import
                .error_key = None;
        }
        Err(error) => {
            eprintln!("[styles] {}: import failed", path.display());
            host.editor_state_mut()
                .editor_ui
                .scene_template_center
                .import
                .error_key = Some(error.message_key());
            crate::message_dialog::alert(
                op_i18n::translate(locale, "assetCenter.style.importTitle"),
                &format!(
                    "{}\n\n{}",
                    path.display(),
                    op_i18n::translate(locale, error.message_key())
                ),
                rfd::MessageLevel::Warning,
            );
        }
    }
    host.mark_editor_state_dirty();
    true
}

/// Write guides the shared flow registered but could not store.
fn drain_pending_persist(host: &mut WidgetHostNative) -> bool {
    let ids = host
        .editor_state_mut()
        .editor_ui
        .scene_template_center
        .take_pending_style_persist();
    if ids.is_empty() {
        return false;
    }
    for id in ids {
        if let Err(error) = user_style_store::persist_user_style_guide(&id) {
            // Reported, not surfaced as a dialog: the guide is live and
            // usable this session, and the only casualty is that it will not
            // be there after a restart.
            eprintln!("[styles] {id}: could not be written: {error}");
        }
    }
    false
}

/// Remove the files of guides the shared flow already forgot.
fn drain_pending_delete(host: &mut WidgetHostNative) -> bool {
    let ids = host
        .editor_state_mut()
        .editor_ui
        .scene_template_center
        .take_pending_style_delete();
    if ids.is_empty() {
        return false;
    }
    for id in ids {
        if let Err(error) = user_style_store::delete_user_style_guide(&id) {
            eprintln!("[styles] {id}: could not be deleted: {error}");
        }
    }
    false
}
