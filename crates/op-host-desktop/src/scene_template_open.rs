//! Desktop arm for opening a scene template as a new document.
//!
//! The panel raises `scene_template_center.pending_open` rather than loading
//! anything itself (see `scene_template_press_flow`); this drains it.
//!
//! A template becomes an UNSAVED document with no bound path, like File > New
//! rather than File > Open: the shipped `.op` is read-only content, and
//! binding it as the save target would let the first Cmd+S overwrite the
//! template for every future use.

use op_editor_core::scene_template_catalog::{scene_template_by_id, scene_template_document};
use op_host_native::widget_host::WidgetHostNative;
use std::path::PathBuf;

use op_host_services::doc_io::preserve_app_preferences;

use crate::persistence::{fit_loaded_document, refresh_title};

/// Drain a pending template-open request. Returns true when the document was
/// replaced.
pub(crate) fn drain_pending_scene_template(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    let Some(template_id) = host
        .editor_state_mut()
        .editor_ui
        .scene_template_center
        .take_pending_open()
    else {
        return false;
    };
    // Replacing the document runs the same collaboration gate as File > New;
    // the panel's own press only opened a panel, which needed no gate.
    if !host.gate_collaboration_action(
        op_editor_core::CollabGateAction::ReplaceDocument,
        op_editor_core::CollabEditSource::User,
    ) {
        return false;
    }
    let Some(source) = scene_template_document(&template_id) else {
        // The catalogue verifies this at load, so reaching here means the
        // embedded set and the catalogue disagree — report it rather than
        // silently doing nothing under the user's click.
        eprintln!("[template] no embedded document for {template_id}");
        return false;
    };
    let locale = host.editor_state().editor_ui.locale;
    let mut state = match op_host_services::doc_io::load_editor_state_from_source(source, locale) {
        Ok(state) => state,
        Err(error) => {
            eprintln!("[template] {template_id}: {error}");
            return false;
        }
    };
    preserve_app_preferences(host.editor_state(), &mut state);
    stamp_template_scenario(&mut state, &template_id);
    state.clear_selection();
    if !host.replace_editor_state(state) {
        return false;
    }
    fit_loaded_document(host, window);
    // No bound path: this is a new unsaved document, so Save must prompt for
    // a destination instead of writing back over the shipped template.
    *current_path = None;
    refresh_title(current_path, window);
    host.force_rotate_layer_panel_owner();
    host.mark_editor_state_dirty();
    true
}

/// Tag the freshly loaded state with what the template it came from IS.
///
/// The catalogue entry states the scene — a deck, a carousel, a tutorial set
/// — so this is a recorded fact rather than a guess about the content, and it
/// rides into `editorMeta` on the first save. An id with no catalogue entry
/// leaves the document untagged; an unknown template is exactly the case
/// where we know nothing.
fn stamp_template_scenario(state: &mut op_editor_core::EditorState, template_id: &str) {
    state.editor_ui.scenario = scene_template_by_id(template_id).map(|template| template.scene);
}

/// Whether any template document fails to load, used by the smoke test below
/// and worth keeping cheap: it parses every shipped template.
#[cfg(test)]
fn all_templates_parse(locale: op_editor_core::Locale) -> Result<(), String> {
    use op_editor_core::EditorState;
    for template in op_editor_core::scene_template_catalog::scene_template_catalogue() {
        let source = scene_template_document(&template.id)
            .ok_or_else(|| format!("{} has no document", template.id))?;
        let state: EditorState =
            op_host_services::doc_io::load_editor_state_from_source(source, locale)
                .map_err(|error| format!("{}: {error}", template.id))?;
        if state.active_children().is_empty() {
            return Err(format!("{} loaded with no top-level nodes", template.id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every shipped template must survive the real loader.
    ///
    /// The catalogue test only checks the JSON shape; this one runs the same
    /// path a click takes, so a template that parses as JSON but fails schema
    /// conversion cannot reach a user as a click that does nothing.
    #[test]
    fn every_shipped_template_loads_through_the_real_loader() {
        all_templates_parse(op_editor_core::Locale::ZhCn).expect("templates load");
        all_templates_parse(op_editor_core::Locale::EnUs).expect("templates load");
    }

    /// Opening a template records what the document is for.
    ///
    /// Every shipped template is checked, not just the deck: the tag comes
    /// from the catalogue entry, so a template added without a scene — or
    /// wired to the wrong one — fails here rather than reaching a user as a
    /// preview that behaves like the wrong kind of document.
    #[test]
    fn opening_a_template_stamps_its_catalogue_scene() {
        use op_editor_core::scene_template_catalog::{scene_template_catalogue, TemplateScene};

        for template in scene_template_catalogue() {
            let mut state = op_host_services::doc_io::load_editor_state_from_source(
                template.document(),
                op_editor_core::Locale::EnUs,
            )
            .expect("template loads");
            assert_eq!(state.editor_ui.scenario, None, "{} before", template.id);

            stamp_template_scenario(&mut state, &template.id);

            assert_eq!(
                state.editor_ui.scenario,
                Some(template.scene),
                "{} after",
                template.id
            );
        }

        let deck = scene_template_by_id("slide-deck").expect("the deck template ships");
        assert_eq!(deck.scene, TemplateScene::Slides);
    }

    /// An id the catalogue does not know leaves the document untagged rather
    /// than inheriting the previous document's scenario.
    #[test]
    fn an_unknown_template_id_clears_the_scenario() {
        let mut state = op_editor_core::EditorState::new();
        state.editor_ui.scenario =
            Some(op_editor_core::scene_template_catalog::TemplateScene::Slides);

        stamp_template_scenario(&mut state, "not-a-template");

        assert_eq!(state.editor_ui.scenario, None);
    }
}
