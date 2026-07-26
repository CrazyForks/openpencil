//! TS-parity whole-document sync (`POST /api/mcp/document`) and the shared
//! JSON-RPC response writer. Split out of `mcp_live.rs` to keep the spine
//! under the 800-line cap.

use super::*;

/// Monotonic counter for the whole-document sync `version` reported back to a
/// TS whole-doc-sync client (parity with `setSyncDocument`'s monotonic
/// version). Process-global so concurrent connections still get distinct,
/// increasing versions; the exact starting value is not part of the contract.
pub(super) static LIVE_SYNC_VERSION: AtomicU64 = AtomicU64::new(0);

/// Handle a TS-style whole-document sync (`POST /api/mcp/document`): validate
/// the body, swap the document into the live `EditorState` on the UI thread,
/// and reply `{ok:true,version}` (or HTTP 400 with `{ok:false,error}`).
pub(super) fn serve_document_sync<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    req_tx: &Sender<UiRequest>,
    wake_ui: &UiWake,
    stateful_lock: &Mutex<()>,
    body: &str,
) -> Result<(), String> {
    let request = match crate::mcp_serve::parse_document_sync_request(body) {
        Ok(request) => request,
        Err(message) => {
            return crate::mcp_serve::write_mcp_http_response(
                stream,
                "400 Bad Request",
                &crate::mcp_serve::rest_error_body(&message),
            );
        }
    };
    let editor_meta = request.resolved_editor_meta(request.embedded_editor_meta);
    if request.metadata_only {
        let _guard = stateful_lock
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        return match request_update_editor_meta(req_tx, wake_ui, editor_meta) {
            Ok(()) => crate::mcp_serve::write_mcp_http_response(
                stream,
                "200 OK",
                &crate::mcp_serve::document_sync_ok(LIVE_SYNC_VERSION.load(Ordering::Relaxed)),
            ),
            Err(transport_err) => crate::mcp_serve::write_mcp_http_response(
                stream,
                "500 Internal Server Error",
                &crate::mcp_serve::rest_error_body(&transport_err),
            ),
        };
    }
    // Load OFF the UI thread (parse only — no skia; layout runs at paint) so a
    // large document never pins the UI thread or holds the lock during the
    // parse. A load failure is a client fault → 400, like the TS validation
    // 400s in `document.post.ts`.
    let loaded = match op_pen_loader::load_canonical(request.document_json) {
        Ok(loaded) => loaded,
        Err(e) => {
            return crate::mcp_serve::write_mcp_http_response(
                stream,
                "400 Bad Request",
                &crate::mcp_serve::rest_error_body(&e.to_string()),
            );
        }
    };
    for w in &loaded.warnings {
        eprintln!("openpencil-desktop mcp: document-sync schema warning: {w:?}");
    }
    // Serialize the swap against concurrent live writes (same lock the JSON-RPC
    // apply path takes). The lock is now held only for the fast in-memory
    // replace, not the parse — so a huge document can't time out mid-load while
    // still eventually mutating the doc.
    let _guard = stateful_lock
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    match request_replace(req_tx, wake_ui, loaded.value, editor_meta) {
        Ok(()) => {
            let version = LIVE_SYNC_VERSION.fetch_add(1, Ordering::Relaxed) + 1;
            crate::mcp_serve::write_mcp_http_response(
                stream,
                "200 OK",
                &crate::mcp_serve::document_sync_ok(version),
            )
        }
        // The UI thread is gone or didn't ack in time — a server fault, mapped
        // to 500 (TS throws → 500 for server-side failures).
        Err(transport_err) => crate::mcp_serve::write_mcp_http_response(
            stream,
            "500 Internal Server Error",
            &crate::mcp_serve::rest_error_body(&transport_err),
        ),
    }
}

pub(super) fn write_json_rpc_response<S: std::io::Write>(
    stream: &mut S,
    response: &str,
) -> Result<(), String> {
    if response.is_empty() {
        crate::mcp_serve::write_mcp_http_response(stream, "202 Accepted", "")
    } else {
        crate::mcp_serve::write_mcp_http_response(stream, "200 OK", response)
    }
}
