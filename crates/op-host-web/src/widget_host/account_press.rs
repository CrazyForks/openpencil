//! Sign-in modal + signed-in account-dropdown press dispatchers (web).
//!
//! Mirrors the native `account_press.rs`, with one structural change:
//! the web host cannot call the auth client directly (the flow runs in
//! the serving daemon), so presses only queue [`PendingAuthAction`]s and
//! flip UI state; `web_auth_sync` drains the queue on its poll tick.

use super::{PendingAuthAction, WidgetHost};
use op_editor_core::{AccountMenuRow, LoginFlowStatus};
use op_editor_ui::widgets::account_menu::AccountMenu;
use op_editor_ui::widgets::login_modal::{LoginModal, LoginModalHit};
use op_editor_ui::widgets::top_bar::TopBar;
use op_editor_ui::widgets::TOP_BAR_HEIGHT;
use op_editor_ui::{Point2D, Rect};

impl WidgetHost {
    /// Sign-in-modal press dispatcher.
    pub(in crate::widget_host) fn dispatch_login_modal_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        let modal = LoginModal::for_editor(&self.editor_state);
        let panel_rect = modal.rect(viewport_w, viewport_h);
        let hit = modal.hit_test(panel_rect, Point2D::new(x, y));
        self.editor_state.editor_ui.pressed_button =
            op_editor_ui::widgets::editor_state_ext::login_modal_button(hit)
                .map(op_editor_core::ButtonPressTarget::LoginModal);
        match hit {
            LoginModalHit::Close => {
                self.cancel_login_flow();
                self.editor_state.editor_ui.login_modal_open = false;
                self.editor_state.editor_ui.login_modal_hover = None;
                self.editor_state.editor_ui.login_modal_stub_hint_shown = false;
            }
            LoginModalHit::Outside => {
                self.blur_text_inputs_on_blank_press();
                self.cancel_login_flow();
                self.editor_state.editor_ui.login_modal_open = false;
                self.editor_state.editor_ui.login_modal_hover = None;
                self.editor_state.editor_ui.login_modal_stub_hint_shown = false;
            }
            LoginModalHit::SignIn => {
                // A flow is already running — don't spawn another
                // loading popup or re-begin; the poll owns the modal.
                if matches!(
                    self.editor_state.editor_ui.login_modal_status,
                    Some(
                        LoginFlowStatus::WaitingBrowser
                            | LoginFlowStatus::WaitingApproval
                            | LoginFlowStatus::Exchanging
                    )
                ) {
                    self.mark_dirty();
                    return;
                }
                // Open the loading popup AND fire the begin request NOW,
                // inside the click's user-activation window — an async
                // `window.open` later would be popup-blocked, and going
                // through the poll tick would add up to two poll cycles
                // of latency before the sso page appears.
                crate::web_auth_sync::begin_login_now();
                self.editor_state.editor_ui.login_modal_status =
                    Some(LoginFlowStatus::WaitingBrowser);
                self.editor_state.editor_ui.login_modal_stub_hint_shown = false;
            }
            LoginModalHit::Inside => {
                self.blur_text_inputs_on_blank_press();
            }
        }
        self.mark_dirty();
    }

    /// Signed-in account-dropdown press dispatcher.
    pub(in crate::widget_host) fn dispatch_account_menu_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        _viewport_h: f32,
    ) {
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_w, TOP_BAR_HEIGHT),
        };
        let top_bar = TopBar::for_editor_ui(&self.editor_state.editor_ui);
        let anchor = top_bar.account_button_rect(top_bar_rect);
        let Some(menu) = AccountMenu::for_editor_ui(&self.editor_state.editor_ui) else {
            self.editor_state.editor_ui.account_menu_open = false;
            return;
        };
        let menu_rect = menu.rect_at(anchor);
        let point = Point2D::new(x, y);
        match menu.hit_test(menu_rect, point) {
            Some(AccountMenuRow::Settings) => {
                self.close_account_menu();
                self.editor_state.editor_ui.agent_settings_open = true;
                self.editor_state.editor_ui.agent_settings.tab =
                    op_editor_core::agent_settings::AgentSettingsTab::Account;
                self.editor_state.chat.blur_input(self.now_ms);
            }
            Some(AccountMenuRow::SignOut) => {
                self.close_account_menu();
                self.editor_state.editor_ui.account = op_editor_core::AccountState::Anonymous;
                self.pending_auth_actions.push(PendingAuthAction::SignOut);
            }
            None => {
                if !(menu_rect).contains(point) {
                    self.blur_text_inputs_on_blank_press();
                    self.close_account_menu();
                }
            }
        }
        self.mark_dirty();
    }

    /// Queue a cancel and clear the local progress note.
    pub(in crate::widget_host) fn cancel_login_flow(&mut self) {
        if self.editor_state.editor_ui.login_modal_status.is_some() {
            self.pending_auth_actions
                .push(PendingAuthAction::CancelLogin);
        }
        self.editor_state.editor_ui.login_modal_status = None;
    }

    fn close_account_menu(&mut self) {
        self.editor_state.editor_ui.account_menu_open = false;
        self.editor_state.editor_ui.account_menu_hover = None;
    }
}
