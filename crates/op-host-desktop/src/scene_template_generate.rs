//! Desktop arm for the Scene Template Center's prompt-to-deck row.
//!
//! The panel records a topic and closes (see `scene_template_press_flow`);
//! this turns that topic into a document. Two halves, in this order:
//!
//! 1. Replace the document with a blank starter — the same `EditorState`
//!    File > New produces, not a template. A deck generated onto whatever
//!    the user had open would add slides to their work rather than build
//!    them a deck, and the scenario gate downstream reads the same emptiness
//!    to decide the document *is* a deck (`chat_session_launch_design`).
//! 2. Send the wrapped topic as an ordinary chat turn, so the generate row
//!    owns no pipeline of its own: whatever the chat path does for a typed
//!    deck request is exactly what it does here.

use op_editor_core::scene_template_prompt::slides_generate_prompt;
use op_editor_core::EditorState;
use op_host_native::widget_host::WidgetHostNative;
use std::path::PathBuf;

use op_host_services::doc_io::preserve_app_preferences;

use crate::persistence::{fit_loaded_document, refresh_title};

/// Drain a pending generate request. Returns true when a document was
/// replaced and a turn queued.
pub(crate) fn drain_pending_scene_template_generate(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    let Some(topic) = host
        .editor_state_mut()
        .editor_ui
        .scene_template_center
        .take_pending_generate()
    else {
        return false;
    };
    let locale = host.editor_state().editor_ui.locale;
    // The widget already rejects a blank topic; this is the second reader of
    // the same rule, and it must not invent a turn out of nothing either.
    let Some(prompt) = slides_generate_prompt(locale, &topic) else {
        return false;
    };
    // Replacing the document runs the same collaboration gate as File > New.
    if !host.gate_collaboration_action(
        op_editor_core::CollabGateAction::ReplaceDocument,
        op_editor_core::CollabEditSource::User,
    ) {
        return false;
    }

    let mut state = EditorState::starter();
    preserve_app_preferences(host.editor_state(), &mut state);
    if !host.replace_editor_state(state) {
        return false;
    }
    fit_loaded_document(host, window);
    host.editor_state_mut().mark_saved_revision();
    host.force_rotate_layer_panel_owner();
    // No bound path: like File > New, the first Save must ask where.
    *current_path = None;
    refresh_title(current_path, window);

    queue_generate_turn(host.editor_state_mut(), &prompt);
    host.mark_editor_state_dirty();
    true
}

/// Put the wrapped topic through the same door a typed message uses.
///
/// Writing the input and calling `begin_send` rather than reaching for the
/// provider directly is the point: the turn then carries the transcript
/// bubbles, the auto-title, and the agent selection every other send gets,
/// and the host's existing `launch_chat_if_pending` drains it.
fn queue_generate_turn(state: &mut EditorState, prompt: &str) {
    state.chat.input.set_text(prompt);
    state.chat.begin_send();
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::scene_template_catalog::TemplateScene;

    /// The two facts the scenario gate downstream reads before stamping a
    /// document as a deck: the prompt classifies as slides, and the page it
    /// lands on is the untouched starter.
    ///
    /// Asserted here rather than only in `chat_session_launch_design` because
    /// this entry point is what has to keep supplying them — a future change
    /// that seeded the new document with content, or dropped the wrapper,
    /// would leave the gate correct and this door broken.
    #[test]
    fn a_generated_turn_satisfies_both_halves_of_the_scenario_gate() {
        let mut state = EditorState::starter();
        assert!(
            crate::chat_session::active_page_is_blank_starter_frame(&state),
            "the document a generate request starts from is the pristine starter"
        );

        let prompt = slides_generate_prompt(state.editor_ui.locale, "Q3 复盘").expect("wraps");
        queue_generate_turn(&mut state, &prompt);

        let sent = state.chat.pending_send.clone().expect("a turn is queued");
        assert_eq!(sent, prompt, "the wrapped topic is what gets sent");
        assert_eq!(
            crate::chat_session::design_turn_scenario(&sent, true),
            Some(TemplateScene::Slides)
        );

        // Queuing a turn must not itself put anything on the canvas, or the
        // gate's second half would fail by the time the turn launches.
        assert!(crate::chat_session::active_page_is_blank_starter_frame(
            &state
        ));
    }

    /// A user message and a streaming assistant bubble, so the generated deck
    /// arrives in a transcript the user can read and follow up in — not as a
    /// document that appeared with no record of what was asked.
    #[test]
    fn the_turn_lands_in_the_transcript_like_any_other_message() {
        let mut state = EditorState::starter();
        assert!(state.chat.messages.is_empty());

        queue_generate_turn(&mut state, "为以下主题制作一份演示文稿（PPT）：Q3 复盘");

        assert_eq!(state.chat.messages.len(), 2, "user message plus assistant");
        assert!(
            state.chat.messages[0].content.contains("Q3 复盘"),
            "the user bubble shows what was asked"
        );
        assert!(
            state.chat.input.text().is_empty(),
            "the chat input is left clean for the user's next message"
        );
        assert!(!state.chat.collapsed, "the panel opens to show the turn");
    }
}
