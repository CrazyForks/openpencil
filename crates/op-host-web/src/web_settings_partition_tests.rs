//! Account-partition isolation: what an empty partition must clear, and what
//! the rebuilt persistence baselines must preserve.
//!
//! Split out of `web_settings_tests.rs` at the 800-line cap; nested under it
//! so `use super::*` still reaches the shared helpers.

use super::*;

#[test]
fn an_empty_partition_clears_the_previous_accounts_credentials() {
    // The account-switch leak: after switching, the in-memory state still held
    // account A's API keys, and an empty partition for B meant "keep whatever
    // was there" instead of "no keys".
    let mut state = EditorState::new();
    let mut writes = Vec::new();
    // A signs in and stores a key.
    let credential_json = r#"{"version":2,"builtin_agents":[{"id":"a1","preset":"custom",
        "display_name":"A's model","kind":"openai-compat","api_key":"sk-account-a",
        "model":"m","base_url":"https://api.example.com/v1","enabled":true}],
        "image_gen_profiles":[],"active_image_gen_profile_id":null,"openverse_oauth":null}"#;
    super::storage::load_into_with(&mut state, None, Some(credential_json), |k, v| {
        writes.push((k.to_string(), v.to_string()));
        true
    });
    assert!(
        !state.editor_ui.agent_settings.builtin_agents.is_empty(),
        "A's credential must load in the first place"
    );

    // B signs in: their partition is empty.
    super::storage::load_into_with(&mut state, None, None, |_, _| true);
    assert!(
        state.editor_ui.agent_settings.builtin_agents.is_empty(),
        "B must not inherit A's API keys from an empty partition"
    );
}

#[test]
fn an_empty_partition_resets_account_scoped_settings_to_defaults() {
    // `apply_payload` only writes fields the blob carries, so an empty
    // partition used to leave A's locale, recent files and provider config in
    // place for B.
    let mut state = EditorState::new();
    state.editor_ui.locale = Locale::Ja;
    state.editor_ui.recent_files = vec![RecentFile {
        path: "/a/secret-project.op".into(),
        modified_at: 1,
    }];
    state.editor_ui.agent_settings.mcp_server.port = 4242;
    state.editor_ui.agent_settings.openverse_client_id = "a-client".into();

    super::reset_account_scoped_settings(&mut state);
    super::storage::load_into_with(&mut state, None, None, |_, _| true);

    let defaults = op_editor_core::EditorUiState::default();
    let default_agents = op_editor_core::AgentSettings::default();
    assert_eq!(state.editor_ui.locale, defaults.locale, "locale must reset");
    assert!(
        state.editor_ui.recent_files.is_empty(),
        "B must not see A's recent files"
    );
    assert_eq!(
        state.editor_ui.agent_settings.mcp_server.port,
        default_agents.mcp_server.port
    );
    assert!(state
        .editor_ui
        .agent_settings
        .openverse_client_id
        .is_empty());
}

#[test]
fn a_populated_partition_still_wins_over_the_defaults() {
    // The reset must not erase the partition being loaded — defaults first,
    // then the target snapshot on top.
    let mut state = EditorState::new();
    state.editor_ui.locale = Locale::Ja;
    let settings = r#"{"version":1,"locale":"fr","mcp_port":5150}"#;

    super::reset_account_scoped_settings(&mut state);
    super::storage::load_into_with(&mut state, Some(settings), None, |_, _| true);

    assert_eq!(state.editor_ui.locale, Locale::Fr);
    assert_eq!(state.editor_ui.agent_settings.mcp_server.port, 5150);
}

#[test]
fn rebuilt_baselines_keep_saving_and_keep_failing_closed() {
    // The regression this pins: setting `settings_fingerprint` to `None`
    // makes the save gate (`if let Some(..)`) skip forever, so nothing was
    // ever persisted again after an account switch.
    let mut state = EditorState::new();
    let healthy = super::storage::load_into_with(&mut state, None, None, |_, _| true);
    assert!(
        healthy.initial_settings_fingerprint(&state).is_some(),
        "a healthy partition must leave the save path enabled"
    );
    assert!(!healthy
        .initial_fingerprint(&state)
        .write_disabled_for_test());

    // …and an unsupported snapshot must still fail closed rather than being
    // "reset" into a writable baseline.
    let unsupported = r#"{"version":9999}"#;
    let mut state = EditorState::new();
    let blocked = super::storage::load_into_with(&mut state, Some(unsupported), None, |_, _| true);
    assert!(
        blocked.initial_settings_fingerprint(&state).is_none(),
        "an unsupported snapshot must keep settings writes disabled"
    );
    assert!(blocked
        .initial_fingerprint(&state)
        .write_disabled_for_test());
}
