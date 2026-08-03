//! GUI-owned local-edit gestures: capture begin/finish and the replay of a
//! conflict-discarded edit. Split off the `collab_runtime` spine at the
//! 800-line cap; pure code motion.

use op_editor_core::{CollabNoticeKind, CollabRejectUiCode};
use op_host_native::WidgetHostNative;

use super::actor::EditorActor;
use super::types::CollabRuntimeError;
use super::DesktopCollabRuntime;

impl DesktopCollabRuntime {
    /// Whether a GUI edit capture is open right now.
    ///
    /// A pointer gesture is one transaction: press opens the capture, release
    /// closes it. Actions queued by that same press (approving an admission,
    /// for example) must not be drained mid-gesture, because the session's
    /// document actor refuses to act while a capture is in flight.
    pub(crate) const fn local_edit_in_flight(&self) -> bool {
        self.transaction_active
    }

    /// Start one GUI-owned edit capture; `false` means busy or no live session is bound.
    pub(crate) fn begin_local_edit(&mut self, host: &mut WidgetHostNative) -> bool {
        if self.transaction_active {
            return false;
        }
        let started = match self.actor.as_mut() {
            Some(EditorActor::Owner(actor)) => actor.session.begin_local_edit(host).is_ok(),
            Some(EditorActor::Guest(actor)) => actor.session.begin_local_edit(host).is_ok(),
            None => return false,
        };
        match started {
            true => {
                self.transaction_active = true;
                true
            }
            false => {
                self.set_notice(host, CollabNoticeKind::Reject(CollabRejectUiCode::ReadOnly));
                false
            }
        }
    }

    pub(crate) fn finish_local_edit(&mut self, host: &mut WidgetHostNative) -> bool {
        if !std::mem::take(&mut self.transaction_active) {
            return false;
        }
        let Some(mut actor) = self.actor.take() else {
            return false;
        };
        let result = match &mut actor {
            EditorActor::Owner(owner) => match owner.session.finish_local_edit(host) {
                Ok(output) => self.route_owner_output(owner, output, host),
                Err(_) => Err(CollabRuntimeError::invalid_session()),
            },
            EditorActor::Guest(guest) => match guest.session.finish_local_edit(host) {
                Ok(output) => self.route_guest_output(guest, output, host),
                Err(_) => Err(CollabRuntimeError::invalid_session()),
            },
        };
        self.actor = Some(actor);
        match result {
            Ok(()) => true,
            Err(error) => {
                // A local editor/core failure can occur after the document
                // changed or the sequencer prepared a commit. Continuing to
                // advertise Active would silently fork owner and guests.
                self.fail_network(host, error.failure);
                false
            }
        }
    }

    /// Resubmit the stashed conflict-discarded property edit as a brand-new
    /// local edit over the current authoritative document.
    ///
    /// The replay deliberately reasserts the dropped desired values over
    /// whatever the fields hold now; it goes through the normal local-edit
    /// pipeline, so a fresh conflict simply produces a fresh stash.
    pub(super) fn reapply_discarded(&mut self, host: &mut WidgetHostNative) {
        if self.discarded_property_edit.is_none() {
            return;
        }
        if !self.begin_local_edit(host) {
            // Busy or read-only; begin_local_edit already raised the notice.
            // The stash is kept so the user can retry once the lane is free.
            return;
        }
        let changes = self
            .discarded_property_edit
            .take()
            .expect("stash presence was checked above");
        host.editor_state_mut().editor_ui.collab.discarded_edit = None;
        match op_collab::reapply_property_changes(&host.editor_state().doc, &changes) {
            Ok(desired) => {
                // Mutating the document inside the capture mirrors a GUI
                // gesture: end_local_edit diffs it and bumps the revision.
                host.editor_state_mut().doc = desired;
            }
            Err(_) => {
                // The target node no longer exists; finish as a no-op.
                self.set_notice(host, CollabNoticeKind::Reject(CollabRejectUiCode::Conflict));
            }
        }
        self.finish_local_edit(host);
    }
}
