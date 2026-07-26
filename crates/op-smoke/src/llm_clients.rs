//! LLM clients for the headless smoke runner.
//!
//! Two `op_orchestrator::LlmClient` impls carved off `main.rs` to keep both
//! files under the 800-line cap:
//!
//! - [`SmokeLlmClient`] — the default path, `agent`'s `QueryEngine` over an
//!   `AnthropicProvider` / `OpenAiCompatProvider`.
//! - [`DirectOpenAiClient`] — `OPENPENCIL_SMOKE_DIRECT=1`, a plain
//!   non-streaming openai-compat POST that can send MiniMax's
//!   `thinking:{type:disabled}` body field the QueryEngine cannot.

use std::sync::Arc;

use agent::abort::AbortController;
use agent::provider::Provider;
use agent::query::QueryEngine;
use agent::stream::Event;
use futures::channel::mpsc;
use futures::StreamExt;
use op_orchestrator::{CallRequest, LlmChunk, LlmClient, LlmError};

/// `LlmClient` impl for the smoke runner — `AnthropicProvider` under a
/// `QueryEngine`, with every call spawned onto the current tokio runtime.
/// Standalone — `op-host-desktop` no longer ships a desktop
/// `LlmClient`; its production path goes through
/// `chat_provider_llm::ChatProviderLlmClient` (wrapping the user's
/// selected chat CLI). The smoke needs to talk to a raw API endpoint
/// to validate orchestrator behaviour independently of any CLI, hence
/// this dedicated client.
pub(crate) struct SmokeLlmClient {
    pub(crate) provider: Arc<dyn Provider>,
    pub(crate) default_model: String,
}

impl LlmClient for SmokeLlmClient {
    fn call(
        &self,
        req: CallRequest,
    ) -> futures::stream::BoxStream<'static, Result<LlmChunk, LlmError>> {
        let (tx, rx) = mpsc::unbounded::<Result<LlmChunk, LlmError>>();
        if req.abort.is_set() {
            let _ = tx.unbounded_send(Err(LlmError {
                message: "aborted".into(),
                aborted: true,
            }));
            return Box::pin(rx);
        }
        let provider = self.provider.clone();
        let model = req
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());
        let system = req.system_prompt.clone();
        let user = req.user_prompt.clone();

        eprintln!(
            "[LLM] call: model={model} system_len={} user_len={}",
            system.len(),
            user.len()
        );

        tokio::spawn(async move {
            // QueryEngine 默认 4096 输出 token,对推理模型(MiniMax-M3 等)远不够
            // ——它先吐 <think>(常 ~3.5k token)再给 JSON,4096 会在答案前截断。
            // 生产路径(chat_provider_llm)用 8192 且关思考;benchmark 走 QueryEngine
            // 无法关思考,故给更宽预算让其 think 完还能产出 JSON。可用
            // OPENPENCIL_SMOKE_MAX_TOKENS 覆盖。
            let max_tokens = std::env::var("OPENPENCIL_SMOKE_MAX_TOKENS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(16384);
            let engine = QueryEngine::new(provider, model)
                .with_system(system)
                .with_max_output_tokens(max_tokens);
            let abort = AbortController::new();
            let stream = match engine.run(user, abort).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[LLM] engine.run error: {e}");
                    let _ = tx.unbounded_send(Err(LlmError {
                        message: e.to_string(),
                        aborted: false,
                    }));
                    return;
                }
            };
            let mut stream = stream;
            // Optional raw-response capture for diagnosing weak-model
            // JSON malformations (set OPENPENCIL_SMOKE_DUMP=1).
            let dump = std::env::var("OPENPENCIL_SMOKE_DUMP").is_ok();
            let mut full = String::new();
            while let Some(item) = stream.next().await {
                let sent = match item {
                    Ok(Event::TextDelta { delta }) => {
                        if dump {
                            full.push_str(&delta);
                        }
                        tx.unbounded_send(Ok(LlmChunk::Text(delta)))
                    }
                    Ok(Event::Thinking { delta }) => {
                        tx.unbounded_send(Ok(LlmChunk::Thinking(delta)))
                    }
                    Ok(Event::Result { .. }) => break,
                    Ok(Event::Error { code, message }) => {
                        eprintln!("[LLM] event error: {code}: {message}");
                        tx.unbounded_send(Err(LlmError {
                            message: format!("{code}: {message}"),
                            aborted: false,
                        }))
                    }
                    Ok(_) => Ok(()),
                    Err(e) => {
                        eprintln!("[LLM] stream error: {e}");
                        tx.unbounded_send(Err(LlmError {
                            message: e.to_string(),
                            aborted: false,
                        }))
                    }
                };
                if sent.is_err() {
                    break;
                }
            }
            if dump && !full.is_empty() {
                eprintln!(
                    "[DUMP] ===== LLM response ({} chars) =====\n{full}\n[DUMP] ===== end =====",
                    full.len()
                );
            }
        });

        Box::pin(rx)
    }
}

/// MiniMax M-series ("MiniMax-M*", legacy "abab*") are reasoning models whose
/// thinking is toggled by the MiniMax `thinking` body field. Mirrors the
/// production gate in `chat_builtin_http::is_minimax_model`.
fn is_minimax_model(model: &str) -> bool {
    let m = model.to_ascii_lowercase();
    m.starts_with("minimax") || m.starts_with("abab")
}

/// GLM reasoning models burn the whole `max_tokens` budget on reasoning and
/// return EMPTY content unless thinking is disabled (measured: an orchestrator
/// sidebar subtask failed 3× "empty content from provider" and shipped
/// missing). Mirrors the production gate in `chat_builtin_http::is_glm_model`.
fn is_glm_model(model: &str) -> bool {
    model.to_ascii_lowercase().starts_with("glm")
}

/// Direct openai-compat `LlmClient` for the harness (OPENPENCIL_SMOKE_DIRECT=1).
///
/// The default [`SmokeLlmClient`] goes through the vendored `agent` QueryEngine,
/// which can't send MiniMax's `thinking:{type:disabled}` field — so M3 (a
/// reasoning model) thinks itself out of budget. This client does a plain
/// non-streaming POST and adds that field for MiniMax models, mirroring the
/// production fix in `chat_builtin_http::run_openai_chat`, so M3-with-thinking-
/// disabled can be validated end-to-end headless (no GUI, no submodule edit).
pub(crate) struct DirectOpenAiClient {
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) default_model: String,
}

impl LlmClient for DirectOpenAiClient {
    fn call(
        &self,
        req: CallRequest,
    ) -> futures::stream::BoxStream<'static, Result<LlmChunk, LlmError>> {
        let (tx, rx) = mpsc::unbounded::<Result<LlmChunk, LlmError>>();
        if req.abort.is_set() {
            let _ = tx.unbounded_send(Err(LlmError {
                message: "aborted".into(),
                aborted: true,
            }));
            return Box::pin(rx);
        }
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let key = self.api_key.clone();
        let model = req
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());
        let system = req.system_prompt.clone();
        let user = req.user_prompt.clone();
        let max_tokens: u32 = std::env::var("OPENPENCIL_SMOKE_MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(16384);
        let dump = std::env::var("OPENPENCIL_SMOKE_DUMP").is_ok();
        eprintln!(
            "[LLM] direct call: model={model} system_len={} user_len={}",
            system.len(),
            user.len()
        );
        tokio::spawn(async move {
            let mut body = serde_json::json!({
                "model": model,
                "stream": false,
                "max_tokens": max_tokens,
                "messages": [
                    { "role": "system", "content": system },
                    { "role": "user", "content": user },
                ],
            });
            // MiniMax reasoning models inject `<think>` into content by default;
            // disable it at the wire level. `OPENPENCIL_SMOKE_DISABLE_THINKING=1`
            // forces it for any model whose endpoint speaks the same schema
            // (Volcengine 方舟 — glm/kimi/doubao — confirmed to honor it), so the
            // latest 方舟-hosted reasoning models can be benchmarked clean too.
            let force_disable = std::env::var("OPENPENCIL_SMOKE_DISABLE_THINKING").is_ok();
            // `OPENPENCIL_SMOKE_KEEP_THINKING=1` keeps reasoning ON even for
            // MiniMax — ab-v9 showed M3-nothink emits lazy minimal manifests
            // (17% M3, ~10s answers); this lets the M3-with-think arm be
            // benchmarked (strip_reasoning handles the <think> blocks).
            let keep_thinking = std::env::var("OPENPENCIL_SMOKE_KEEP_THINKING").is_ok();
            if !keep_thinking && (force_disable || is_minimax_model(&model) || is_glm_model(&model))
            {
                if let Some(obj) = body.as_object_mut() {
                    obj.insert("thinking".into(), serde_json::json!({ "type": "disabled" }));
                }
            }
            // Connect + overall deadlines so a hung provider endpoint surfaces
            // as an error instead of pinning the headless harness forever
            // (mirrors the desktop's builtin_http_client).
            let client = reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(15))
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());
            let resp = match client.post(&url).bearer_auth(&key).json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.unbounded_send(Err(LlmError {
                        message: format!("POST {url}: {e}"),
                        aborted: false,
                    }));
                    return;
                }
            };
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                let head: String = text.chars().take(300).collect();
                let _ = tx.unbounded_send(Err(LlmError {
                    message: format!("http {status}: {head}"),
                    aborted: false,
                }));
                return;
            }
            let content = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| {
                    v["choices"][0]["message"]["content"]
                        .as_str()
                        .map(str::to_string)
                })
                .unwrap_or_default();
            if dump {
                eprintln!(
                    "[DUMP] ===== LLM response ({} chars) =====\n{content}\n[DUMP] ===== end =====",
                    content.len()
                );
            }
            if content.trim().is_empty() {
                let _ = tx.unbounded_send(Err(LlmError {
                    message: "empty content from provider".into(),
                    aborted: false,
                }));
            } else {
                let _ = tx.unbounded_send(Ok(LlmChunk::Text(content)));
            }
        });
        Box::pin(rx)
    }
}
