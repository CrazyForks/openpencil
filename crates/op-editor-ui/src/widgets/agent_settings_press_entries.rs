//! Built-in-agent / ACP-agent / image-gen-profile press arms of the
//! agent-settings modal — the list-entry half of
//! [`super::agent_settings_press_flow::apply_agent_settings_hit`].
//!
//! Split out of `agent_settings_press_flow.rs` to keep every file under
//! the repo's 800-line cap.

use op_editor_core::agent_settings::{
    AcpAgentField, BuiltinAgentField, BuiltinAgentPresetMenuTarget, ImageGenField, ImageTestStatus,
    SettingsFocus,
};
use op_editor_core::host_settings_commit::{commit_settings_focus, SettingsCommitScope};
use op_editor_core::EditorState;

use crate::widgets::agent_settings_panel::AgentSettingsHit;
use crate::widgets::agent_settings_press_flow::SettingsPressOutcome;
use crate::widgets::agent_settings_press_focus::*;

/// Built-in-agent / ACP-agent / image-gen-profile arms — the list-entry
/// half of the match, split out to keep both functions readable.
pub(crate) fn apply_entry_hit(
    state: &mut EditorState,
    hit: AgentSettingsHit,
    scope: SettingsCommitScope,
    now_ms: u64,
) -> SettingsPressOutcome {
    let commit = |state: &mut EditorState| {
        commit_settings_focus(state, scope);
    };
    let take_over = matches!(scope, SettingsCommitScope::Operator);
    match hit {
        // ─── Image-generation profiles ─────────────────────────────
        AgentSettingsHit::SetActiveGenConfig(index) => {
            commit(state);
            if let Some(id) = image_gen_profile_id(state, index) {
                state
                    .editor_ui
                    .agent_settings
                    .set_active_image_gen_profile(&id);
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::RemoveGenConfig(index) => {
            commit(state);
            if let Some(id) = image_gen_profile_id(state, index) {
                state.editor_ui.agent_settings.remove_image_gen_profile(&id);
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::TestGenConfig(index) => {
            commit(state);
            if let Some(profile) = state
                .editor_ui
                .agent_settings
                .image_gen_profiles
                .get_mut(index)
            {
                profile.test_status = if profile.api_key.trim().is_empty() {
                    ImageTestStatus::Invalid
                } else {
                    ImageTestStatus::Testing
                };
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::ToggleGenConfigEditor(index) => {
            let was_editing = matches!(
                state.editor_ui.agent_settings.focus,
                Some(SettingsFocus::ImageGenProfile { index: focused, .. }) if focused == index
            );
            commit(state);
            if !was_editing {
                focus_image_gen_profile(state, index, ImageGenField::Name, now_ms);
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::AddGenConfig => {
            commit(state);
            let id = state.editor_ui.agent_settings.add_image_gen_profile();
            let index = state
                .editor_ui
                .agent_settings
                .image_gen_profiles
                .iter()
                .position(|profile| profile.id == id)
                .unwrap_or(0);
            focus_image_gen_profile(state, index, ImageGenField::Name, now_ms);
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::ToggleGenProviderMenu(index) => {
            commit(state);
            let settings = &mut state.editor_ui.agent_settings;
            settings.image_gen_provider_menu_open =
                (settings.image_gen_provider_menu_open != Some(index)).then_some(index);
            focus_image_gen_profile(state, index, ImageGenField::Name, now_ms);
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::SelectGenProvider { index, provider: _ } => {
            commit(state);
            focus_image_gen_profile(state, index, ImageGenField::Name, now_ms);
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::FocusGenConfig { index, field } => {
            commit(state);
            state.editor_ui.agent_settings.image_gen_provider_menu_open = None;
            focus_image_gen_profile(state, index, field, now_ms);
            SettingsPressOutcome::handled()
        }
        // ─── Built-in (API-key) agents ─────────────────────────────
        AgentSettingsHit::FocusBuiltinAgent { index, field } => {
            commit(state);
            focus_builtin_agent(state, index, field, now_ms);
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::FocusBuiltinAgentDraft(field) => {
            commit(state);
            focus_builtin_agent_draft(state, field, now_ms);
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::ToggleBuiltinAgentKind(index) => {
            commit(state);
            if take_over {
                state
                    .editor_ui
                    .agent_settings
                    .take_over_browser_builtin_agent(index);
            }
            if let Some(agent) = state.editor_ui.agent_settings.builtin_agents.get_mut(index) {
                agent.toggle_kind_for_preset();
                state.rebuild_chat_models();
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::ToggleBuiltinAgentDraftKind => {
            commit(state);
            if let Some(agent) = state.editor_ui.agent_settings.builtin_agent_draft.as_mut() {
                agent.toggle_kind_for_preset();
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::ToggleBuiltinAgentPresetMenu(index) => {
            commit(state);
            let target = match index {
                Some(index) => BuiltinAgentPresetMenuTarget::Agent(index),
                None => BuiltinAgentPresetMenuTarget::Draft,
            };
            let settings = &mut state.editor_ui.agent_settings;
            settings.builtin_preset_menu_open =
                (settings.builtin_preset_menu_open != Some(target)).then_some(target);
            settings.builtin_preset_menu_scroll.offset = 0.0;
            settings.builtin_preset_menu_hover = None;
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::SelectBuiltinAgentPreset { index, preset } => {
            commit(state);
            match index {
                Some(index) => {
                    if take_over {
                        state
                            .editor_ui
                            .agent_settings
                            .take_over_browser_builtin_agent(index);
                    }
                    state
                        .editor_ui
                        .agent_settings
                        .set_builtin_agent_preset(index, preset);
                    state.rebuild_chat_models();
                }
                None => state
                    .editor_ui
                    .agent_settings
                    .set_builtin_agent_draft_preset(preset),
            }
            let settings = &mut state.editor_ui.agent_settings;
            settings.builtin_preset_menu_open = None;
            settings.builtin_preset_menu_scroll.offset = 0.0;
            settings.builtin_preset_menu_hover = None;
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::ToggleBuiltinAgentEnabled(index) => {
            commit(state);
            if take_over {
                state
                    .editor_ui
                    .agent_settings
                    .take_over_browser_builtin_agent(index);
            }
            if let Some(agent) = state.editor_ui.agent_settings.builtin_agents.get_mut(index) {
                agent.enabled = !agent.enabled;
                state.rebuild_chat_models();
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::EditBuiltinAgent(index) => {
            commit(state);
            focus_builtin_agent(state, index, BuiltinAgentField::DisplayName, now_ms);
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::RemoveBuiltinAgent(index) => {
            commit(state);
            let agents = &mut state.editor_ui.agent_settings.builtin_agents;
            if index < agents.len() {
                agents.remove(index);
                clear_focus(state);
                state.rebuild_chat_models();
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::AddProvider => {
            commit(state);
            state.editor_ui.agent_settings.begin_builtin_agent_draft();
            focus_builtin_agent_draft(state, BuiltinAgentField::ApiKey, now_ms);
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::SaveBuiltinAgentDraft => {
            commit(state);
            if state
                .editor_ui
                .agent_settings
                .save_builtin_agent_draft()
                .is_some()
            {
                clear_focus(state);
                state.rebuild_chat_models();
            } else {
                // Rejected draft keeps the form open, parked on the field
                // that must be filled in.
                focus_builtin_agent_draft(state, BuiltinAgentField::ApiKey, now_ms);
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::CancelBuiltinAgentDraft => {
            state.editor_ui.agent_settings.cancel_builtin_agent_draft();
            clear_focus(state);
            SettingsPressOutcome::handled()
        }
        // ─── ACP agents ────────────────────────────────────────────
        AgentSettingsHit::FocusAcpAgent { index, field } => {
            commit(state);
            focus_acp_agent(state, index, field, now_ms);
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::FocusAcpAgentDraft(field) => {
            commit(state);
            focus_acp_agent_draft(state, field, now_ms);
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::EditAcpAgent(index) => {
            commit(state);
            focus_acp_agent(state, index, AcpAgentField::DisplayName, now_ms);
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::RemoveAcpAgent(index) => {
            commit(state);
            let id = state
                .editor_ui
                .agent_settings
                .acp_agents
                .get(index)
                .map(|agent| agent.id.clone());
            if let Some(id) = id {
                state.editor_ui.agent_settings.remove_acp_agent(&id);
                clear_focus(state);
                state.rebuild_chat_models();
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::ToggleAcpConnected(index) => {
            commit(state);
            let settings = &state.editor_ui.agent_settings;
            // A card that was never configured focuses its transport
            // field instead of starting a doomed probe.
            let needs_config_focus = settings.acp_agents.get(index).is_some_and(|agent| {
                !settings.acp_agent_verified_connected(&agent.id) && !agent.ready()
            });
            if needs_config_focus {
                let field = state
                    .editor_ui
                    .agent_settings
                    .acp_agents
                    .get(index)
                    .map(|agent| transport_field(agent.connection_type));
                if let Some(field) = field {
                    focus_acp_agent(state, index, field, now_ms);
                }
            } else if state
                .editor_ui
                .agent_settings
                .acp_agent_verified_connected_at(index)
            {
                state.editor_ui.agent_settings.disconnect_acp_agent(index);
                state.rebuild_chat_models();
            } else if state
                .editor_ui
                .agent_settings
                .begin_acp_agent_connect(index)
                .is_some()
            {
                state.rebuild_chat_models();
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::AddAcpAgent => {
            commit(state);
            state.editor_ui.agent_settings.begin_acp_agent_draft();
            focus_acp_agent_draft(state, AcpAgentField::Command, now_ms);
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::SaveAcpAgentDraft => {
            commit(state);
            if state
                .editor_ui
                .agent_settings
                .save_acp_agent_draft()
                .is_some()
            {
                clear_focus(state);
            } else {
                let field = state
                    .editor_ui
                    .agent_settings
                    .acp_agent_draft
                    .as_ref()
                    .map(|agent| transport_field(agent.connection_type))
                    .unwrap_or(AcpAgentField::Command);
                focus_acp_agent_draft(state, field, now_ms);
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::CancelAcpAgentDraft => {
            state.editor_ui.agent_settings.cancel_acp_agent_draft();
            clear_focus(state);
            SettingsPressOutcome::handled()
        }
        // Every remaining variant is handled by the caller. Spelled out
        // rather than `_` so a new hit variant is a compile error in one
        // of the two halves instead of a silent no-op.
        AgentSettingsHit::Close
        | AgentSettingsHit::Outside
        | AgentSettingsHit::Inside
        | AgentSettingsHit::SelectTab(_)
        | AgentSettingsHit::Connect(_)
        | AgentSettingsHit::ToggleMcpServer
        | AgentSettingsHit::ToggleMcpCli(_)
        | AgentSettingsHit::CopyMcpClientConfig
        | AgentSettingsHit::Fonts(_)
        | AgentSettingsHit::ToggleImagesAdvanced
        | AgentSettingsHit::FocusSearchField(_)
        | AgentSettingsHit::OpenImageRegisterLink
        | AgentSettingsHit::TestImageSearch
        | AgentSettingsHit::ToggleAutoUpdate
        | AgentSettingsHit::SelectPencilCursor(_)
        | AgentSettingsHit::ToggleExperimental
        | AgentSettingsHit::OpenLoginModal
        | AgentSettingsHit::SignOutAccount
        | AgentSettingsHit::FocusMcpPort => {
            debug_assert!(false, "handled by apply_agent_settings_hit");
            SettingsPressOutcome::handled()
        }
    }
}
