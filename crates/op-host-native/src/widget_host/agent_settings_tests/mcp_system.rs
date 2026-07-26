//! MCP tab (server start/stop, client-config copy) and System tab
//! (auto-update, experimental gate) presses.
//!
//! Split out of `agent_settings_tests.rs` to keep every file under the
//! repo's 800-line cap.

use super::*;

#[test]
fn starting_mcp_server_commits_port_draft_and_clears_focus() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::McpPort);
    host.editor_state_mut()
        .editor_ui
        .settings_input
        .set_text("3101");

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let server_card_top = content_y + 36.0;
    let button_x = content_x + content_w - 16.0 - 72.0;
    assert!(host.dispatch_agent_settings_press(
        button_x + 36.0,
        server_card_top + 26.0,
        1200.0,
        800.0
    ));

    let state = host.editor_state();
    assert!(state.editor_ui.agent_settings.mcp_server.running);
    assert_eq!(state.editor_ui.agent_settings.mcp_server.port, 3101);
    assert!(state.editor_ui.agent_settings.focus.is_none());
    assert!(state.editor_ui.settings_input.text().is_empty());
    assert_eq!(
        state.editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::McpServer
        ))
    );

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn copy_mcp_client_config_queues_clipboard_text() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .mcp_server
        .running = true;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .mcp_server
        .port = 4123;

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let client_config_y = content_y + 36.0 + 52.0 + 8.0;
    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 22.0,
        client_config_y + 18.0,
        1200.0,
        800.0
    ));

    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some("{\n  \"type\": \"http\",\n  \"url\": \"http://127.0.0.1:4123/mcp\"\n}")
    );
}

#[test]
fn system_auto_update_switch_toggles_preference() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::System;
    assert!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .auto_update_enabled
    );

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let card_y = content_y + 12.0 + 36.0;
    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 28.0,
        card_y + 28.0,
        1200.0,
        800.0
    ));

    assert!(
        !host
            .editor_state()
            .editor_ui
            .agent_settings
            .auto_update_enabled
    );
}

#[test]
fn system_experimental_switch_toggles_preference() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::System;
    assert!(
        !host
            .editor_state()
            .editor_ui
            .agent_settings
            .experimental_features_enabled
    );

    let (cx, cy, cw) = agent_settings_content_metrics(&host);
    assert!(host.dispatch_agent_settings_press(
        cx + cw - 28.0,
        experimental_switch_y(cy),
        1200.0,
        800.0
    ));

    assert!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .experimental_features_enabled
    );
}

/// Preview graduated out of the experimental-features gate (2026-07): the
/// Play button + preview interaction are now a regular always-on feature,
/// so toggling the experimental gate off no longer force-exits a live
/// preview session (contrast `disabling_experimental_clears_widget_
/// property_focus` below — Widget-config stays gated and still clears).
#[test]
fn disabling_experimental_leaves_active_preview_running() {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"frame","id":"root","name":"Root","x":0,"y":0,"width":200,"height":200,
             "children":[
               {"type":"rectangle","id":"r","name":"R","x":10,"y":10,"width":50,"height":50}
             ]}
        ]}"#,
    )
    .expect("fixture parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .experimental_features_enabled = true;
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::System;

    assert!(host.enter_preview((800.0, 600.0)), "preview should enter");
    assert!(host.preview_active());

    let (cx, cy, cw) = agent_settings_content_metrics(&host);
    assert!(host.dispatch_agent_settings_press(
        cx + cw - 28.0,
        experimental_switch_y(cy),
        1200.0,
        800.0
    ));

    assert!(
        !host
            .editor_state()
            .editor_ui
            .agent_settings
            .experimental_features_enabled
    );
    assert!(
        host.preview_active(),
        "disabling experimental must NOT exit the live preview session"
    );
    assert!(host.editor_state().editor_ui.preview.mode);
}

#[test]
fn disabling_experimental_clears_widget_property_focus() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .experimental_features_enabled = true;
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::System;
    // A widget field still holds focus when the gate flips off.
    host.editor_state_mut().ui.property_focus =
        Some(op_editor_core::PropertyFocus::WidgetPlaceholder);

    let (cx, cy, cw) = agent_settings_content_metrics(&host);
    assert!(host.dispatch_agent_settings_press(
        cx + cw - 28.0,
        experimental_switch_y(cy),
        1200.0,
        800.0
    ));

    assert!(
        host.editor_state().ui.property_focus.is_none(),
        "stale Widget property focus must be cleared so it can't commit"
    );
}

#[test]
fn copying_mcp_client_config_records_feedback_time() {
    let mut host = WidgetHostNative::new();
    host.set_now_ms(4_321);
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .mcp_server
        .running = true;

    let (content_x, content_y, content_w) = agent_settings_content_metrics(&host);
    let client_config_y = content_y + 36.0 + 52.0 + 8.0;

    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 22.0,
        client_config_y + 18.0,
        1200.0,
        800.0
    ));

    assert_eq!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .mcp_client_config_copied_at_ms,
        Some(4_321)
    );
    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some("{\n  \"type\": \"http\",\n  \"url\": \"http://127.0.0.1:3100/mcp\"\n}")
    );
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::McpClientConfigCopy
        ))
    );

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}
