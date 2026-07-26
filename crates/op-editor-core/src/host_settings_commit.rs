//! Settings-modal draft commit shared by the native and web widget
//! hosts.
//!
//! Both twins carried this ~190-line `SettingsFocus` walk
//! (`widget_host/settings_dispatch.rs::commit_settings_focus_if_any` on
//! native, `widget_host/keyboard_settings_commit.rs::
//! commit_settings_focus` on the web) as near-identical copies. The only
//! genuine difference is credential OWNERSHIP — see
//! [`SettingsCommitScope`] — so that is the one parameter; everything
//! else is one body.

use crate::agent_settings::{
    AcpAgentField, BuiltinAgentField, ImageGenField, ImageSearchField, SettingsFocus,
};
use crate::editor_ui_state::EditorUiState;
use crate::state::EditorState;

/// Who owns the credential entry a settings draft is about to write to.
///
/// A browser-pushed credential snapshot identifies itself through a
/// scoped id (`web-credential:builtin:…` / `web-credential:image:…`) and
/// through `openverse_credential_owner`. When the DESKTOP operator edits
/// such an entry, the edit is an ownership transfer: the entry is re-idded
/// as local (`AgentSettings::take_over_browser_*`) and the Openverse
/// owner tag is dropped, so the daemon's `web_credentials` merge stops
/// treating it as browser-managed. The browser must NOT do any of that —
/// there it is already the owner, and re-idding its own snapshot would
/// orphan the entry on the next sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCommitScope {
    /// Desktop / daemon operator — commits transfer ownership.
    Operator,
    /// In-browser host — commits leave the credential scoping alone.
    Browser,
}

impl SettingsCommitScope {
    fn takes_over_browser_entries(self) -> bool {
        matches!(self, Self::Operator)
    }
}

/// Commit the focused settings-modal input and drop the focus + caret.
///
/// Returns `true` when a focus was actually taken (so the host marks
/// dirty); `false` is the no-focus fast path both twins used to spell as
/// an early `return`.
pub fn commit_settings_focus(state: &mut EditorState, scope: SettingsCommitScope) -> bool {
    let Some(focus) = state.editor_ui.agent_settings.focus.take() else {
        return false;
    };
    let draft = state.editor_ui.settings_input.text().to_owned();
    state.editor_ui.settings_input.set_text("");
    let trimmed = draft.trim();
    match focus {
        SettingsFocus::McpPort => {
            if let Ok(port) = trimmed.parse::<u16>() {
                state.editor_ui.agent_settings.mcp_server.port = port.max(1024);
            }
        }
        SettingsFocus::ImageSearch(field) => {
            commit_image_search(&mut state.editor_ui, scope, field, trimmed);
        }
        SettingsFocus::BuiltinAgent { index, field } => {
            let settings = &mut state.editor_ui.agent_settings;
            if scope.takes_over_browser_entries() {
                settings.take_over_browser_builtin_agent(index);
            }
            if let Some(agent) = settings.builtin_agents.get_mut(index) {
                write_builtin_field(agent, field, trimmed);
                // The chat picker lists models per ready agent — a
                // re-keyed / re-modelled agent changes that set.
                state.rebuild_chat_models();
            }
        }
        SettingsFocus::BuiltinAgentDraft(field) => {
            if let Some(agent) = state.editor_ui.agent_settings.builtin_agent_draft.as_mut() {
                write_builtin_field(agent, field, trimmed);
            }
        }
        SettingsFocus::ImageGenProfile { index, field } => {
            let settings = &mut state.editor_ui.agent_settings;
            if scope.takes_over_browser_entries() {
                settings.take_over_browser_image_profile(index);
            }
            if let Some(profile) = settings.image_gen_profiles.get_mut(index) {
                match field {
                    ImageGenField::Name => profile.name = trimmed.to_string(),
                    ImageGenField::ApiKey => profile.api_key = trimmed.to_string(),
                    ImageGenField::Model => profile.model = trimmed.to_string(),
                    ImageGenField::BaseUrl => {
                        profile.base_url = (!trimmed.is_empty()).then(|| trimmed.to_string());
                    }
                }
            }
        }
        SettingsFocus::AcpAgent { index, field } => {
            if let Some(agent) = state.editor_ui.agent_settings.acp_agents.get_mut(index) {
                write_acp_field(agent, field, &draft);
                state.rebuild_chat_models();
            }
        }
        SettingsFocus::AcpAgentDraft(field) => {
            if let Some(agent) = state.editor_ui.agent_settings.acp_agent_draft.as_mut() {
                write_acp_field(agent, field, &draft);
            }
        }
    }
    true
}

fn commit_image_search(
    ui: &mut EditorUiState,
    scope: SettingsCommitScope,
    field: ImageSearchField,
    trimmed: &str,
) {
    match field {
        ImageSearchField::ClientId => {
            ui.agent_settings.openverse_client_id = trimmed.to_string();
        }
        ImageSearchField::ClientSecret => {
            ui.agent_settings.openverse_client_secret = trimmed.to_string();
        }
    }
    if scope.takes_over_browser_entries() {
        ui.agent_settings.openverse_credential_owner = None;
    }
}

/// Write one built-in-agent field from a committed draft.
///
/// `DisplayName` keeps the previous value on an empty draft (a nameless
/// card would be unclickable); `BaseUrl` falls back to the preset's
/// default and is refused outright on presets that pin it. The widget
/// hit-test already skips the BaseUrl row for those presets
/// (`agent_settings_builtin.rs`), so the guard is belt-and-braces — but
/// only the native twin carried it, and defensive is the right side to
/// unify on.
fn write_builtin_field(
    agent: &mut crate::agent_settings::BuiltinAgentConfig,
    field: BuiltinAgentField,
    trimmed: &str,
) {
    match field {
        BuiltinAgentField::DisplayName => {
            if !trimmed.is_empty() {
                agent.display_name = trimmed.to_string();
            }
        }
        BuiltinAgentField::ApiKey => agent.api_key = trimmed.to_string(),
        BuiltinAgentField::Model => agent.model = trimmed.to_string(),
        BuiltinAgentField::BaseUrl => {
            if agent.base_url_editable() {
                agent.base_url = if trimmed.is_empty() {
                    agent.kind.default_base_url().to_string()
                } else {
                    trimmed.to_string()
                };
            }
        }
    }
}

/// Write one ACP-agent field from a committed draft. Every transport
/// field drops `connected` — the live session no longer matches the
/// config it was started from. `Args` / `Env` parse the RAW draft (their
/// own splitters handle whitespace), everything else the trimmed form.
fn write_acp_field(
    agent: &mut crate::agent_settings::AcpAgentConfig,
    field: AcpAgentField,
    draft: &str,
) {
    let trimmed = draft.trim();
    match field {
        AcpAgentField::DisplayName => {
            if !trimmed.is_empty() {
                agent.display_name = trimmed.to_string();
            }
        }
        AcpAgentField::Command => {
            agent.command = trimmed.to_string();
            agent.connected = false;
        }
        AcpAgentField::Args => {
            agent.set_args_text(draft);
            agent.connected = false;
        }
        AcpAgentField::Env => {
            agent.set_env_text(draft);
            agent.connected = false;
        }
        AcpAgentField::Url => {
            agent.url = (!trimmed.is_empty()).then(|| trimmed.to_string());
            agent.connected = false;
        }
    }
}
