//! `/api/collab/*` handlers.
//!
//! All three run inside the REST tier, so they hold the state lock for their
//! whole body and must never block on IO. They are deliberately thin: the
//! session work happens on the driver thread, and a posted action only lands
//! in the pending slot the driver drains.

use op_editor_core::collab_wire::{
    CollabActionWire, CollabActionWireError, CollabLocalPresenceWire, CollabStateWire,
};

use super::{WebCanvasState, WebReply};

/// Longest body either POST route accepts.
///
/// Both bodies are a handful of scalars; anything larger is a client bug or an
/// attempt to make the daemon allocate.
const MAX_COLLAB_BODY_BYTES: usize = 8 * 1024;

/// `GET /api/collab/state` — the whole projection plus both sequence numbers.
pub(crate) fn state(state: &mut WebCanvasState) -> WebReply {
    let wire = CollabStateWire::from_ui(
        &state.editor.editor_ui.collab,
        state.collab.seq(),
        state.editor.document_revision(),
    );
    match serde_json::to_string(&wire) {
        Ok(body) => WebReply {
            status: "200 OK",
            body,
        },
        Err(error) => WebReply {
            status: "500 Internal Server Error",
            body: crate::mcp_serve::rest_error_body(&error.to_string()),
        },
    }
}

/// `POST /api/collab/action` — enqueue one UI action for the driver.
///
/// Returns 202 without waiting: the runtime call can start a network worker,
/// and the REST tier holds the state lock. The driver is woken so the action is
/// picked up on the next tick rather than at the idle timeout.
pub(crate) fn action(body: &str, state: &mut WebCanvasState) -> WebReply {
    if body.len() > MAX_COLLAB_BODY_BYTES {
        return error_reply(
            "413 Payload Too Large",
            "payload-too-large",
            "body too large",
        );
    }
    let wire: CollabActionWire = match serde_json::from_str(body) {
        Ok(wire) => wire,
        Err(error) => {
            return error_reply("400 Bad Request", "malformed-action", &error.to_string())
        }
    };
    // Hook point for the public multi-account deployment: it must refuse
    // `wire.reaches_caller_named_network()` here, because those actions resolve
    // a caller-named socket address or enumerate the host's LAN. The local and
    // managed daemons allow them — that is desktop parity, and the operator is
    // the only client.
    let action = match wire.into_ui_action() {
        Ok(action) => action,
        Err(error) => return action_error_reply(error),
    };
    // The pending slot holds exactly one action, and the runtime consumes it
    // through `take_pending_action`. Overwriting an undrained action would
    // silently drop whatever the user asked for first, so this reports the
    // collision instead.
    if state.editor.editor_ui.collab.pending_action.is_some() {
        return error_reply(
            "409 Conflict",
            "collab-busy",
            "a collaboration action is already queued",
        );
    }
    state.editor.editor_ui.collab.pending_action = Some(action);
    let seq = state.collab.bump_seq();
    state.collab.wake_driver();
    WebReply {
        status: "202 Accepted",
        body: serde_json::json!({ "ok": true, "collabSeq": seq }).to_string(),
    }
}

/// `POST /api/collab/presence` — publish the local cursor.
///
/// Stored rather than sent: the driver replays it through
/// `publish_local_presence`, which is throttled to one frame interval. Posting
/// faster than that costs nothing beyond the request itself.
pub(crate) fn presence(body: &str, state: &mut WebCanvasState) -> WebReply {
    if body.len() > MAX_COLLAB_BODY_BYTES {
        return error_reply(
            "413 Payload Too Large",
            "payload-too-large",
            "body too large",
        );
    }
    let wire: CollabLocalPresenceWire = match serde_json::from_str(body) {
        Ok(wire) => wire,
        Err(error) => {
            return error_reply("400 Bad Request", "malformed-presence", &error.to_string())
        }
    };
    // A presence update is not a projection change for the poller's purposes —
    // it is the local peer's own cursor going out, not incoming state — so this
    // deliberately does not bump `collabSeq`.
    state
        .collab
        .set_presence_override(wire.cursor.map(|point| (point.x, point.y)));
    state.collab.wake_driver();
    WebReply {
        status: "202 Accepted",
        body: r#"{"ok":true}"#.into(),
    }
}

fn action_error_reply(error: CollabActionWireError) -> WebReply {
    let code = match error {
        CollabActionWireError::InvalidRequestKey => "invalid-request-key",
        CollabActionWireError::InvalidAddress => "invalid-address",
    };
    error_reply("400 Bad Request", code, &error.to_string())
}

fn error_reply(status: &'static str, code: &str, message: &str) -> WebReply {
    WebReply {
        status,
        body: serde_json::json!({
            "ok": false,
            "error": code,
            "message": message,
        })
        .to_string(),
    }
}

#[cfg(test)]
#[path = "collab_routes_tests.rs"]
mod tests;
