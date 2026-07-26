//! Minimal JSON-RPC 2.0 engine over the ndJSON transport.
//!
//! [`JsonRpcEngine`] allocates request ids, correlates responses
//! through per-id oneshot channels, and — via [`dispatch_inbound`] —
//! routes inbound frames: responses to their waiter, `session/update`
//! notifications to a channel, and `session/request_permission`
//! requests to an auto-approval reply (TS parity — the user already
//! trusted the agent by configuring it).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use crate::protocol::{
    classify_inbound, Inbound, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
    RequestPermissionParams, SessionNotification, METHOD_REQUEST_PERMISSION, METHOD_SESSION_UPDATE,
};
use crate::types::AcpError;

/// Backpressure ceiling for the outbound-frame queue (requests we send plus
/// the auto-generated replies to agent → client requests). Deliberately
/// generous: a healthy session never queues more than a handful of frames,
/// so hitting this means the writer (or the agent reading it) has stalled.
pub const OUTBOUND_CAPACITY: usize = 1024;

/// Backpressure ceiling for buffered `session/update` notifications. A chatty
/// or malicious agent can stream these faster than the UI drains them; the
/// bound turns unbounded memory growth into bounded, logged drops.
pub const NOTIFICATION_CAPACITY: usize = 1024;

/// Map of in-flight request id → response waiter.
type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, AcpError>>>>>;

/// Count of frames/notifications dropped because a bounded queue was full.
/// Used only to keep the drop log from becoming its own flood.
static DROPPED: AtomicU64 = AtomicU64::new(0);

/// Non-blocking enqueue for the reader task. A FULL queue means the consumer
/// fell behind a flooding agent — drop the message and log, rate-limited to
/// the first drop and then every 256th so the log cannot become its own flood.
/// A CLOSED queue is the ordinary teardown path (the consumer went away) and
/// is silent.
fn offer<T>(tx: &mpsc::Sender<T>, message: T, what: &str) {
    if let Err(mpsc::error::TrySendError::Full(_)) = tx.try_send(message) {
        let count = DROPPED.fetch_add(1, Ordering::Relaxed) + 1;
        if count == 1 || count.is_multiple_of(256) {
            eprintln!("[op-acp] dropped {what}: queue full (total dropped: {count})");
        }
    }
}

/// Shared JSON-RPC engine — cloned between the connection handle and
/// the background reader task.
#[derive(Clone)]
pub struct JsonRpcEngine {
    out_tx: mpsc::Sender<Value>,
    pending: Pending,
    next_id: Arc<AtomicU64>,
}

impl JsonRpcEngine {
    /// Build an engine that writes outbound frames to `out_tx`.
    pub fn new(out_tx: mpsc::Sender<Value>) -> Self {
        Self {
            out_tx,
            pending: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// The shared pending-request map (the reader task resolves it).
    pub fn pending(&self) -> Pending {
        self.pending.clone()
    }

    /// A clone of the outbound-frame sender.
    pub fn out_tx(&self) -> mpsc::Sender<Value> {
        self.out_tx.clone()
    }

    /// Send a request and await its correlated response, up to
    /// `timeout`.
    ///
    /// `timeout` is a deadline for the WHOLE call, enqueue included: the
    /// outbound queue is bounded, so a stalled writer must not silently double
    /// the caller's budget. Awaiting the send is safe here — `call` runs on the
    /// caller's task, never on the writer task that drains the queue, so it
    /// cannot block its own drain.
    pub async fn call(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, AcpError> {
        let deadline = tokio::time::Instant::now() + timeout;
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(id, tx);

        let req = JsonRpcRequest::new(id, method, params);
        let frame = serde_json::to_value(&req).map_err(|e| AcpError::Protocol(e.to_string()))?;
        let forget = || {
            self.pending
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .remove(&id);
        };
        match tokio::time::timeout_at(deadline, self.out_tx.send(frame)).await {
            Ok(Ok(())) => {}
            // The writer task dropped the receiver — connection died.
            Ok(Err(_)) => {
                forget();
                return Err(AcpError::Closed);
            }
            Err(_) => {
                forget();
                return Err(AcpError::Transport(format!(
                    "request '{method}' timed out queueing for the agent"
                )));
            }
        }

        match tokio::time::timeout_at(deadline, rx).await {
            Ok(Ok(result)) => result,
            // The reader task dropped the sender — connection died.
            Ok(Err(_)) => Err(AcpError::Closed),
            Err(_) => {
                forget();
                Err(AcpError::Transport(format!("request '{method}' timed out")))
            }
        }
    }
}

/// Choose an "allow" option from a `session/request_permission`
/// request and build the JSON-RPC result that selects it. Mirrors the
/// TS `requestPermission` handler.
pub fn auto_approve_permission(params: &Value) -> Value {
    let parsed: RequestPermissionParams = serde_json::from_value(params.clone())
        .unwrap_or(RequestPermissionParams { options: vec![] });
    let chosen = parsed
        .options
        .iter()
        .find(|o| {
            matches!(o.kind.as_deref(), Some("allow_once") | Some("allow_always"))
                || o.option_id.starts_with("allow")
        })
        .or_else(|| parsed.options.first());
    let option_id = chosen
        .map(|o| o.option_id.clone())
        .unwrap_or_else(|| "allow".to_string());
    serde_json::json!({
        "outcome": { "outcome": "selected", "optionId": option_id }
    })
}

/// Route one inbound JSON frame: resolve a response waiter, forward a
/// `session/update` notification, or reply to an agent → client
/// request.
///
/// Called synchronously from the reader task, which is also the only producer
/// the notification consumer and the writer task can be starved by. Both
/// bounded queues are therefore fed with `try_send`, never an awaited send:
/// blocking the reader would stop it from resolving in-flight responses (and,
/// for `out_tx`, from ever draining the very queue it is waiting on). A full
/// queue drops the message with a rate-limited log instead.
pub fn dispatch_inbound(
    value: Value,
    pending: &Pending,
    notif_tx: &mpsc::Sender<SessionNotification>,
    out_tx: &mpsc::Sender<Value>,
) {
    match classify_inbound(&value) {
        Inbound::Response { id, result, error } => {
            if let Some(tx) = pending
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .remove(&id)
            {
                let resolved = match error {
                    Some(e) => Err(AcpError::Rpc {
                        code: e.code,
                        message: e.message,
                    }),
                    None => Ok(result.unwrap_or(Value::Null)),
                };
                let _ = tx.send(resolved);
            }
        }
        Inbound::Request { id, method, params } => {
            let response = if method == METHOD_REQUEST_PERMISSION {
                JsonRpcResponse::ok(id, auto_approve_permission(&params))
            } else {
                // Unsupported agent → client request — reply with a
                // JSON-RPC "method not found" so the agent fails fast
                // instead of hanging.
                JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32601,
                        message: format!("method '{method}' not supported"),
                        data: None,
                    }),
                }
            };
            if let Ok(frame) = serde_json::to_value(&response) {
                offer(out_tx, frame, "outbound reply");
            }
        }
        Inbound::Notification { method, params } => {
            if method == METHOD_SESSION_UPDATE {
                if let Ok(note) = serde_json::from_value::<SessionNotification>(params) {
                    offer(notif_tx, note, "session/update notification");
                }
            }
            // Other notifications are not surfaced.
        }
        Inbound::Unknown => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_approve_prefers_allow_option() {
        let params = serde_json::json!({
            "options": [
                { "optionId": "reject-1", "kind": "reject_once" },
                { "optionId": "ok-1", "kind": "allow_always" }
            ]
        });
        let out = auto_approve_permission(&params);
        assert_eq!(out["outcome"]["outcome"], "selected");
        assert_eq!(out["outcome"]["optionId"], "ok-1");
    }

    #[test]
    fn auto_approve_falls_back_to_first_option() {
        let params = serde_json::json!({
            "options": [{ "optionId": "first", "kind": "custom" }]
        });
        assert_eq!(
            auto_approve_permission(&params)["outcome"]["optionId"],
            "first"
        );
        // No options at all → the generic "allow" sentinel.
        let empty = serde_json::json!({ "options": [] });
        assert_eq!(
            auto_approve_permission(&empty)["outcome"]["optionId"],
            "allow"
        );
    }

    #[tokio::test]
    async fn call_correlates_a_response() {
        let (out_tx, mut out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let engine = JsonRpcEngine::new(out_tx);
        let (notif_tx, _notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
        let pending = engine.pending();
        let reply_tx = engine.out_tx();

        // Background "agent": read the request, echo a response.
        tokio::spawn(async move {
            let req = out_rx.recv().await.unwrap();
            let id = req["id"].as_u64().unwrap();
            let response = serde_json::json!({
                "jsonrpc": "2.0", "id": id, "result": { "pong": true }
            });
            dispatch_inbound(response, &pending, &notif_tx, &reply_tx);
        });

        let result = engine
            .call("ping", serde_json::json!({}), Duration::from_secs(2))
            .await
            .unwrap();
        assert_eq!(result["pong"], true);
    }

    #[tokio::test]
    async fn call_surfaces_rpc_error() {
        let (out_tx, mut out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let engine = JsonRpcEngine::new(out_tx);
        let (notif_tx, _notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
        let pending = engine.pending();
        let reply_tx = engine.out_tx();
        tokio::spawn(async move {
            let req = out_rx.recv().await.unwrap();
            let id = req["id"].as_u64().unwrap();
            let err = serde_json::json!({
                "jsonrpc": "2.0", "id": id,
                "error": { "code": -32000, "message": "boom" }
            });
            dispatch_inbound(err, &pending, &notif_tx, &reply_tx);
        });
        let err = engine
            .call("fail", serde_json::json!({}), Duration::from_secs(2))
            .await
            .unwrap_err();
        assert!(matches!(err, AcpError::Rpc { code: -32000, .. }));
    }

    /// Build one `session/update` frame carrying `session` as its id.
    fn update_frame(session: &str) -> Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {
                "sessionId": session,
                "update": { "sessionUpdate": "agent_message_chunk",
                            "content": { "type": "text", "text": "spam" } }
            }
        })
    }

    #[tokio::test]
    async fn a_flooding_agent_cannot_grow_the_notification_queue_without_bound() {
        let (out_tx, _out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let (notif_tx, mut notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));

        // Nobody is draining — dispatch far more updates than the queue holds.
        // `dispatch_inbound` is sync, so this also proves it never blocks the
        // reader task waiting for room.
        let flood = NOTIFICATION_CAPACITY + 500;
        for index in 0..flood {
            dispatch_inbound(
                update_frame(&format!("s{index}")),
                &pending,
                &notif_tx,
                &out_tx,
            );
        }

        let mut buffered = 0;
        while notif_rx.try_recv().is_ok() {
            buffered += 1;
        }
        assert_eq!(
            buffered, NOTIFICATION_CAPACITY,
            "queue must cap at its capacity, not grow to the flood size"
        );
    }

    #[tokio::test]
    async fn responses_still_resolve_after_the_notification_queue_overflows() {
        // The load-bearing property: dropping overflow must not wedge the
        // reader, so an in-flight request still gets its answer.
        let (out_tx, mut out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let engine = JsonRpcEngine::new(out_tx);
        let (notif_tx, _notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
        let pending = engine.pending();
        let reply_tx = engine.out_tx();

        tokio::spawn(async move {
            let req = out_rx.recv().await.unwrap();
            let id = req["id"].as_u64().unwrap();
            for index in 0..(NOTIFICATION_CAPACITY + 50) {
                dispatch_inbound(
                    update_frame(&format!("s{index}")),
                    &pending,
                    &notif_tx,
                    &reply_tx,
                );
            }
            let response = serde_json::json!({
                "jsonrpc": "2.0", "id": id, "result": { "pong": true }
            });
            dispatch_inbound(response, &pending, &notif_tx, &reply_tx);
        });

        let result = engine
            .call("ping", serde_json::json!({}), Duration::from_secs(5))
            .await
            .expect("response must land despite the notification flood");
        assert_eq!(result["pong"], true);
    }

    #[tokio::test]
    async fn a_full_outbound_queue_never_blocks_the_reader() {
        // Permission auto-replies are produced by the reader task itself. If
        // the writer stalled, an awaited send here would deadlock the reader
        // against the queue it is the only producer for.
        let (out_tx, _out_rx) = mpsc::channel::<Value>(1);
        let (notif_tx, _notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let permission = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "session/request_permission",
            "params": { "options": [{ "optionId": "allow-1", "kind": "allow_once" }] }
        });
        // First fills the single slot, the rest overflow — none may block.
        for _ in 0..4 {
            dispatch_inbound(permission.clone(), &pending, &notif_tx, &out_tx);
        }
    }

    #[tokio::test]
    async fn call_fails_fast_when_the_outbound_queue_is_closed() {
        let (out_tx, out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        drop(out_rx);
        let engine = JsonRpcEngine::new(out_tx);
        let err = engine
            .call("ping", serde_json::json!({}), Duration::from_secs(5))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AcpError::Closed),
            "expected Closed, got {err:?}"
        );
    }

    #[tokio::test]
    async fn notification_reaches_the_channel() {
        let (out_tx, _out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let (notif_tx, mut notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let note = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {
                "sessionId": "s1",
                "update": { "sessionUpdate": "agent_message_chunk",
                            "content": { "type": "text", "text": "hi" } }
            }
        });
        dispatch_inbound(note, &pending, &notif_tx, &out_tx);
        let received = notif_rx.recv().await.unwrap();
        assert_eq!(received.session_id.as_deref(), Some("s1"));
    }
}
