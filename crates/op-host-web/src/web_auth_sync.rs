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

/// Wire the auth relay onto the mounted shell. Called once from mount;
/// the interval runs for the page lifetime.
pub(crate) fn start<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let base = crate::daemon_base::daemon_base();
    fetch_status(inner, &base);

    let busy = Rc::new(Cell::new(false));
    // The verification popup must open exactly once per flow.
    let browser_opened = Rc::new(Cell::new(false));
    let ticks = Rc::new(Cell::new(0u32));
    let inner = inner.clone();
    let tick: Rc<dyn Fn()> = Rc::new(move || {
        drain_actions(&inner, &base, &browser_opened);
        maybe_poll_login(&inner, &base, &busy, &browser_opened);
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
    browser_opened: &Rc<Cell<bool>>,
) {
    let actions = inner.borrow_mut().host_mut().take_pending_auth_actions();
    for action in actions {
        let path = match action {
            PendingAuthAction::BeginLogin => {
                browser_opened.set(false);
                "/api/auth/login/begin"
            }
            PendingAuthAction::CancelLogin => "/api/auth/login/cancel",
            PendingAuthAction::SignOut => "/api/auth/logout",
        };
        let inner_cb = inner.clone();
        let ok = live_sync::post_json(
            &format!("{base}{path}"),
            "{}",
            Some(Rc::new(move |body: String| {
                // Begin can be refused (stub daemon build) — surface it.
                if body.contains(r#""ok":true"#) {
                    return;
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
        if !ok && action == PendingAuthAction::BeginLogin {
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
    busy: &Rc<Cell<bool>>,
    browser_opened: &Rc<Cell<bool>>,
) {
    {
        let b = inner.borrow();
        let ui = &b.host().editor_state().editor_ui;
        let flow_active = ui.login_modal_open && ui.login_modal_status.is_some();
        if !flow_active || busy.get() {
            return;
        }
    }
    busy.set(true);
    let inner = inner.clone();
    let busy_cb = busy.clone();
    let browser_opened = browser_opened.clone();
    let ok = live_sync::get(
        &format!("{base}/api/auth/login/status"),
        Rc::new(move |body: String| {
            busy_cb.set(false);
            let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) else {
                return;
            };
            apply_login_status(&inner, &parsed, &browser_opened);
        }),
    );
    if !ok {
        busy.set(false);
    }
}

fn apply_login_status<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    parsed: &serde_json::Value,
    browser_opened: &Rc<Cell<bool>>,
) {
    let mut b = inner.borrow_mut();
    let ui = &mut b.host_mut().editor_state_mut().editor_ui;
    if !ui.login_modal_open {
        return; // dismissed while the request was in flight
    }
    let previous = ui.login_modal_status;
    match parsed["state"].as_str().unwrap_or_default() {
        "starting" => ui.login_modal_status = Some(LoginFlowStatus::WaitingBrowser),
        "waiting_approval" => {
            ui.login_modal_status = Some(LoginFlowStatus::WaitingApproval);
            if !browser_opened.get() {
                if let Some(url) = parsed["verification_uri"].as_str() {
                    if !url.is_empty() {
                        browser_opened.set(true);
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
        // "idle" / "canceled": another tab or the daemon finished the
        // flow out from under us — reset the note, keep the modal.
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
