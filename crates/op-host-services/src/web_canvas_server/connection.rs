//! One-connection handling: route dispatch across static assets, SSE, REST
//! and JSON-RPC (`serve_one`), plus the SSE writer it hands long-lived
//! subscribers to. Split out of `web_canvas_server.rs` to keep the spine
//! under the 800-line cap.

use super::*;

/// Handle one connection. Routes: static host page + wasm bundle (`GET /`,
/// `GET /pkg/*` via `crate::web_static`); SSE live-update stream (`GET
/// /api/mcp/events`); REST whole-doc sync / health (`/api/*` via
/// [`handle_web_canvas_request`]); else JSON-RPC `/mcp` tool dispatch. A
/// mutation (REST POST or a mutating tool call) bumps the version and is
/// broadcast to SSE subscribers. The state `Mutex` is held only across the
/// in-memory operation, never across the (long-lived) SSE wait.
///
/// Managed mode (`WebCanvasState::managed_token`) layers a token gate
/// (`RequestAuth::allows`) in front of every privileged branch below —
/// only the static GET routes and `OPTIONS` preflight stay tokenless — and
/// an origin allowlist (`cors_origin_for`) that replaces the permissive
/// `Access-Control-Allow-Origin: *` with an echo of the exact allowlisted
/// `Origin` (or no header at all). Non-managed mode is untouched: `auth`
/// always allows and `cors_origin` is always `Some("*")`.
///
/// Returns `Ok(true)` when the client requested a token-authed graceful
/// shutdown (same `openpencil/shutdown` contract as `--mcp-http`) — the
/// caller then stops the accept loop so `op stop` never signals a pid.
pub(super) fn serve_one<S: Read + Write>(
    stream: &mut S,
    state: &Mutex<WebCanvasState>,
    hub: &SseHub,
) -> Result<bool> {
    let req = crate::mcp_serve::read_http_request(stream)?;
    let (auth, allow_origins) = {
        let guard = state.lock().unwrap_or_else(|p| p.into_inner());
        let auth = RequestAuth {
            managed: guard.managed_token.is_some(),
            token: guard.managed_token.clone().unwrap_or_default(),
        };
        (auth, guard.allow_origins.clone())
    };
    let cors_origin: Option<String> = if auth.managed {
        cors_origin_for(&allow_origins, req.origin.as_deref())
    } else {
        Some("*".to_string())
    };
    let cors_origin = cors_origin.as_deref();
    if req.method == "OPTIONS" {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "204 No Content",
            "",
            cors_origin,
        )?;
        return Ok(false);
    }
    if is_sensitive_browser_post(&req) && !credential_request_origin_allowed(&req) {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "403 Forbidden",
            &crate::mcp_serve::rest_error_body("cross-origin sensitive request is forbidden"),
            cors_origin,
        )?;
        return Ok(false);
    }
    // Sensitive JSON routes refuse CORS "simple request" content types
    // (text/plain, form-encoded, or none): a drive-by page can fire those
    // without a preflight, and unmanaged daemons have no token gate.
    if is_sensitive_browser_post(&req) && !content_type_is_json(req.content_type.as_deref()) {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "415 Unsupported Media Type",
            &crate::mcp_serve::rest_error_body(
                "this route requires Content-Type: application/json",
            ),
            cors_origin,
        )?;
        return Ok(false);
    }
    // Static serving: the host page (`/`) and the wasm-bindgen bundle
    // (`/pkg/*`). Owns only those paths — everything else falls through.
    if req.method == "GET" {
        let bundle_dir = crate::web_static::resolve_bundle_dir();
        if let Some(reply) =
            crate::web_static::handle_static_request(&req.path, bundle_dir.as_deref())
        {
            return crate::web_static::write_static_response(stream, &reply, cors_origin)
                .map(|()| false);
        }
    }
    // Sign-in popup interstitial — same auth-exempt static surface as the
    // bundle routes above (it renders a spinner and nothing else).
    if req.method == "GET" && req.path == op_editor_core::auth_routes::LOADING_PAGE {
        let reply = crate::web_static::StaticReply {
            status: "200 OK",
            content_type: "text/html; charset=utf-8",
            body: crate::web_auth::LOADING_PAGE_HTML.as_bytes().to_vec(),
        };
        return crate::web_static::write_static_response(stream, &reply, cors_origin)
            .map(|()| false);
    }
    // Managed-mode token gate: everything below this point is a privileged
    // branch (SSE, AI streams, `/mcp`, `/api/*`, the `POST /` JSON-RPC
    // alias) — the static GET routes above and the `OPTIONS` preflight
    // already returned. Unmanaged mode's `allows` always returns true, so
    // this is a no-op there.
    if !auth.allows(&req.method, &req.path, req.token.as_deref()) {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "401 Unauthorized",
            r#"{"ok":false,"error":"unauthorized"}"#,
            cors_origin,
        )?;
        return Ok(false);
    }
    // Device-login begin: waits (per-connection thread, off the state
    // lock) for the pairing's verification URI so the popup can navigate
    // straight from this response — handled here rather than in the
    // whole-body REST tier, which runs under the state mutex.
    if req.method == "POST" && req.path == op_editor_core::auth_routes::LOGIN_BEGIN {
        let reply = crate::web_auth::login_begin_and_wait(state);
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            reply.status,
            &reply.body,
            cors_origin,
        )?;
        return Ok(false);
    }
    // SSE live-update stream: the browser shell subscribes and re-syncs whenever
    // the document version advances. Subscribe BEFORE reading the current
    // version so no broadcast is missed (a duplicate is harmless — versions are
    // monotonic). The state lock is released before the long SSE wait.
    if req.method == "GET" && req.path == "/api/mcp/events" {
        let rx = hub.subscribe();
        let current = state.lock().unwrap_or_else(|p| p.into_inner()).version;
        return serve_sse(stream, rx, current, cors_origin).map(|()| false);
    }
    // AI proxy stream: the browser bundle POSTs a model request and we
    // stream the provider's `ChatDelta`s back as SSE. Streaming route
    // (long-lived socket write), so handled here rather than in the
    // whole-body REST handler. Parse the body + build the provider
    // under the state lock, then DROP the lock before the long stream
    // — `proxy_provider` returns an owned `Box<dyn ChatProvider>`, so
    // nothing borrows the editor across the stream.
    if req.method == "POST" && req.path == "/api/ai/stream" {
        let Some(ai_req) = crate::ai_proxy::parse_ai_stream_body(&req.body) else {
            return crate::ai_proxy::write_sse_error(stream, "invalid request body", cors_origin)
                .map_err(|e| WebCanvasError::Transport(format!("ai stream error: {e}")))
                .map(|()| false);
        };
        let provider = {
            let guard = state.lock().unwrap_or_else(|p| p.into_inner());
            crate::ai_proxy::proxy_provider_for_request(
                &guard.editor,
                &ai_req,
                guard.credential_persistence,
            )
        };
        let provider = match provider {
            Ok(Some(provider)) => provider,
            Ok(None) => {
                return crate::ai_proxy::write_sse_error(
                    stream,
                    "no model configured",
                    cors_origin,
                )
                .map_err(|e| WebCanvasError::Transport(format!("ai stream error: {e}")))
                .map(|()| false);
            }
            Err(error) => {
                return crate::ai_proxy::write_sse_error(stream, &error.to_string(), cors_origin)
                    .map_err(|e| WebCanvasError::Transport(format!("ai stream error: {e}")))
                    .map(|()| false);
            }
        };
        return crate::ai_proxy::stream_ai_response(stream, ai_req, provider.as_ref(), cors_origin)
            .map_err(|e| WebCanvasError::Transport(format!("ai stream: {e}")))
            .map(|()| false);
    }
    // Standard web chat/design turn: same external-CLI routing shape as
    // desktop standard mode (classify → chat / modify / new design), but
    // applied against this web-canvas daemon's document authority.
    if req.method == "POST" && req.path == "/api/ai/standard" {
        let Some(standard_req) = crate::web_chat_standard::parse_standard_turn_body(&req.body)
        else {
            return crate::ai_proxy::write_sse_error(stream, "invalid request body", cors_origin)
                .map_err(|e| WebCanvasError::Transport(format!("ai standard error: {e}")))
                .map(|()| false);
        };
        return crate::web_chat_standard::stream_standard_turn(
            stream,
            standard_req,
            state,
            hub,
            cors_origin,
        )
        .map_err(|e| WebCanvasError::Transport(format!("ai standard: {e}")))
        .map(|()| false);
    }
    // Image panel Search popover (desktop `image_panel_host` parity). Long
    // blocking network (8 s timeout × ladder), so it runs on this
    // connection's own thread AFTER the brief parse-under-lock — the REST
    // handler below holds the state lock for its whole body and must not
    // host provider dials. Living under `/api/ai/` keeps it inside the
    // sensitive-POST origin gate and the managed-mode token gate.
    if req.method == "POST" && req.path == "/api/ai/image/search" {
        let parsed = {
            let guard = state.lock().unwrap_or_else(|p| p.into_inner());
            crate::web_image_search::parse_search_request(&req.body, &guard.editor)
        };
        let (status, body) = match parsed {
            Ok((query, credentials)) => {
                // One slot per running job — each holds this connection
                // thread for minutes of provider network, so unbounded
                // concurrency would exhaust the daemon's threads.
                match crate::web_image_search::ImageJobSlot::acquire() {
                    Some(_slot) => {
                        let outcome = crate::web_image_search::run_search_blocking(
                            &query,
                            credentials.as_ref(),
                        );
                        (
                            "200 OK",
                            crate::web_image_search::search_outcome_to_json(&outcome),
                        )
                    }
                    None => (
                        "429 Too Many Requests",
                        r#"{"ok":false,"error":"too many concurrent image requests"}"#.to_string(),
                    ),
                }
            }
            Err(error) => (
                "400 Bad Request",
                serde_json::json!({ "ok": false, "error": error.to_string() }).to_string(),
            ),
        };
        crate::mcp_serve::write_mcp_http_response_with_origin(stream, status, &body, cors_origin)?;
        return Ok(false);
    }
    // Image panel Generate popover (desktop `image_generate_host` parity).
    // Same threading rules as the search route; Replicate polling can run
    // for minutes.
    if req.method == "POST" && req.path == "/api/ai/image/generate" {
        let parsed = {
            let guard = state.lock().unwrap_or_else(|p| p.into_inner());
            crate::web_image_generate::parse_generate_request(&req.body, &guard.editor)
        };
        let (status, body) = match parsed {
            // Shares the search route's in-flight ceiling (see above).
            Ok(request) => match crate::web_image_search::ImageJobSlot::acquire() {
                Some(_slot) => match crate::web_image_generate::run_generate_blocking(&request) {
                    Ok(url) => ("200 OK", crate::web_image_generate::generate_ok_json(&url)),
                    Err(message) => (
                        "502 Bad Gateway",
                        crate::web_image_generate::generate_error_json(&message),
                    ),
                },
                None => (
                    "429 Too Many Requests",
                    r#"{"ok":false,"error":"too many concurrent image requests"}"#.to_string(),
                ),
            },
            Err(message) => (
                "400 Bad Request",
                crate::web_image_generate::generate_error_json(&message),
            ),
        };
        crate::mcp_serve::write_mcp_http_response_with_origin(stream, status, &body, cors_origin)?;
        return Ok(false);
    }
    // Offline `.fig` -> `.op` convert for the VS Code plugin: it can't parse
    // fig-kiwi itself, so it POSTs the raw bytes here and boots the returned
    // document JSON through its normal open-document push. Conversion is
    // pure (no network, no state) so — unlike the image routes above — it
    // runs on this connection's own thread without ever touching the state
    // lock; only large/slow parsing needs to stay off it.
    if req.method == "POST" && req.path == "/api/figma/convert" {
        let (status, body) = match crate::figma_convert::convert_fig_json(&req.body) {
            Ok(body) => ("200 OK", body),
            Err(error) => (
                "400 Bad Request",
                serde_json::json!({ "ok": false, "error": error.to_string() }).to_string(),
            ),
        };
        crate::mcp_serve::write_mcp_http_response_with_origin(stream, status, &body, cors_origin)?;
        return Ok(false);
    }
    // All `/api/mcp/*` REST paths go to the REST handler — including ones this
    // daemon doesn't implement yet, which it answers with 404 rather than
    // mis-routing them into the JSON-RPC dispatch below.
    if req.path.starts_with("/api/") {
        let reply = {
            let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
            let before = guard.version;
            let settings_before = crate::settings_io::fingerprint(&guard.editor);
            let credential_settings_before = (req.method == "POST"
                && req.path == "/api/settings/credentials")
                .then(|| guard.editor.editor_ui.agent_settings.clone());
            let reply = handle_web_canvas_request(&req.method, &req.path, &req.body, &mut guard);
            let reply = persist_api_settings(
                &req.method,
                &req.path,
                &mut guard,
                settings_before,
                credential_settings_before,
                reply,
                crate::settings_io::save_checked,
            );
            // Broadcast INSIDE the state lock so the version bump and its
            // broadcast are atomic — otherwise two concurrent mutations could
            // broadcast their versions out of order (SSE clients seeing N then
            // N-1). `broadcast` only sends to unbounded channels (non-blocking),
            // so the lock is held briefly. Lock order is always state→hub.
            if guard.version != before {
                hub.broadcast(guard.version);
            }
            reply
        };
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            reply.status,
            &reply.body,
            cors_origin,
        )?;
        return Ok(false);
    }
    // JSON-RPC tool dispatch is served ONLY as a POST to `/` or `/mcp`. An
    // unknown path is 404; a known path with the wrong method (e.g. `GET /mcp`)
    // is 405 — never silently dispatched as a tool call.
    let is_jsonrpc_path = req.path == "/" || req.path == "/mcp";
    if !is_jsonrpc_path {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "404 Not Found",
            r#"{"ok":false,"error":"Not found. Use /, /pkg/*, /api/mcp/document, /api/mcp/sync-reset, /api/mcp/server, /api/mcp/events, /api/file/save, /api/export/raster, /api/export/pdf, or /mcp."}"#,
            cors_origin,
        )?;
        return Ok(false);
    }
    if req.method != "POST" {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "405 Method Not Allowed",
            r#"{"ok":false,"error":"Method not allowed. POST a JSON-RPC message to /mcp."}"#,
            cors_origin,
        )?;
        return Ok(false);
    }
    // Token-authed graceful shutdown (`op stop`): same contract as the
    // `--mcp-http` server — only the exact per-instance token passed by the
    // spawning CLI (via OPENPENCIL_MCP_TOKEN) authenticates; a stale file, a
    // recycled pid, or a random client cannot shut the daemon down.
    if let Some(id) = crate::mcp_serve::shutdown_request_id(
        &req.body,
        &crate::mcp_serve::headless_token_from_env().unwrap_or_default(),
    ) {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "200 OK",
            &crate::mcp_serve::shutdown_ok_response(&id),
            cors_origin,
        )?;
        return Ok(true);
    }
    // `debug_screenshot` for `--serve-web`: the browser shell mirrors this
    // daemon's document, so the daemon can satisfy the live screenshot tool from
    // the same raster export path desktop live MCP uses. Keep this ahead of the
    // generic dispatch, whose headless debug tool can only report no live
    // canvas.
    #[cfg(feature = "mcp-debug-tools")]
    if let Some(response) = {
        let guard = state.lock().unwrap_or_else(|p| p.into_inner());
        crate::mcp_live::screenshot::maybe_serve(
            &req.body,
            op_mcp::debug_tools_enabled(),
            |shot_req| {
                let spec = crate::mcp_live::screenshot::capture_spec(&shot_req);
                crate::export::screenshot::capture(&guard.editor, &spec)
            },
        )
    } {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "200 OK",
            &response,
            cors_origin,
        )?;
        return Ok(false);
    }
    // JSON-RPC `/mcp` dispatch against the in-memory document. A mutating apply
    // bumps the sync version, broadcast to SSE subscribers so the browser shell
    // sees JSON-RPC-driven changes too.
    let response = {
        let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
        let before = guard.version;
        let mut applied_any = false;
        // Mechanical passthrough only — this daemon (`--serve-web`/`op
        // start`) is a SEPARATE request loop from `mcp_live.rs`'s
        // `McpLiveServer` (desktop `--live-mcp`), not the same struct;
        // wiring canvas-generation indicators here (so a `batch_design`
        // call against a headless `op start` daemon also relays the
        // radar-scan to the browser shell) is tracked as follow-up
        // scope, not part of this pass.
        let response = crate::mcp_serve::process_message_with_applier(
            &mut guard.editor,
            &req.body,
            |_tool_name, editor, cmd| {
                let ok = editor.apply(cmd.clone());
                applied_any |= ok;
                ok
            },
        )?
        .unwrap_or_default();
        if applied_any {
            guard.version += 1;
        }
        // Atomic bump+broadcast under the state lock (see the REST path) so SSE
        // version events stay monotonic across concurrent mutations.
        if guard.version != before {
            hub.broadcast(guard.version);
        }
        response
    };
    let status = if response.is_empty() {
        "202 Accepted"
    } else {
        "200 OK"
    };
    crate::mcp_serve::write_mcp_http_response_with_origin(stream, status, &response, cors_origin)?;
    Ok(false)
}

/// Stream Server-Sent Events to a subscribed client: write the SSE headers,
/// emit the current version immediately (initial sync), then forward each
/// version bump from `rx` as a `data: {"version":N}` event. A periodic
/// heartbeat comment keeps the connection alive AND detects a disconnected
/// client (the write fails once the socket is gone). Returns when the client
/// disconnects (write error) or the hub is dropped.
pub(super) fn serve_sse<S: Write>(
    stream: &mut S,
    rx: Receiver<u64>,
    current_version: u64,
    cors_origin: Option<&str>,
) -> Result<()> {
    let cors_line = cors_origin
        .map(|origin| format!("Access-Control-Allow-Origin: {origin}\r\n"))
        .unwrap_or_default();
    let headers = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         Connection: keep-alive\r\n\
         {cors_line}\r\n"
    );
    stream
        .write_all(headers.as_bytes())
        .map_err(|e| WebCanvasError::Transport(format!("sse headers: {e}")))?;
    write_sse_event(stream, current_version)?;
    loop {
        match rx.recv_timeout(SSE_HEARTBEAT) {
            Ok(mut version) => {
                // Coalesce any further queued bumps — only the latest version
                // matters (the client re-fetches the whole document on it), so
                // a burst of mutations collapses to a single event and the
                // channel can't accumulate unboundedly behind a slow client.
                while let Ok(next) = rx.try_recv() {
                    version = next;
                }
                write_sse_event(stream, version)?;
            }
            Err(RecvTimeoutError::Timeout) => {
                // SSE comment heartbeat — no-op for the client, but a failed
                // write here is how we notice it disconnected.
                stream
                    .write_all(b": ping\n\n")
                    .map_err(|e| WebCanvasError::Transport(format!("sse heartbeat: {e}")))?;
                stream
                    .flush()
                    .map_err(|e| WebCanvasError::Transport(format!("sse flush: {e}")))?;
            }
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}

/// Format + write one SSE `data:` event carrying the document version.
pub(super) fn write_sse_event<S: Write>(stream: &mut S, version: u64) -> Result<()> {
    let event = format!("data: {{\"version\":{version}}}\n\n");
    stream
        .write_all(event.as_bytes())
        .map_err(|e| WebCanvasError::Transport(format!("sse write: {e}")))?;
    stream
        .flush()
        .map_err(|e| WebCanvasError::Transport(format!("sse flush: {e}")))
}
