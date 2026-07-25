//! Browser device-login flow driver: bridges `op-auth-bridge` status
//! polls into editor UI state. The desktop event loop calls
//! [`WidgetHostNative::poll_auth`] from its background-drain pass and
//! opens URLs drained via [`WidgetHostNative::take_pending_browser_url`]
//! — this module never spawns processes itself.

use super::WidgetHostNative;
use op_auth_bridge::AuthStatus;
use op_editor_core::{AccountState, LoginFlowError, LoginFlowStatus};

impl WidgetHostNative {
    /// Start the browser device-login flow (called from the sign-in
    /// modal's primary button when a real auth backend is linked).
    pub(in crate::widget_host) fn begin_browser_login(&mut self) {
        if self.auth_login_handle.is_some() {
            return; // one flow at a time; the running one owns the modal
        }
        let handle = op_auth_bridge::login_begin();
        if handle == 0 {
            self.editor_state.editor_ui.login_modal_status =
                Some(LoginFlowStatus::Failed(LoginFlowError::Unavailable));
            return;
        }
        self.auth_login_handle = Some(handle);
        self.auth_browser_opened = false;
        self.editor_state.editor_ui.login_modal_status = Some(LoginFlowStatus::WaitingBrowser);
        self.editor_state.editor_ui.login_modal_stub_hint_shown = false;
    }

    /// Abort an in-flight login (modal closed / outside click). The
    /// bridge flow settles into Canceled on its own thread; UI state
    /// resets immediately.
    pub fn cancel_auth_login(&mut self) {
        if let Some(handle) = self.auth_login_handle.take() {
            op_auth_bridge::cancel(handle);
        }
        self.auth_browser_opened = false;
        self.auth_pending_browser_url = None;
        self.editor_state.editor_ui.login_modal_status = None;
    }

    /// Whether a login flow is running — drives the desktop event loop's
    /// periodic wakeups so status changes land while the app idles.
    pub fn auth_flow_active(&self) -> bool {
        self.auth_login_handle.is_some()
    }

    /// The verification URL the host should open in the system browser,
    /// at most once per flow.
    pub fn take_pending_browser_url(&mut self) -> Option<String> {
        self.auth_pending_browser_url.take()
    }

    /// Poll the in-flight login flow (and the restored session) and fold
    /// status changes into editor UI state. Returns whether anything
    /// visible changed.
    pub fn poll_auth(&mut self) -> bool {
        let Some(handle) = self.auth_login_handle else {
            return self.mirror_session_drop();
        };
        let ui = &mut self.editor_state.editor_ui;
        let previous = ui.login_modal_status;
        match op_auth_bridge::poll(handle) {
            AuthStatus::Idle | AuthStatus::Starting => {
                ui.login_modal_status = Some(LoginFlowStatus::WaitingBrowser);
            }
            AuthStatus::WaitingApproval { verification_uri } => {
                ui.login_modal_status = Some(LoginFlowStatus::WaitingApproval);
                if !self.auth_browser_opened && !verification_uri.is_empty() {
                    self.auth_browser_opened = true;
                    self.auth_pending_browser_url = Some(verification_uri);
                }
            }
            AuthStatus::Exchanging => {
                ui.login_modal_status = Some(LoginFlowStatus::Exchanging);
            }
            AuthStatus::SignedIn {
                display_name,
                primary_email,
                ..
            } => {
                ui.account = AccountState::SignedIn {
                    handle: primary_email.unwrap_or_else(|| display_name.clone()),
                    display_name,
                };
                ui.login_modal_status = None;
                ui.login_modal_open = false;
                ui.login_modal_hover = None;
                self.auth_login_handle = None;
                self.auth_browser_opened = false;
                self.mark_dirty();
                return true;
            }
            AuthStatus::Error { code } => {
                ui.login_modal_status = Some(LoginFlowStatus::Failed(match code.as_str() {
                    "denied" => LoginFlowError::Denied,
                    "expired" => LoginFlowError::Expired,
                    _ => LoginFlowError::Unavailable,
                }));
                self.auth_login_handle = None;
                self.auth_browser_opened = false;
            }
            AuthStatus::Canceled => {
                ui.login_modal_status = None;
                self.auth_login_handle = None;
                self.auth_browser_opened = false;
            }
        }
        let changed = self.editor_state.editor_ui.login_modal_status != previous
            || self.auth_pending_browser_url.is_some();
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Mirror a session the library dropped after the optimistic startup
    /// restore (the background revalidation hit a 401/403) back into the
    /// UI — otherwise the avatar would show a signed-in ghost until the
    /// next launch. Cheap: one in-process status snapshot per drain pass.
    fn mirror_session_drop(&mut self) -> bool {
        if !self.editor_state.editor_ui.account.is_signed_in() || !op_auth_bridge::available() {
            return false;
        }
        if matches!(
            op_auth_bridge::poll(op_auth_bridge::SESSION_HANDLE),
            AuthStatus::Idle
        ) {
            self.editor_state.editor_ui.account = AccountState::Anonymous;
            self.mark_dirty();
            return true;
        }
        false
    }
}
