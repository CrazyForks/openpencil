use op_editor_core::EditorState;
use op_orchestrator::{AppendContext, DesignRequest};

/// Resolve the selected chat model's capability id for the orchestrator.
/// Built-in (API-key) agents expose their concrete model id. ACP entries
/// preserve their `acp:<id>` catalog identity so the model-profile resolver
/// can choose its conservative weak-agent default instead of treating a
/// missing id as Full tier. The ACP marker is not a transport model override:
/// `selected_cli_model_id` still yields `None` for ACP providers.
///
/// Fixed CLI agents keep choosing their own model internally and yield `None`
/// here (the CLI-side selection rides `ChatProviderLlmClient::with_model`
/// instead). The returned id feeds model-aware orchestrator policy —
/// tier-gated skill filtering, the element-manifest routing gate, and the M3
/// thinking policy — and matches the configuration the ab-v9 benchmarks ran
/// with (op-smoke has always passed `OPENPENCIL_ORCHESTRATOR_MODEL` through).
fn selected_orchestrator_model(state: &EditorState) -> Option<String> {
    let entry = state.chat.selected_model_entry()?;
    if entry.acp_agent_id().is_some() {
        return Some(entry.value.clone());
    }
    let id = entry.builtin_provider_id.as_deref()?;
    state
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .find(|agent| agent.id == id)
        .map(|agent| agent.model.trim().to_string())
        .filter(|model| !model.is_empty())
}

pub(crate) fn build_design_request(
    prompt: String,
    state: &EditorState,
    append_context: Option<AppendContext>,
) -> DesignRequest {
    DesignRequest {
        prompt,
        model: selected_orchestrator_model(state),
        provider: None,
        design_md: state.doc.design_md.clone(),
        // Detected by `chat_intent::detect_append_intent` when the
        // prompt asks to extend the existing page (GAP #33). TS wires
        // this from the agent tool executor (agent-tool-executor.ts:234);
        // the shell's design pipeline is that path's equivalent.
        append_context,
        concurrency: state.chat.agent_team_size,
        validation_enabled: true,
        visual_ref_enabled: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::{
        AgentProvider, BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey, EditorState,
        ModelEntry,
    };

    #[test]
    fn built_in_design_requests_enable_validation() {
        let mut state = EditorState::new();
        state.chat.agent_team_size = 4;

        let req = build_design_request("draw a mobile settings screen".into(), &state, None);

        assert!(req.validation_enabled);
        assert!(!req.visual_ref_enabled);
        assert_eq!(req.concurrency, 4);
        // No selected model entry → no model id (CLI agents pick their own).
        assert_eq!(req.model, None);
        assert!(req.append_context.is_none());
    }

    #[test]
    fn append_context_rides_the_request() {
        let state = EditorState::new();
        let ctx = AppendContext {
            target_parent_id: "content-root".into(),
            target_width: 390.0,
            existing_section_labels: vec!["Hero".into()],
            is_mobile: true,
        };

        let req = build_design_request("continue the page".into(), &state, Some(ctx));

        let ctx = req.append_context.expect("append context attached");
        assert_eq!(ctx.target_parent_id, "content-root");
        assert_eq!(ctx.existing_section_labels, vec!["Hero".to_string()]);
    }

    #[test]
    fn listed_follow_on_screens_keep_x2_request_out_of_append_mode() {
        let mut state = EditorState::new();
        state.active_children_mut().clear();
        for (id, name) in [("home", "Home"), ("trips", "Trips"), ("saved", "Saved")] {
            state.active_children_mut().push(
                serde_json::from_value(serde_json::json!({
                    "type": "frame",
                    "id": id,
                    "name": name,
                    "width": 375,
                    "height": 812,
                    "children": [{
                        "type": "frame",
                        "id": format!("{id}-content"),
                        "name": "Content",
                        "width": 375,
                        "height": 700,
                        "children": []
                    }]
                }))
                .expect("screen fixture"),
            );
        }
        state.chat.agent_team_size = 2;
        for prompt in [
            "继续完成 explore/profile界面",
            "Continue generating the explore/profile interface",
        ] {
            let append_context =
                op_host_services::chat_intent::detect_append_intent(&state, prompt);
            let req = build_design_request(prompt.into(), &state, append_context);

            assert!(
                req.append_context.is_none(),
                "the continuation must create sibling roots instead of targeting Home: {prompt}"
            );
            assert_eq!(req.concurrency, 2, "the x2 setting must survive routing");
            assert!(
                op_host_services::chat_intent::should_auto_generate_design_md(
                    &state,
                    prompt,
                    req.append_context.as_ref(),
                ),
                "follow-on screens must extract the existing canvas design system first: {prompt}"
            );
        }
    }

    #[test]
    fn selected_builtin_agent_model_reaches_the_orchestrator() {
        let mut state = EditorState::new();
        state
            .editor_ui
            .agent_settings
            .builtin_agents
            .push(BuiltinAgentConfig {
                id: "builtin-1".into(),
                preset: BuiltinAgentPresetKey::Custom,
                display_name: "MiniMax".into(),
                kind: BuiltinAgentKind::OpenAiCompat,
                api_key: "sk-test".into(),
                model: "MiniMax-M3".into(),
                base_url: "http://localhost:9".into(),
                enabled: true,
            });
        let mut entry = ModelEntry::new(AgentProvider::ClaudeCode, "MiniMax-M3", "MiniMax M3");
        entry.builtin_provider_id = Some("builtin-1".into());
        state.chat.available_models = vec![entry];
        state.chat.selected_model = 0;

        let req = build_design_request("draw a dashboard".into(), &state, None);

        // Drives tier-gated prompts, the manifest routing gate, and the
        // M3 thinking policy — must match the agent the session will call.
        assert_eq!(req.model.as_deref(), Some("MiniMax-M3"));
    }

    #[test]
    fn selected_acp_agent_reaches_the_orchestrator_as_basic_tier() {
        let mut state = EditorState::new();
        state.chat.available_models = vec![ModelEntry::acp("custom/vendor", "Custom ACP")];
        state.chat.selected_model = 0;

        let req = build_design_request("draw a dashboard".into(), &state, None);

        assert_eq!(req.model.as_deref(), Some("acp:custom/vendor"));
        assert_eq!(
            op_orchestrator::resolve_model_profile(req.model.as_deref().unwrap()).tier,
            op_orchestrator::ModelTier::Basic
        );
    }
}
