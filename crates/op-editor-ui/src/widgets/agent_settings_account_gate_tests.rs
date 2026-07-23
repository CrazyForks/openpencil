use crate::widgets::agent_settings_panel::AgentSettingsPanel;
use op_editor_core::agent_settings::AgentSettingsTab;
use op_editor_core::EditorState;

#[test]
fn account_tab_release_gate_hides_nav_and_falls_back_from_stale_state() {
    const { assert!(!crate::widgets::ACCOUNT_UI_AVAILABLE) };
    let default_state = EditorState::default();
    let default_panel = AgentSettingsPanel::for_editor(&default_state);
    let expected_agents_height = default_panel.content_total_height();

    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Account;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);

    let expected_tabs = [
        AgentSettingsTab::Agents,
        AgentSettingsTab::Mcp,
        AgentSettingsTab::Images,
        AgentSettingsTab::Fonts,
        AgentSettingsTab::System,
    ];
    for (index, expected) in expected_tabs.into_iter().enumerate() {
        let point = crate::Point2D::new(
            rect.origin.x + 100.0,
            rect.origin.y + 56.0 + index as f32 * 30.0 + 14.0,
        );
        assert_eq!(panel.nav_at(rect, point), Some(expected));
    }

    let hidden_account_row = crate::Point2D::new(
        rect.origin.x + 100.0,
        rect.origin.y + 56.0 + expected_tabs.len() as f32 * 30.0 + 14.0,
    );
    assert_eq!(panel.nav_at(rect, hidden_account_row), None);
    assert_eq!(
        panel.content_total_height(),
        expected_agents_height,
        "a persisted Account tab must fall back to the first visible tab"
    );
}
