//! Drive the daemon's device-login proxy from the web shell.
//!
//! The wasm bundle ships no auth code: presses queue
//! [`PendingAuthAction`]s on the host, and this module's poll tick turns
//! them into `/api/auth/*` calls. While a login flow runs it polls
//! `GET /api/auth/login/status`, opens the verification page in a popup
//! exactly once, and folds progress into the same
//! `login_modal_status` / `account` fields the desktop host uses — the
//! login modal renders identically on both hosts.
//!
//! On startup one `GET /api/auth/status` seeds `account_ui_available`
//! and (when the daemon restored a shared session) the signed-in state.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use op_editor_core::{AccountState, LoginFlowError, LoginFlowStatus};

use crate::live_sync;
use crate::repaint_ctx::RepaintContext;
use crate::widget_host::PendingAuthAction;

/// Poll cadence while idle; each tick is a queue drain (usually empty)
/// plus, only during a login flow, one status GET.
const AUTH_POLL_INTERVAL_MS: i32 = 600;
/// Steady-state session health check — every N ticks (~30 s) re-fetch
/// `/api/auth/status` so a session revoked or expired daemon-side (or a
/// sign-in/out from the desktop GUI or another tab) reaches this shell
/// without a reload. Network failures change nothing (the callback
/// simply never fires), so an offline blip can't sign the user out.
const STATUS_REFRESH_TICKS: u32 = 50;

/// Consecutive `idle` login-status answers tolerated before the flow is
/// declared dead. A begin POST races the first status poll (the daemon
/// may not have created the flow yet), so a single `idle` MUST be
/// treated as transient — resetting on it immediately was the original
/// "first click opens only a blank window" bug.
const MAX_IDLE_STREAK: u32 = 5;

/// Shared latches for the login flow's async choreography.
#[derive(Clone)]
struct FlowCells {
    /// One status request in flight at a time.
    busy: Rc<Cell<bool>>,
    /// The verification popup must be navigated exactly once per flow.
    browser_opened: Rc<Cell<bool>>,
    /// A begin POST is in flight — suppress status polls until it lands
    /// so they can't observe the daemon's pre-begin `idle`.
    begin_inflight: Rc<Cell<bool>>,
    /// Consecutive transient `idle` answers observed.
    idle_streak: Rc<Cell<u32>>,
}

impl FlowCells {
    fn new() -> Self {
        Self {
            busy: Rc::new(Cell::new(false)),
            browser_opened: Rc::new(Cell::new(false)),
            begin_inflight: Rc::new(Cell::new(false)),
            idle_streak: Rc::new(Cell::new(0)),
        }
    }
}

/// Wire the auth relay onto the mounted shell. Called once from mount;
/// the interval runs for the page lifetime.
pub(crate) fn start<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let base = crate::daemon_base::daemon_base();
    fetch_status(inner, &base);

    let cells = FlowCells::new();
    let ticks = Rc::new(Cell::new(0u32));
    let inner = inner.clone();
    let tick: Rc<dyn Fn()> = Rc::new(move || {
        drain_actions(&inner, &base, &cells);
        maybe_poll_login(&inner, &base, &cells);
        let count = ticks.get() + 1;
        if count >= STATUS_REFRESH_TICKS {
            ticks.set(0);
            fetch_status(&inner, &base);
        } else {
            ticks.set(count);
        }
    });
    let _ = live_sync::start_interval(AUTH_POLL_INTERVAL_MS, tick);
}

fn fetch_status<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>, base: &str) {
    let inner = inner.clone();
    let _ = live_sync::get(
        &format!("{base}/api/auth/status"),
        Rc::new(move |body: String| {
            let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) else {
                return;
            };
            let mut b = inner.borrow_mut();
            let ui = &mut b.host_mut().editor_state_mut().editor_ui;
            let available = parsed["available"].as_bool().unwrap_or(false);
            let account = if parsed["signed_in"].as_bool().unwrap_or(false) {
                let display_name = parsed["display_name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                AccountState::SignedIn {
                    handle: parsed["primary_email"]
                        .as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| display_name.clone()),
                    display_name,
                }
            } else {
                AccountState::Anonymous
            };
            // Don't clobber mid-flow UI: while a login flow runs the
            // login-status poll owns the account fields.
            let flow_active = ui.login_modal_open && ui.login_modal_status.is_some();
            let changed =
                ui.account_ui_available != available || (!flow_active && ui.account != account);
            if changed {
                ui.account_ui_available = available;
                if !flow_active {
                    ui.account = account;
                }
                b.host_mut().mark_editor_state_dirty();
                let _ = b.repaint();
            }
        }),
    );
}

fn drain_actions<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    base: &str,
    cells: &FlowCells,
) {
    let actions = inner.borrow_mut().host_mut().take_pending_auth_actions();
    for action in actions {
        let path = match action {
            PendingAuthAction::BeginLogin => {
                cells.browser_opened.set(false);
                cells.idle_streak.set(0);
                cells.begin_inflight.set(true);
                "/api/auth/login/begin"
            }
            PendingAuthAction::CancelLogin => "/api/auth/login/cancel",
            PendingAuthAction::SignOut => "/api/auth/logout",
        };
        let inner_cb = inner.clone();
        let begin_inflight = cells.begin_inflight.clone();
        let is_begin = action == PendingAuthAction::BeginLogin;
        let ok = live_sync::post_json(
            &format!("{base}{path}"),
            "{}",
            Some(Rc::new(move |body: String| {
                if is_begin {
                    begin_inflight.set(false);
                }
                // Begin can be refused (stub daemon build) — surface it.
                if body.contains(r#""ok":true"#) {
                    return;
                }
                if is_begin {
                    close_login_popup_placeholder();
                }
                let mut b = inner_cb.borrow_mut();
                let ui = &mut b.host_mut().editor_state_mut().editor_ui;
                if ui.login_modal_open {
                    ui.login_modal_status =
                        Some(LoginFlowStatus::Failed(LoginFlowError::Unavailable));
                    b.host_mut().mark_editor_state_dirty();
                    let _ = b.repaint();
                }
            })),
        );
        if action == PendingAuthAction::CancelLogin {
            close_login_popup_placeholder();
        }
        if !ok && is_begin {
            cells.begin_inflight.set(false);
            close_login_popup_placeholder();
            let mut b = inner.borrow_mut();
            let ui = &mut b.host_mut().editor_state_mut().editor_ui;
            ui.login_modal_status = Some(LoginFlowStatus::Failed(LoginFlowError::Unavailable));
            b.host_mut().mark_editor_state_dirty();
            let _ = b.repaint();
        }
    }
}

fn maybe_poll_login<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    base: &str,
    cells: &FlowCells,
) {
    {
        let b = inner.borrow();
        let ui = &b.host().editor_state().editor_ui;
        let flow_active = ui.login_modal_open && ui.login_modal_status.is_some();
        // While the begin POST is in flight the daemon may not have the
        // flow yet — a poll now would observe a misleading `idle`.
        if !flow_active || cells.busy.get() || cells.begin_inflight.get() {
            return;
        }
    }
    cells.busy.set(true);
    let inner = inner.clone();
    let cells_cb = cells.clone();
    let ok = live_sync::get(
        &format!("{base}/api/auth/login/status"),
        Rc::new(move |body: String| {
            cells_cb.busy.set(false);
            let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) else {
                return;
            };
            apply_login_status(&inner, &parsed, &cells_cb);
        }),
    );
    if !ok {
        cells.busy.set(false);
    }
}

fn apply_login_status<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    parsed: &serde_json::Value,
    cells: &FlowCells,
) {
    let mut b = inner.borrow_mut();
    let ui = &mut b.host_mut().editor_state_mut().editor_ui;
    if !ui.login_modal_open {
        return; // dismissed while the request was in flight
    }
    let previous = ui.login_modal_status;
    if parsed["state"].as_str() != Some("idle") {
        cells.idle_streak.set(0);
    }
    match parsed["state"].as_str().unwrap_or_default() {
        "starting" => ui.login_modal_status = Some(LoginFlowStatus::WaitingBrowser),
        "waiting_approval" => {
            ui.login_modal_status = Some(LoginFlowStatus::WaitingApproval);
            if !cells.browser_opened.get() {
                if let Some(url) = parsed["verification_uri"].as_str() {
                    if !url.is_empty() {
                        cells.browser_opened.set(true);
                        navigate_login_popup(url);
                    }
                }
            }
        }
        "exchanging" => ui.login_modal_status = Some(LoginFlowStatus::Exchanging),
        "signed_in" => {
            let display_name = parsed["display_name"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            ui.account = AccountState::SignedIn {
                handle: parsed["primary_email"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| display_name.clone()),
                display_name,
            };
            ui.login_modal_status = None;
            ui.login_modal_open = false;
            ui.login_modal_hover = None;
        }
        "error" => {
            close_login_popup_placeholder();
            ui.login_modal_status = Some(LoginFlowStatus::Failed(
                match parsed["code"].as_str().unwrap_or_default() {
                    "denied" => LoginFlowError::Denied,
                    "expired" => LoginFlowError::Expired,
                    _ => LoginFlowError::Unavailable,
                },
            ));
        }
        "idle" => {
            // Transient while the just-begun flow races our poll; only a
            // sustained streak means the flow is really gone (daemon
            // restarted, or another tab finished it).
            let streak = cells.idle_streak.get() + 1;
            cells.idle_streak.set(streak);
            if streak >= MAX_IDLE_STREAK {
                close_login_popup_placeholder();
                ui.login_modal_status = None;
            }
        }
        // "canceled": another tab or the daemon finished the flow out
        // from under us — reset the note, keep the modal.
        _ => {
            close_login_popup_placeholder();
            ui.login_modal_status = None;
        }
    }
    if ui.login_modal_status != previous || !ui.login_modal_open {
        b.host_mut().mark_editor_state_dirty();
        let _ = b.repaint();
    }
}

thread_local! {
    /// Placeholder popup opened synchronously inside the SignIn click
    /// (user-activation context — the only moment `window.open` is
    /// reliably allowed). The poll callback later navigates it to the
    /// verification page; opening from the async callback instead gets
    /// popup-blocked, which is exactly the "have to click sign-in
    /// twice" failure this design removes.
    static PENDING_POPUP: RefCell<Option<web_sys::Window>> = const { RefCell::new(None) };
}

/// Open the placeholder popup. Must be called synchronously from a
/// pointer-event handler (the web sign-in press dispatcher).
pub(crate) fn open_login_popup_placeholder() {
    let opened = web_sys::window()
        .and_then(|window| {
            window
                .open_with_url_and_target("about:blank", "_blank")
                .ok()
        })
        .flatten();
    PENDING_POPUP.with(|slot| *slot.borrow_mut() = opened);
}

/// Close a still-pending placeholder (flow canceled or failed before
/// the verification page was known).
pub(crate) fn close_login_popup_placeholder() {
    PENDING_POPUP.with(|slot| {
        if let Some(popup) = slot.borrow_mut().take() {
            let _ = popup.close();
        }
    });
}

/// Point the placeholder at the verification page; when the placeholder
/// is missing (blocked, or an older flow) fall back to a direct open —
/// it may be blocked outside a gesture, but the approval also works
/// from any logged-in browser tab, so the flow still completes.
fn navigate_login_popup(url: &str) {
    let pending = PENDING_POPUP.with(|slot| slot.borrow_mut().take());
    match pending {
        Some(popup) => {
            let _ = popup.location().set_href(url);
        }
        None => {
            if let Some(window) = web_sys::window() {
                let _ = window.open_with_url_and_target(url, "_blank");
            }
        }
    }
}
