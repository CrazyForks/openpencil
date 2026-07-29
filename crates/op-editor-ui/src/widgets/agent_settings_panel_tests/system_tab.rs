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
fn system_tab_reserves_space_for_update_status() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::System;
    let panel = AgentSettingsPanel::for_editor(&state);

    assert_eq!(
        panel.content_total_height(),
        378.0,
        "System tab = title + update status + experimental + pencil-cursor picker"
    );
}

#[test]
fn system_tab_paints_each_update_probe_result() {
    let cases = [
        (op_editor_core::UpdateStatus::Idle, "Not checked yet"),
        (op_editor_core::UpdateStatus::Checking, "Checking…"),
        (op_editor_core::UpdateStatus::UpToDate, "Up to date"),
        (
            op_editor_core::UpdateStatus::Available {
                version: "9.8.7".to_string(),
            },
            "Update available v9.8.7",
        ),
        (op_editor_core::UpdateStatus::Error, "Check failed"),
    ];

    for (status, expected) in cases {
        let mut state = EditorState::default();
        state.editor_ui.locale = op_editor_core::Locale::EnUs;
        state.editor_ui.agent_settings.tab = AgentSettingsTab::System;
        state.editor_ui.update_status = status;
        let panel = AgentSettingsPanel::for_editor(&state);
        let rect = panel.rect(1200.0, 800.0);
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        assert!(
            backend
                .text_effective_points
                .iter()
                .any(|(text, _)| text == expected),
            "System tab should paint update status {expected:?}"
        );
    }
}
