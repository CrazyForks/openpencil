//! System tab: the auto-update card and its click target.
//!
//! Split out of `agent_settings_panel_tests.rs` to keep that file under the
//! 800-line cap.

use super::*;

#[test]
fn system_auto_update_switch_has_click_target() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::System;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let card_y = content_y + 12.0 + 36.0;
    let point = crate::Point2D::new(content_x + content_w - 28.0, card_y + 28.0);

    assert_eq!(
        panel.hit_test(rect, point),
        AgentSettingsHit::ToggleAutoUpdate
    );
}

#[test]
fn system_tab_uses_ts_compact_auto_update_card_height() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::System;
    let panel = AgentSettingsPanel::for_editor(&state);

    assert_eq!(
        panel.content_total_height(),
        320.0,
        "System tab = title + auto-update + experimental + pencil-cursor picker"
    );
}
