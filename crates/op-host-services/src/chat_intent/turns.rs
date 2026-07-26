//! Router worker: the pre-computed `CliTurnPlan` and the two turn runners
//! it dispatches to (`run_cli_turn` for the classify → chat/design route,
//! `run_modify_turn` for the selection-scoped edit route). Split out of
//! `chat_intent.rs` to keep the spine under the 800-line cap.

use super::*;

// ---------------------------------------------------------------------------
// Router worker
// ---------------------------------------------------------------------------

/// Everything the router worker needs, pre-computed on the UI thread.
pub struct CliTurnPlan {
    pub user_text: String,
    /// TS `pageChildren.length === 0` (modify degrades to new).
    pub page_children_empty: bool,
    /// Session-untracked transport for the classification call.
    pub classify_provider: Box<dyn ChatProvider>,
    /// Chat-session-tracked transport for the plain chat route.
    pub chat_provider: Box<dyn ChatProvider>,
    /// Session-untracked transport for the design / modify routes.
    pub design_provider: Box<dyn ChatProvider>,
    /// Fully-assembled plain-chat request (system prompt + history +
    /// per-turn knobs + attachments).
    pub chat_request: ChatRequest,
    /// `generateDesignModification` request; `None` when the page had
    /// no modification target (route degrades to new).
    pub modify_request: Option<ChatRequest>,
    /// Orchestrator request for the new-design route (append context
    /// already detected + attached).
    pub design_request: DesignRequest,
    /// State snapshot for the design route's `RemoteDocSink` mirror.
    pub initial_state: EditorState,
    /// Host-owned indicator epoch shared with the parked `DesignSession`.
    /// The worker registers frame/node indicators under this value and
    /// `DesignSession::drop` clears that same run when Done / Stop /
    /// New Chat retires the session.
    pub indicator_epoch: u64,
    /// Shared with the parked `DesignSession` so Stop / New Chat can cancel
    /// the design route even while its concurrent screen workers are active.
    pub abort: AbortFlag,
    pub model: Option<String>,
}

/// TS `ai-chat-handlers.ts:721-722` — the modification progress step.
pub(super) const MODIFY_STEP: &str =
    r#"<step title="Checking guidelines">Analyzing modification request...</step>"#;
pub(super) const MODIFY_RETRY_REMINDER: &str =
    "\n\nCRITICAL: Respond with ONLY I(...) JavaScript statements -- never prose, explanations, or numbered/bulleted lists. If you truly cannot make the change, return an empty program.";

pub(super) struct ModifyTurnParse {
    full_response: String,
    nodes: Vec<crate::chat_canvas_tools::DesignModificationOp>,
    stream_error: Option<String>,
}

pub(super) fn applied_modify_nodes_json(
    nodes: &[crate::chat_canvas_tools::DesignModificationOp],
) -> Option<String> {
    let node_values = nodes
        .iter()
        .map(|(_, node)| node.clone())
        .collect::<Vec<_>>();
    if node_values.is_empty() {
        return None;
    }
    serde_json::to_string_pretty(&node_values).ok()
}

pub(super) fn stream_and_parse_modify_turn(
    provider: &dyn ChatProvider,
    request: ChatRequest,
) -> ModifyTurnParse {
    let mut full_response = String::new();
    let mut stream_error: Option<String> = None;
    for delta in provider.send(request) {
        match delta {
            ChatDelta::TextDelta(s) => full_response.push_str(&s),
            // TS: thinking chunks are ignored for modification — the
            // caller already shows progress.
            ChatDelta::Thinking(_) | ChatDelta::ToolUse { .. } => {}
            ChatDelta::Error(msg) => {
                stream_error = Some(msg);
                break;
            }
            ChatDelta::Done { .. } => break,
        }
    }

    // TS order: parse first; a stream error only surfaces when no
    // nodes could be extracted (design-generator.ts:158-165).
    let nodes = parse_modify_nodes(&full_response);
    ModifyTurnParse {
        full_response,
        nodes,
        stream_error,
    }
}

pub(super) fn with_modify_retry_reminder(mut request: ChatRequest) -> ChatRequest {
    request.system_prompt.push_str(MODIFY_RETRY_REMINDER);
    request
}

/// The TS degrade rules (`ai-chat-handlers.ts:700-705`): a modify
/// intent on an empty page becomes a new design, and `isModification`
/// additionally requires a usable target (here: a built modify plan —
/// after the empty-page degrade a surviving Modify always has one in
/// practice, so the plan check is a belt-and-braces guard).
pub(super) fn resolve_route(
    classified: DesignIntent,
    page_children_empty: bool,
    has_modify_plan: bool,
) -> DesignIntent {
    match classified {
        DesignIntent::Modify if page_children_empty => DesignIntent::New,
        DesignIntent::Modify if !has_modify_plan => DesignIntent::New,
        other => other,
    }
}

/// Run one CLI standard-mode turn end-to-end on the worker thread:
/// classify, then route to chat / modification / design. The caller
/// parked a `ChatSession` on `chat_tx` + the tool channel behind
/// `executor`, and a `DesignSession` on `delta_tx` / `cmd_tx`; the
/// routes not taken drop their senders so the matching pump retires
/// its session.
pub fn run_cli_turn(
    plan: CliTurnPlan,
    chat_tx: Sender<ChatDelta>,
    executor: UiChatToolExecutor,
    delta_tx: Sender<DesignDelta>,
    cmd_tx: Sender<DesignCmdReq>,
) {
    let modify_target_frame_ids = selected_frame_ids(&plan.initial_state).unwrap_or_default();
    let classified = classify_intent_for_standard_route(
        plan.classify_provider.as_ref(),
        &plan.initial_state,
        &plan.user_text,
        plan.model.clone(),
    );
    // Stop / New Chat may arrive while the classifier LLM is in flight. The
    // provider call itself has its own bounded timeout, but once it returns we
    // must not launch a fresh chat, modify, or concurrent design run for the
    // discarded session.
    if plan.abort.is_set() {
        return;
    }
    let modify_request = plan.modify_request;
    let intent = resolve_route(
        classified,
        plan.page_children_empty,
        modify_request.is_some(),
    );

    match intent {
        DesignIntent::Chat => {
            drop(delta_tx);
            drop(cmd_tx);
            drop(executor);
            for delta in plan.chat_provider.send(plan.chat_request) {
                if chat_tx.send(delta).is_err() {
                    return; // turn aborted (Stop / New Chat)
                }
            }
        }
        DesignIntent::Modify => {
            drop(delta_tx);
            drop(cmd_tx);
            let request = modify_request.expect("checked above");
            run_modify_turn(
                plan.design_provider.as_ref(),
                request,
                &chat_tx,
                &executor,
                modify_target_frame_ids,
            );
        }
        DesignIntent::New => {
            // Hold the chat channel open for the duration so the
            // trailing bubble keeps its streaming state while the
            // design pumps fill it.
            let _chat_hold = chat_tx;
            drop(executor);
            // Share one provider Arc between the design LLM and the
            // (flag-gated) vision validator so the real Class-C loop reuses
            // the user's selected auth/model.
            let provider_arc: Arc<dyn op_ai::chat_provider::ChatProvider> =
                Arc::from(plan.design_provider);
            let llm = ChatProviderLlmClient::new(provider_arc.clone()).with_model(plan.model);
            run_design_worker(
                llm,
                plan.design_request,
                plan.initial_state,
                delta_tx,
                cmd_tx,
                plan.indicator_epoch,
                plan.abort,
                Some(provider_arc),
            );
        }
    }
}

/// The DESIGN_MODIFY route — port of `generateDesignModification` +
/// `extractAndApplyDesignModification` + the handler glue
/// (`ai-chat-handlers.ts:708-741`, `design-generator.ts:95-173`).
pub fn run_modify_turn(
    provider: &dyn ChatProvider,
    request: ChatRequest,
    chat_tx: &Sender<ChatDelta>,
    executor: &UiChatToolExecutor,
    target_frame_ids: Vec<String>,
) {
    if chat_tx
        .send(ChatDelta::TextDelta(MODIFY_STEP.to_string()))
        .is_err()
    {
        return;
    }

    let mut parsed = stream_and_parse_modify_turn(provider, request.clone());
    if parsed.nodes.is_empty() {
        parsed = stream_and_parse_modify_turn(provider, with_modify_retry_reminder(request));
    }

    let ModifyTurnParse {
        full_response,
        nodes,
        stream_error,
    } = parsed;
    if !nodes.is_empty() {
        let args = serde_json::json!({
            "nodes": &nodes,
            "targetFrameIds": target_frame_ids,
        });
        let result = executor.execute(APPLY_MODIFICATION_OP, &args.to_string());
        let applied = serde_json::from_str::<serde_json::Value>(&result.content)
            .ok()
            .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
            .unwrap_or(0);
        if applied > 0 {
            if let Some(json) = applied_modify_nodes_json(&nodes) {
                if chat_tx
                    .send(ChatDelta::TextDelta(format!("\n```json\n{json}\n```")))
                    .is_err()
                {
                    return;
                }
            }
            // TS `ai-chat-handlers.ts:830-831`.
            let _ = chat_tx.send(ChatDelta::TextDelta("\n\n<!-- APPLIED -->".to_string()));
        } else if chat_tx
            .send(ChatDelta::TextDelta(format!("\n{full_response}")))
            .is_err()
        {
            return;
        }
        let _ = chat_tx.send(ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        });
        return;
    }

    let message = if let Some(err) = stream_error {
        err
    } else {
        "The model returned a description instead of an applyable edit. Try rephrasing (e.g. name the element to add) or run it again."
            .to_string()
    };
    let _ = chat_tx.send(ChatDelta::Error(message));
    let _ = chat_tx.send(ChatDelta::Done {
        stop_reason: StopReason::Aborted,
    });
}
