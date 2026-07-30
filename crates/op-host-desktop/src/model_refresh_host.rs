//! Host pump for the picker-open model-catalog refresh.
//!
//! Same request-seam shape as `provider_probe_host`: the shared click
//! flow raises `editor_ui.pending_model_catalog_refresh` when the model
//! picker opens, this pump drains it onto a worker thread, and a later
//! frame lands the result into `chat.discovered_models`. The TTL debounce
//! and the never-clear-on-failure policy live in
//! [`op_host_services::model_catalog_refresh`]; everything here is the
//! desktop arm around them.

use std::time::Instant;

use crate::DesktopApp;

impl DesktopApp {
    /// Land a finished refresh, then start one if the picker asked for it.
    /// Returns true when editor state changed.
    pub(crate) fn drain_model_catalog_refresh(&mut self) -> bool {
        let mut changed = self
            .model_catalog_refresh
            .poll_into(self.host.editor_state_mut());
        if changed {
            self.host.mark_editor_state_dirty();
        }
        if !self
            .host
            .editor_state_mut()
            .editor_ui
            .take_pending_model_catalog_refresh()
        {
            return changed;
        }
        // Only providers whose connect probe actually succeeded — an
        // unconnected (or failed) CLI has nothing to re-discover, and
        // probing it here would resurrect the startup-probe cost on
        // every picker open.
        let connected = self
            .host
            .editor_state()
            .editor_ui
            .agent_settings
            .verified_connected_mask();
        changed |= self.spawn_refresh(connected);
        changed
    }

    #[cfg(not(test))]
    fn spawn_refresh(&mut self, connected: [bool; 6]) -> bool {
        self.model_catalog_refresh
            .request(connected, Instant::now())
    }

    /// Unit tests exercise the seam / mask / TTL plumbing, never the
    /// discovery body — execing the developer's real `codex` (and every
    /// other installed CLI) from a test would be slow and side-effectful.
    /// Same reasoning as the `cfg!(test)` guard on the startup
    /// `ModelProbe` in `app_state.rs`.
    #[cfg(test)]
    fn spawn_refresh(&mut self, connected: [bool; 6]) -> bool {
        self.model_catalog_refresh
            .request_with(connected, Instant::now(), |_| Vec::new())
    }

    /// Whether a refresh worker is in flight — keeps the idle event loop
    /// waking so a landed catalog is drained without a stray input event.
    pub(crate) fn model_catalog_refresh_pending(&self) -> bool {
        self.model_catalog_refresh.is_pending()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::agent_settings::ProviderConnectPhase;
    use op_editor_core::AgentProvider;

    fn connect(app: &mut DesktopApp, provider: AgentProvider) {
        let index = AgentProvider::ALL
            .iter()
            .position(|candidate| *candidate == provider)
            .expect("known provider");
        let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
        settings.connected[index] = true;
        settings.provider_connection[index].phase = ProviderConnectPhase::Connected;
    }

    #[test]
    fn opening_the_picker_requests_a_refresh_for_connected_providers() {
        let mut app = DesktopApp::new(None);
        connect(&mut app, AgentProvider::CodexCli);
        app.host
            .editor_state_mut()
            .editor_ui
            .toggle_chat_model_picker();

        assert!(app.drain_model_catalog_refresh());
        assert!(app.model_catalog_refresh_pending());
        // The seam is consumed once — a repaint before the worker lands
        // must not queue a second probe.
        assert!(
            !app.host
                .editor_state()
                .editor_ui
                .pending_model_catalog_refresh,
            "the request seam is drained by the pump"
        );
    }

    #[test]
    fn opening_the_picker_with_no_connected_provider_starts_nothing() {
        let mut app = DesktopApp::new(None);
        app.host
            .editor_state_mut()
            .editor_ui
            .toggle_chat_model_picker();

        assert!(!app.drain_model_catalog_refresh());
        assert!(!app.model_catalog_refresh_pending());
    }

    #[test]
    fn closing_the_picker_never_requests_a_refresh() {
        let mut app = DesktopApp::new(None);
        connect(&mut app, AgentProvider::CodexCli);
        let ui = &mut app.host.editor_state_mut().editor_ui;
        ui.toggle_chat_model_picker();
        ui.take_pending_model_catalog_refresh();
        ui.toggle_chat_model_picker();

        assert!(!app.drain_model_catalog_refresh());
        assert!(!app.model_catalog_refresh_pending());
    }
}
