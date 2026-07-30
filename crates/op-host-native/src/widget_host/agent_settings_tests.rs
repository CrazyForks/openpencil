//! Agent-settings modal press tests for the native host — shared
//! geometry fixtures plus the module spine.
//!
//! The grouped test bodies live in the sibling `agent_settings_tests/`
//! directory so every file stays under the repo's 800-line cap.

use super::WidgetHostNative;
use op_editor_core::agent_settings::{
    AcpAgentField, AgentSettingsTab, BuiltinAgentField, ImageGenField, ImageGenProvider,
    ImageSearchField, ImageTestStatus, SettingsFocus,
};
use op_editor_core::{AgentSettingsButton, BuiltinAgentPresetKey, ButtonPressTarget};
use op_editor_ui::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
use op_editor_ui::Point2D;

mod agents;
mod hover;
mod images;
mod mcp_system;

fn agent_settings_content_metrics(host: &WidgetHostNative) -> (f32, f32, f32) {
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    (
        rect.origin.x + 200.0 + 24.0,
        rect.origin.y + 24.0,
        rect.size.x - 200.0 - 48.0,
    )
}

fn acp_header_y(content_y: f32) -> f32 {
    content_y + 12.0 + 120.0 + 28.0
}

fn acp_card_y(content_y: f32) -> f32 {
    acp_header_y(content_y) + 28.0 + 28.0
}

fn experimental_switch_y(host: &WidgetHostNative, x: f32) -> f32 {
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    let mut hits = (rect.origin.y.ceil() as i32..(rect.origin.y + rect.size.y).floor() as i32)
        .map(|y| y as f32 + 0.5)
        .filter(|y| {
            matches!(
                panel.hit_test(rect, Point2D::new(x, *y)),
                AgentSettingsHit::ToggleExperimental
            )
        });
    let first = hits.next().expect("experimental switch hit region");
    let last = hits.next_back().unwrap_or(first);
    (first + last) / 2.0
}
