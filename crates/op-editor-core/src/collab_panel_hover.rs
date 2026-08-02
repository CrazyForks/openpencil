//! Secret-free pointer identities for the collaboration popover.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabPanelHover {
    Close,
    OpenSignIn,
    CopyInvite,
    CopyShareEndpoint,
    OpenCreate,
    Start,
    StartLan,
    OpenJoin,
    BeginDiscovery,
    Connect,
    Cancel,
    JoinAddress,
    ClearJoinAddress,
    Discovered(usize),
    Retry,
    Leave,
    DiscardPending,
    ReapplyDiscarded,
    SaveAsFork,
    ApproveAdmissionEditor,
    ApproveAdmissionViewer,
    RejectAdmission,
    ConfirmOwnerIdentity,
    RejectOwnerIdentity,
}

impl crate::CollabUiState {
    /// Update host availability and invalidate feedback whose screen may have
    /// changed under a stationary cursor.
    pub fn set_availability(&mut self, availability: crate::CollabAvailability) {
        if self.availability != availability {
            self.panel.hover = None;
            self.availability = availability;
        }
    }

    /// Update phase and invalidate feedback whose action may have moved under
    /// a stationary cursor.
    pub fn set_phase(&mut self, phase: crate::CollabConnectionPhase) {
        if self.phase != phase {
            self.panel.hover = None;
            self.phase = phase;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CollabPanelHover;
    use crate::{CollabAvailability, CollabNoticeKind, CollabUiState};

    #[test]
    fn screen_and_notice_changes_clear_stale_panel_hover() {
        let mut state = CollabUiState::default();
        state.panel.hover = Some(CollabPanelHover::Start);
        state.set_notice(CollabNoticeKind::OwnerLeft, 7);
        assert_eq!(state.panel.hover, None);

        state.panel.hover = Some(CollabPanelHover::OpenSignIn);
        state.set_availability(CollabAvailability::SignInRequired);
        assert_eq!(state.panel.hover, None);
    }
}
