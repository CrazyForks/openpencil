//! The `/api/ai/*` branch of the connection dispatch.
//!
//! Pure code motion out of `connection.rs` at the 800-line cap — the four
//! routes below are byte-for-byte the ones that lived there, in the same
//! order, and they keep the property that put them in the connection tier in
//! the first place: each does long-running network work and therefore parses
//! under the state lock, then drops it before the stream / dial.

use super::*;

/// Serve one `/api/ai/*` route.
///
/// `Ok(None)` means the request is not an AI route and the caller should keep
/// walking its own table. `Ok(Some(shutdown))` mirrors `serve_one`'s return.
pub(super) fn serve_ai_route<S: Read + Write>(
    stream: &mut S,
    req: &crate::mcp_serve::HttpRequest,
    ctx: &ConnCtx<'_>,
    cors_origin: Option<&str>,
) -> Result<Option<bool>> {
    if req.method != "POST" {
        return Ok(None);
    }
    let state = ctx.state;
    match req.path.as_str() {
        // AI proxy stream: the browser bundle POSTs a model request and we
        // stream the provider's `ChatDelta`s back as SSE. Streaming route
        // (long-lived socket write), so handled here rather than in the
        // whole-body REST handler. Parse the body + build the provider
        // under the state lock, then DROP the lock before the long stream
        // — `proxy_provider` returns an owned `Box<dyn ChatProvider>`, so
        // nothing borrows the editor across the stream.
        "/api/ai/stream" => {
            let Some(ai_req) = crate::ai_proxy::parse_ai_stream_body(&req.body) else {
                return crate::ai_proxy::write_sse_error(
                    stream,
                    "invalid request body",
                    cors_origin,
                )
                .map_err(|e| WebCanvasError::Transport(format!("ai stream error: {e}")))
                .map(|()| Some(false));
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
                    .map(|()| Some(false));
                }
                Err(error) => {
                    return crate::ai_proxy::write_sse_error(
                        stream,
                        &error.to_string(),
                        cors_origin,
                    )
                    .map_err(|e| WebCanvasError::Transport(format!("ai stream error: {e}")))
                    .map(|()| Some(false));
                }
            };
            crate::ai_proxy::stream_ai_response(stream, ai_req, provider.as_ref(), cors_origin)
                .map_err(|e| WebCanvasError::Transport(format!("ai stream: {e}")))
                .map(|()| Some(false))
        }
        // Standard web chat/design turn: same external-CLI routing shape as
        // desktop standard mode (classify → chat / modify / new design), but
        // applied against this web-canvas daemon's document authority.
        "/api/ai/standard" => {
            let Some(standard_req) = crate::web_chat_standard::parse_standard_turn_body(&req.body)
            else {
                return crate::ai_proxy::write_sse_error(
                    stream,
                    "invalid request body",
                    cors_origin,
                )
                .map_err(|e| WebCanvasError::Transport(format!("ai standard error: {e}")))
                .map(|()| Some(false));
            };
            crate::web_chat_standard::stream_standard_turn(
                stream,
                standard_req,
                state,
                ctx.hub,
                cors_origin,
            )
            .map_err(|e| WebCanvasError::Transport(format!("ai standard: {e}")))
            .map(|()| Some(false))
        }
        // Image panel Search popover (desktop `image_panel_host` parity). Long
        // blocking network (8 s timeout × ladder), so it runs on this
        // connection's own thread AFTER the brief parse-under-lock — the REST
        // handler holds the state lock for its whole body and must not host
        // provider dials. Living under `/api/ai/` keeps it inside the
        // sensitive-POST origin gate and the managed-mode token gate.
        "/api/ai/image/search" => {
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
                            r#"{"ok":false,"error":"too many concurrent image requests"}"#
                                .to_string(),
                        ),
                    }
                }
                Err(error) => (
                    "400 Bad Request",
                    serde_json::json!({ "ok": false, "error": error.to_string() }).to_string(),
                ),
            };
            crate::mcp_serve::write_mcp_http_response_with_origin(
                stream,
                status,
                &body,
                cors_origin,
            )?;
            Ok(Some(false))
        }
        // Image panel Generate popover (desktop `image_generate_host` parity).
        // Same threading rules as the search route; Replicate polling can run
        // for minutes.
        "/api/ai/image/generate" => {
            let parsed = {
                let guard = state.lock().unwrap_or_else(|p| p.into_inner());
                crate::web_image_generate::parse_generate_request(&req.body, &guard.editor)
            };
            let (status, body) = match parsed {
                // Shares the search route's in-flight ceiling (see above).
                Ok(request) => match crate::web_image_search::ImageJobSlot::acquire() {
                    Some(_slot) => match crate::web_image_generate::run_generate_blocking(&request)
                    {
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
            crate::mcp_serve::write_mcp_http_response_with_origin(
                stream,
                status,
                &body,
                cors_origin,
            )?;
            Ok(Some(false))
        }
        _ => Ok(None),
    }
}
