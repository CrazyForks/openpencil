//! ACP agent cards: draft form geometry, carets and hover-only actions.
//!
//! Split out of `agent_settings_panel_tests.rs` to keep that file under the
//! 800-line cap.

use super::*;

#[test]
fn agents_tab_acp_cards_replace_empty_hint_height() {
    let empty = AgentSettingsPanel::for_editor(&EditorState::default()).content_total_height();
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_acp_agent();
    state.editor_ui.agent_settings.add_acp_agent();
    let with_acp = AgentSettingsPanel::for_editor(&state).content_total_height();

    assert!(
        with_acp > empty,
        "configured ACP agents should contribute list-card height instead of a fixed empty hint"
    );
}

#[test]
fn agents_content_height_contains_every_provider_card() {
    let state = EditorState::default();
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content = crate::widgets::agent_settings_panel_geometry::content_rect(rect);
    let last = crate::widgets::agent_settings_panel_geometry::agent_card_rect_in(
        rect,
        AgentProvider::ALL.len() - 1,
        &state.editor_ui.agent_settings,
    );

    assert!(
        last.origin.y + last.size.y <= content.origin.y + panel.content_total_height(),
        "scrollable content must include the final CLI provider card"
    );
}

#[test]
fn acp_draft_form_reserves_room_for_args_and_env_fields() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.begin_acp_agent_draft();

    let height =
        crate::widgets::agent_settings_acp::content_height(&state.editor_ui.agent_settings);

    assert!(
        height >= 320.0,
        "ACP draft form should include display name, connection type, command, args, env, and actions"
    );
}

#[test]
fn focused_acp_agent_field_paints_visible_caret_at_blink_on_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_acp_agent();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::AcpAgent {
        index: 0,
        field: AcpAgentField::Command,
    });
    state.editor_ui.settings_input.set_text("node server.js");

    let panel = AgentSettingsPanel::for_editor_at(&state, 100);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert_eq!(caret_fills(&backend.fills, panel.theme.foreground).len(), 1);
}

#[test]
fn focused_empty_acp_command_field_paints_visible_caret_at_blink_on_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_acp_agent();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::AcpAgent {
        index: 0,
        field: AcpAgentField::Command,
    });
    state.editor_ui.settings_input.set_text("");

    let panel = AgentSettingsPanel::for_editor_at(&state, 100);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert_eq!(caret_fills(&backend.fills, panel.theme.foreground).len(), 1);
}

#[test]
fn focused_acp_agent_field_hides_caret_at_blink_off_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_acp_agent();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::AcpAgent {
        index: 0,
        field: AcpAgentField::Command,
    });
    state.editor_ui.settings_input.set_text("node server.js");

    let panel = AgentSettingsPanel::for_editor_at(&state, 500);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(caret_fills(&backend.fills, panel.theme.foreground).is_empty());
}

#[test]
fn acp_agent_compact_actions_are_hover_only_click_targets() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_acp_agent();
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let card_y = content_y + 12.0 + 120.0 + 28.0 + 28.0 + 28.0;

    assert_eq!(
        panel.hit_test(
            rect,
            crate::Point2D::new(content_x + content_w - 156.0, card_y + 30.0)
        ),
        AgentSettingsHit::Inside
    );
    assert_eq!(
        panel.hit_test(
            rect,
            crate::Point2D::new(content_x + content_w - 128.0, card_y + 30.0)
        ),
        AgentSettingsHit::Inside
    );

    state.editor_ui.agent_settings.hover_acp_agent = 0;
    let panel = AgentSettingsPanel::for_editor(&state);
    assert_eq!(
        panel.hit_test(
            rect,
            crate::Point2D::new(content_x + content_w - 156.0, card_y + 30.0)
        ),
        AgentSettingsHit::EditAcpAgent(0)
    );
    assert_eq!(
        panel.hit_test(
            rect,
            crate::Point2D::new(content_x + content_w - 128.0, card_y + 30.0)
        ),
        AgentSettingsHit::RemoveAcpAgent(0)
    );
}
