//! Drive the daemon's collaboration runtime from the web shell.
//!
//! The wasm bundle carries the collaboration *panel* but no transport: a relay
//! session needs device credentials only a native process can hold, so the
//! session lives in the daemon and this module is the wire between them. It is
//! the collaboration twin of [`crate::web_auth_sync`] and follows its shape —
//! one interval, `thread_local` latches, every write through the shared
//! `RepaintContext`.
//!
//! Three jobs per tick, in this order:
//!
//! 1. drain the panel's queued [`CollabUiAction`] to `POST /api/collab/action`;
//! 2. pull `GET /api/collab/state` when the projection moved, and install it;
//! 3. publish the local cursor to `POST /api/collab/presence`.
//!
//! ## Two counters, two meanings
//!
//! The daemon publishes `documentRevision` and `collabSeq` separately.
//! [`crate::live_sync_glue`]'s version poll already runs every 400 ms, so it
//! hands the observed `collabSeq` to [`note_version_probe`] instead of this
//! module opening a second probe. A `collabSeq` bump pulls *state*; it must
//! never pull the document, or every remote cursor move would refetch the whole
//! canvas.
//!
//! ## Availability
//!
//! Only ever what the daemon projected. Before the first successful `/state`
//! the panel keeps its default `Unavailable`, so a shell that cannot reach a
//! daemon offers nothing rather than offering a session it cannot start.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use op_editor_core::collab_wire::{
    CollabActionWire, CollabLocalPresenceWire, CollabPointWire, CollabStateWire,
};
use op_editor_core::{collab_routes, CollabConnectionPhase, CollabUiAction};

use crate::live_sync;
use crate::repaint_ctx::RepaintContext;

/// Tick cadence with no session — matched to the document version poll, since
/// with nothing running there is nothing to be responsive about.
const IDLE_TICK_MS: i32 = 400;
/// Tick cadence once a session exists. Presence and admission prompts are the
/// interactive parts of collaboration; 150 ms keeps them from feeling laggy
/// without approaching a per-frame request rate.
const SESSION_TICK_MS: i32 = 150;
/// Floor between presence posts. The daemon throttles again on its side to one
/// frame interval, so anything under this is spent bytes for no visible gain.
const PRESENCE_MIN_INTERVAL_MS: f64 = 100.0;

thread_local! {
    /// Latest `collabSeq` seen on the wire, from either probe.
    static OBSERVED_SEQ: Cell<u64> = const { Cell::new(0) };
    /// `collabSeq` of the projection currently installed in the panel.
    /// `None` until the first `/state` lands, which is what forces that first
    /// pull regardless of the counter.
    static APPLIED_SEQ: Cell<Option<u64>> = const { Cell::new(None) };
    /// One `/state` request in flight at a time.
    static STATE_BUSY: Cell<bool> = const { Cell::new(false) };
    /// An action already posted and not yet answered.
    static ACTION_BUSY: Cell<bool> = const { Cell::new(false) };
    /// An action the daemon refused with `collab-busy`, kept for the next tick.
    /// Losing it would silently drop something the user clicked.
    static ACTION_RETRY: RefCell<Option<CollabUiAction>> = const { RefCell::new(None) };
    /// `performance.now()` of the last presence post.
    static LAST_PRESENCE_MS: Cell<f64> = const { Cell::new(f64::NEG_INFINITY) };
    /// Last cursor actually sent, so an unmoved cursor costs nothing.
    static LAST_PRESENCE_POINT: Cell<Option<(f64, f64)>> = const { Cell::new(None) };
    /// Node ids present in the document the daemon last handed us.
    ///
    /// The browser mints ids from a local sequential counter, which is exactly
    /// what an active session cannot accept — see [`push_blocked_by_session`].
    static DAEMON_NODE_IDS: RefCell<Option<HashSet<op_editor_core::NodeId>>> =
        const { RefCell::new(None) };
}

/// Feed the shared version probe's `collabSeq` in.
///
/// `live_sync_glue` already polls `GET /api/mcp/version` every 400 ms and the
/// daemon answers both counters there, so collaboration rides that request
/// rather than opening a second one.
pub(crate) fn note_version_probe(body: &str) {
    if let Some(seq) = op_editor_core::collab_wire::parse_collab_seq_probe(body) {
        OBSERVED_SEQ.set(seq);
    }
}

/// Record the node ids of a document just applied from the daemon.
pub(crate) fn note_daemon_document(state: &op_editor_core::EditorState) {
    DAEMON_NODE_IDS.with(|ids| *ids.borrow_mut() = Some(document_node_ids(state)));
}

/// Whether a live session forbids pushing the current local document.
///
/// The browser has no owner-assigned id namespace: it mints `n<counter>` from a
/// local sequential allocator, and two peers creating a node in the same moment
/// would mint the same id. The collaboration protocol replays those ids
/// verbatim, so a colliding pair silently forks the document — the failure this
/// refuses to produce.
///
/// The check is deliberately whole-document rather than per-gesture: draw,
/// duplicate, paste, group and import all mint through the same counter, and
/// gating the single push covers every one of them without a guard at each
/// call site. Any id the daemon has not seen blocks the push; edits to existing
/// nodes (move, restyle, delete) carry no new ids and go through untouched.
///
/// The local node stays on screen until the next pull replaces it with the
/// daemon's document, so the divergence is bounded and self-healing.
pub(crate) fn push_blocked_by_session(state: &op_editor_core::EditorState) -> bool {
    if state.editor_ui.collab.phase != CollabConnectionPhase::Active {
        return false;
    }
    DAEMON_NODE_IDS.with(|known| {
        let known = known.borrow();
        let Some(known) = known.as_ref() else {
            // No daemon document seen yet in this session; refuse rather than
            // guess, since the pull that would settle it is one tick away.
            return true;
        };
        document_node_ids(state)
            .iter()
            .any(|id| !known.contains(id))
    })
}

/// Node ids in a document, through `op-editor-core`'s own walker so pages and
/// nodes share the one collision domain the allocator uses.
fn document_node_ids(state: &op_editor_core::EditorState) -> HashSet<op_editor_core::NodeId> {
    op_editor_core::collect_document_ids(&state.doc)
}

/// Wire the collaboration relay onto the mounted shell. Called once from mount.
pub(crate) fn start<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let base = crate::daemon_base::daemon_base();
    let inner = inner.clone();
    let tick: Rc<dyn Fn()> = Rc::new(move || {
        drain_pending_action(&inner, &base);
        maybe_pull_state(&inner, &base);
        maybe_push_presence(&inner, &base);
    });
    schedule(tick, IDLE_TICK_MS);
}

/// Re-arming timeout rather than a fixed interval: the cadence has to follow
/// the session phase, and re-reading it at each arming is what makes a session
/// starting between two ticks speed the loop up immediately.
fn schedule(tick: Rc<dyn Fn()>, delay_ms: i32) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once_into_js(move || {
        tick();
        schedule(tick, current_cadence_ms());
    });
    let _ = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(closure.unchecked_ref(), delay_ms);
}

/// Cadence for the next arming, from the phase the last projection installed.
fn current_cadence_ms() -> i32 {
    match APPLIED_SEQ.get() {
        // Before the first projection there is nothing to be responsive to.
        None => IDLE_TICK_MS,
        Some(_) if SESSION_LIVE.get() => SESSION_TICK_MS,
        Some(_) => IDLE_TICK_MS,
    }
}

thread_local! {
    /// Whether the installed projection is anything other than Idle. Read by
    /// the scheduler, which has no access to the host.
    static SESSION_LIVE: Cell<bool> = const { Cell::new(false) };
}

fn drain_pending_action<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>, base: &str) {
    if ACTION_BUSY.get() {
        return;
    }
    let action = ACTION_RETRY
        .with(|slot| slot.borrow_mut().take())
        .or_else(|| {
            inner
                .try_borrow_mut()
                .ok()?
                .host_mut()
                .editor_state_mut()
                .editor_ui
                .collab
                .take_pending_action()
        });
    let Some(action) = action else {
        return;
    };
    let Some(wire) = wire_action(&action) else {
        // No wire form: nothing the daemon could do with it. Dropping is
        // correct — re-queueing would spin forever.
        return;
    };
    let Ok(body) = serde_json::to_string(&wire) else {
        return;
    };
    ACTION_BUSY.set(true);
    let retry_action = action.clone();
    let started = live_sync::post_json_with_status(
        &format!("{base}{}", collab_routes::ACTION),
        &body,
        Rc::new(move |status, response| {
            ACTION_BUSY.set(false);
            if status == 409 && response.contains("collab-busy") {
                // The daemon's single action slot was still full. Hold the
                // action for the next tick instead of dropping what the user
                // asked for.
                ACTION_RETRY.with(|slot| *slot.borrow_mut() = Some(retry_action.clone()));
            }
        }),
    );
    if !started {
        ACTION_BUSY.set(false);
        ACTION_RETRY.with(|slot| *slot.borrow_mut() = Some(action));
    }
}

/// Map the panel's action onto its wire form.
///
/// Exhaustive by construction so a new `CollabUiAction` variant fails to
/// compile here rather than silently becoming a no-op at runtime.
fn wire_action(action: &CollabUiAction) -> Option<CollabActionWire> {
    let key = |k: &op_editor_core::CollabAdmissionRequestKey| k.as_str().to_owned();
    Some(match action {
        CollabUiAction::OpenCreate => CollabActionWire::OpenCreate,
        CollabUiAction::Start => CollabActionWire::Start,
        CollabUiAction::StartLan => CollabActionWire::StartLan,
        CollabUiAction::SetRelayRegion { region } => CollabActionWire::SetRelayRegion {
            region: (*region).into(),
        },
        CollabUiAction::OpenJoin => CollabActionWire::OpenJoin,
        CollabUiAction::BeginDiscovery => CollabActionWire::BeginDiscovery,
        CollabUiAction::JoinDiscovered { discovery_id } => CollabActionWire::JoinDiscovered {
            discovery_id: discovery_id.clone(),
        },
        CollabUiAction::JoinAddress { endpoint } => CollabActionWire::JoinAddress {
            endpoint: endpoint.clone(),
        },
        CollabUiAction::Cancel => CollabActionWire::Cancel,
        CollabUiAction::Retry => CollabActionWire::Retry,
        CollabUiAction::Leave => CollabActionWire::Leave,
        CollabUiAction::DiscardPending => CollabActionWire::DiscardPending,
        CollabUiAction::ReapplyDiscarded => CollabActionWire::ReapplyDiscarded,
        CollabUiAction::SaveAsFork => CollabActionWire::SaveAsFork,
        CollabUiAction::ApproveAdmissionEditor { request_key } => {
            CollabActionWire::ApproveAdmissionEditor {
                request_key: key(request_key),
            }
        }
        CollabUiAction::ApproveAdmissionViewer { request_key } => {
            CollabActionWire::ApproveAdmissionViewer {
                request_key: key(request_key),
            }
        }
        CollabUiAction::RejectAdmission { request_key } => CollabActionWire::RejectAdmission {
            request_key: key(request_key),
        },
        CollabUiAction::ConfirmOwnerIdentity { request_key } => {
            CollabActionWire::ConfirmOwnerIdentity {
                request_key: key(request_key),
            }
        }
        CollabUiAction::RejectOwnerIdentity { request_key } => {
            CollabActionWire::RejectOwnerIdentity {
                request_key: key(request_key),
            }
        }
    })
}

fn maybe_pull_state<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>, base: &str) {
    if STATE_BUSY.get() {
        return;
    }
    let applied = APPLIED_SEQ.get();
    let observed = OBSERVED_SEQ.get();
    // Pull when the projection moved, when nothing has ever been pulled, or
    // continuously while a session runs — presence and admission prompts change
    // faster than the shared version probe reports.
    let due = applied != Some(observed) || applied.is_none() || SESSION_LIVE.get();
    if !due {
        return;
    }
    STATE_BUSY.set(true);
    let inner = inner.clone();
    let started = live_sync::get_with_status(
        &format!("{base}{}", collab_routes::STATE),
        Rc::new(move |status, body| {
            STATE_BUSY.set(false);
            if status != 200 {
                return;
            }
            let Ok(wire) = serde_json::from_str::<CollabStateWire>(&body) else {
                return;
            };
            apply_state(&inner, &wire);
        }),
    );
    if !started {
        STATE_BUSY.set(false);
    }
}

fn apply_state<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>, wire: &CollabStateWire) {
    let Ok(mut context) = inner.try_borrow_mut() else {
        return;
    };
    let now_ms = now_ms();
    let state = context.host_mut().editor_state_mut();
    // Everything lands through the projection's own `set_*` API, which
    // re-runs each sanitising constructor — the wire is data, not state.
    wire.apply_to(&mut state.editor_ui.collab, now_ms);
    APPLIED_SEQ.set(Some(wire.collab_seq));
    SESSION_LIVE.set(state.editor_ui.collab.phase != CollabConnectionPhase::Idle);
    context.host_mut().mark_editor_state_dirty();
    let _ = context.repaint();
}

fn maybe_push_presence<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>, base: &str) {
    if !SESSION_LIVE.get() {
        return;
    }
    let now = now_ms_f64();
    if now - LAST_PRESENCE_MS.get() < PRESENCE_MIN_INTERVAL_MS {
        return;
    }
    let Ok(context) = inner.try_borrow() else {
        return;
    };
    let (width, height) = context.viewport_size();
    let cursor = context.host().last_cursor_doc_point(width, height);
    drop(context);

    if LAST_PRESENCE_POINT.get() == cursor {
        return;
    }
    let wire = CollabLocalPresenceWire {
        cursor: cursor.map(|(x, y)| CollabPointWire { x, y }),
        client_id: None,
    };
    let Ok(body) = serde_json::to_string(&wire) else {
        return;
    };
    LAST_PRESENCE_MS.set(now);
    LAST_PRESENCE_POINT.set(cursor);
    let _ = live_sync::post_json(&format!("{base}{}", collab_routes::PRESENCE), &body, None);
}

fn now_ms_f64() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now())
        .unwrap_or(0.0)
}

fn now_ms() -> u64 {
    now_ms_f64().max(0.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_with(ids: &[&str]) -> op_editor_core::EditorState {
        let children: Vec<serde_json::Value> = ids
            .iter()
            .map(|id| {
                serde_json::json!({
                    "type": "rectangle", "id": id,
                    "x": 0, "y": 0, "width": 4, "height": 4
                })
            })
            .collect();
        let mut state = op_editor_core::EditorState::starter();
        state.doc = serde_json::from_value(serde_json::json!({
            "version": "1.0",
            "children": children,
        }))
        .expect("valid document");
        state
    }

    fn reset_latches() {
        DAEMON_NODE_IDS.with(|ids| *ids.borrow_mut() = None);
    }

    #[test]
    fn an_idle_shell_never_blocks_a_push() {
        reset_latches();
        let state = doc_with(&["n100", "n101"]);
        assert_eq!(
            state.editor_ui.collab.phase,
            CollabConnectionPhase::Idle,
            "precondition"
        );
        assert!(
            !push_blocked_by_session(&state),
            "a shell with no session must sync exactly as it did before"
        );
    }

    #[test]
    fn an_active_session_blocks_a_push_that_invents_a_node_id() {
        reset_latches();
        let mut state = doc_with(&["n100"]);
        note_daemon_document(&state);
        state
            .editor_ui
            .collab
            .set_phase(CollabConnectionPhase::Active);

        // Editing what the daemon already knows is fine.
        assert!(!push_blocked_by_session(&state));

        // Minting a new local id is not: the browser has no owner-assigned
        // namespace, so this id could collide with a peer's.
        let mut grown = doc_with(&["n100", "n101"]);
        grown
            .editor_ui
            .collab
            .set_phase(CollabConnectionPhase::Active);
        assert!(push_blocked_by_session(&grown));
    }

    #[test]
    fn deleting_a_node_during_a_session_is_not_blocked() {
        reset_latches();
        let seed = doc_with(&["n100", "n101"]);
        note_daemon_document(&seed);

        let mut shrunk = doc_with(&["n100"]);
        shrunk
            .editor_ui
            .collab
            .set_phase(CollabConnectionPhase::Active);
        assert!(
            !push_blocked_by_session(&shrunk),
            "removals carry no new ids and must keep syncing"
        );
    }

    #[test]
    fn an_active_session_blocks_until_the_first_daemon_document_arrives() {
        reset_latches();
        let mut state = doc_with(&["n100"]);
        state
            .editor_ui
            .collab
            .set_phase(CollabConnectionPhase::Active);
        assert!(
            push_blocked_by_session(&state),
            "with no daemon document to compare against, refusing is the safe answer"
        );
    }

    #[test]
    fn every_ui_action_has_a_wire_form() {
        let key = op_editor_core::CollabAdmissionRequestKey::new("req-1").expect("valid key");
        for action in [
            CollabUiAction::OpenCreate,
            CollabUiAction::Start,
            CollabUiAction::StartLan,
            CollabUiAction::OpenJoin,
            CollabUiAction::BeginDiscovery,
            CollabUiAction::Cancel,
            CollabUiAction::Retry,
            CollabUiAction::Leave,
            CollabUiAction::DiscardPending,
            CollabUiAction::ReapplyDiscarded,
            CollabUiAction::SaveAsFork,
            CollabUiAction::JoinAddress {
                endpoint: "1.2.3.4:5".into(),
            },
            CollabUiAction::JoinDiscovered {
                discovery_id: "d".into(),
            },
            CollabUiAction::ApproveAdmissionEditor {
                request_key: key.clone(),
            },
            CollabUiAction::RejectOwnerIdentity { request_key: key },
        ] {
            let wire = wire_action(&action).expect("every action maps");
            assert!(serde_json::to_string(&wire).is_ok(), "{action:?}");
        }
    }

    #[test]
    fn admission_keys_survive_the_round_trip_to_the_daemon() {
        let key = op_editor_core::CollabAdmissionRequestKey::new("req-abc_1").expect("valid");
        let wire = wire_action(&CollabUiAction::RejectAdmission {
            request_key: key.clone(),
        })
        .expect("maps");
        assert_eq!(
            wire.clone().into_ui_action().expect("revalidates"),
            CollabUiAction::RejectAdmission { request_key: key }
        );
    }
}
