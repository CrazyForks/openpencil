//! MCP tab: client-config copy feedback, port field focus and carets.
//!
//! Split out of `agent_settings_panel_tests.rs` to keep that file under the
//! 800-line cap.

use super::*;

#[test]
fn mcp_running_client_config_paints_copy_icon_like_ts() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.mcp_server.running = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let client_config_y = content_y + 36.0 + 52.0 + 8.0;
    let icon_origin = Point2D::new(content_x + content_w - 27.0, client_config_y + 13.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend.icon_strokes.iter().any(|(at, size, _)| {
            (*size - 10.0).abs() < 0.01
                && (at.x - icon_origin.x).abs() < 0.01
                && (at.y - icon_origin.y).abs() < 0.01
        }),
        "running MCP client config should expose a TS-like copy icon button"
    );
}

#[test]
fn copied_mcp_client_config_paints_check_feedback_like_ts() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.mcp_server.running = true;
    state
        .editor_ui
        .agent_settings
        .mcp_client_config_copied_at_ms = Some(1_000);
    let panel = AgentSettingsPanel::for_editor_at(&state, 1_500);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let check_paths = Icon::Check.paths();
    let copy_paths = Icon::Copy.paths();
    assert!(
        backend
            .svg_strokes
            .iter()
            .any(|(path, _, _)| check_paths.contains(&path.as_str())),
        "recent MCP client config copy should replace the copy glyph with check feedback"
    );
    assert!(
        !backend
            .svg_strokes
            .iter()
            .any(|(path, _, _)| copy_paths.contains(&path.as_str())),
        "copy glyph should not be visible while copied feedback is active"
    );
}

#[test]
fn expired_mcp_client_config_copy_feedback_restores_copy_icon() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.mcp_server.running = true;
    state
        .editor_ui
        .agent_settings
        .mcp_client_config_copied_at_ms = Some(1_000);
    let panel = AgentSettingsPanel::for_editor_at(&state, 3_100);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let copy_paths = Icon::Copy.paths();
    assert!(
        backend
            .svg_strokes
            .iter()
            .any(|(path, _, _)| copy_paths.contains(&path.as_str())),
        "expired MCP client config copy feedback should restore the copy glyph"
    );
}

#[test]
fn mcp_running_client_config_copy_icon_is_clickable() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.mcp_server.running = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let client_config_y = content_y + 36.0 + 52.0 + 8.0;

    assert_eq!(
        panel.hit_test(
            rect,
            Point2D::new(content_x + content_w - 22.0, client_config_y + 18.0)
        ),
        AgentSettingsHit::CopyMcpClientConfig
    );
}

#[test]
fn mcp_port_field_is_not_focusable_while_server_is_running() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.mcp_server.running = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let server_card_top = content_y + 36.0;
    let button_x = content_x + content_w - 16.0 - 72.0;
    let port_x = button_x - 8.0 - 64.0;
    let point = crate::Point2D::new(port_x + 32.0, server_card_top + 26.0);

    assert_eq!(panel.hit_test(rect, point), AgentSettingsHit::Inside);
}

#[test]
fn mcp_running_server_exposes_client_config_height() {
    let mut stopped_state = EditorState::default();
    stopped_state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    let stopped = AgentSettingsPanel::for_editor(&stopped_state).content_total_height();

    let mut running_state = EditorState::default();
    running_state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    running_state.editor_ui.agent_settings.mcp_server.running = true;
    let running = AgentSettingsPanel::for_editor(&running_state).content_total_height();

    assert!(
        running > stopped,
        "running MCP server should reserve space for the HTTP client config row"
    );
}

#[test]
fn focused_mcp_port_paints_visible_caret_at_blink_on_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::McpPort);
    state.editor_ui.settings_input.set_text("3845");

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
fn focused_mcp_port_hides_caret_at_blink_off_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::McpPort);
    state.editor_ui.settings_input.set_text("3845");

    let panel = AgentSettingsPanel::for_editor_at(&state, 500);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(caret_fills(&backend.fills, panel.theme.foreground).is_empty());
}
