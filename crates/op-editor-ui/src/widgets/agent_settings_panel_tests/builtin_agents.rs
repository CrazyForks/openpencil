//! Built-in agent cards: field hit-tests, carets, drafts and compact actions.
//!
//! Split out of `agent_settings_panel_tests.rs` to keep that file under the
//! 800-line cap.

use super::*;

#[test]
fn hit_test_resolves_builtin_agent_api_key_field() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_builtin_agent();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::ApiKey,
    });
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let first_card_y = content_y + 12.0 + 28.0 + 28.0;
    let point = crate::Point2D::new(content_x + 92.0, first_card_y + 116.0);

    assert_eq!(
        panel.hit_test(rect, point),
        AgentSettingsHit::FocusBuiltinAgent {
            index: 0,
            field: BuiltinAgentField::ApiKey,
        }
    );
}

#[test]
fn focused_builtin_agent_field_paints_visible_caret_at_blink_on_phase() {
    let mut state = EditorState::default();
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MINIMAX", "", "MiniMax-M2.7");
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::BaseUrl,
    });
    state
        .editor_ui
        .settings_input
        .set_text("https://api.minimaxi.com/v1");

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
fn focused_builtin_agent_field_hides_caret_at_blink_off_phase() {
    let mut state = EditorState::default();
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MINIMAX", "", "MiniMax-M2.7");
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::BaseUrl,
    });
    state
        .editor_ui
        .settings_input
        .set_text("https://api.minimaxi.com/v1");

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
fn focused_builtin_agent_field_paints_from_settings_draft() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_builtin_agent();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::ApiKey,
    });
    state.editor_ui.settings_input.set_text("sk-draft");

    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let first_card_y = content_y + 12.0 + 28.0 + 28.0;
    let api_key_input = Rect {
        origin: Point2D::new(content_x + 12.0 + 68.0, first_card_y + 76.0 + 28.0),
        size: Point2D::new(content_w - 24.0 - 68.0, 24.0),
    };
    let api_key_baseline_y = api_key_input.origin.y + 16.0;
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert_eq!(
        panel.settings.focus,
        Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::ApiKey,
        })
    );
    let draft_point = backend
        .text_effective_points
        .iter()
        .find_map(|(text, point)| (text == "sk-draft").then_some(*point))
        .expect("focused settings draft should paint");
    assert!(
        api_key_input.contains(draft_point),
        "focused settings draft text should render inside the API key field"
    );
    assert!(
        (draft_point.y - api_key_baseline_y).abs() < 0.01,
        "focused settings draft text should use the same baseline as unfocused text"
    );
}

#[test]
fn builtin_agent_cards_use_ts_compact_height_when_not_editing() {
    let mut state = EditorState::default();
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("DeepSeek", "sk-test", "deepseek-v4-pro");
    let panel = AgentSettingsPanel::for_editor(&state);

    // Header + subtitle + two TS-style compact cards with one gap
    // after each card. The pre-parity expanded form was >400 px.
    assert_eq!(
        panel.settings.builtin_agents.len(),
        2,
        "fixture should exercise multiple compact provider cards"
    );
    assert_eq!(
        crate::widgets::agent_settings_builtin::content_height(&panel.settings),
        192.0,
        "the built-in section should stay compact independently of the CLI provider count"
    );
}

#[test]
fn hit_test_resolves_builtin_agent_compact_switch() {
    let mut state = EditorState::default();
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let content_y = rect.origin.y + 24.0;
    let first_card_y = content_y + 12.0 + 28.0 + 28.0;
    let point = crate::Point2D::new(content_x + content_w - 90.0, first_card_y + 30.0);

    assert_eq!(
        panel.hit_test(rect, point),
        AgentSettingsHit::ToggleBuiltinAgentEnabled(0)
    );
}

#[test]
fn builtin_agent_compact_edit_button_is_a_hover_only_click_target() {
    let mut state = EditorState::default();
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let content_y = rect.origin.y + 24.0;
    let first_card_y = content_y + 12.0 + 28.0 + 28.0;
    let point = crate::Point2D::new(content_x + content_w - 52.0, first_card_y + 30.0);

    assert_eq!(panel.hit_test(rect, point), AgentSettingsHit::Inside);

    state.editor_ui.agent_settings.hover_builtin_agent = 0;
    let panel = AgentSettingsPanel::for_editor(&state);
    assert_eq!(
        panel.hit_test(rect, point),
        AgentSettingsHit::EditBuiltinAgent(0)
    );
}
