//! System tab: the auto-update row and its click target.
//!
//! Split out of `agent_settings_panel_tests.rs` to keep that file under the
//! 800-line cap.

use super::*;
use crate::widgets::agent_settings_rows::{tab_hero_height, ROW_HEIGHT};

/// Centre-y of System row `index`, below the tab's hero block.
fn system_row_mid_y(rect: Rect, index: usize) -> f32 {
    let content = crate::widgets::agent_settings_panel::content_viewport(rect);
    content.origin.y + tab_hero_height(1) + index as f32 * ROW_HEIGHT + ROW_HEIGHT / 2.0
}

#[test]
fn system_auto_update_switch_has_click_target() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::System;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content = crate::widgets::agent_settings_panel::content_viewport(rect);
    // Row order is Appearance, Auto-update, Experimental, Pencil cursor.
    let point = crate::Point2D::new(
        content.origin.x + content.size.x - 18.0,
        system_row_mid_y(rect, 1),
    );

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
        tab_hero_height(1) + 4.0 * ROW_HEIGHT + 32.0 + 24.0,
        "System tab = hero + Appearance/Auto-update/Experimental/Pencil-cursor rows \
         + version footnote + bottom pad"
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
