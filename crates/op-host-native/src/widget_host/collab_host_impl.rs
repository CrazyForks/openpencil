//! `CollabHost` impl for the GUI host.
//!
//! The trait comes from `op-collab-host` and the type from this crate, so the
//! orphan rule forces the impl into one of the two. It lives here rather than
//! in `op-collab-host` because that crate must not depend on any host.
//!
//! Every method forwards to the inherent one in `scene_state.rs`; the
//! collaboration runtime's behaviour is unchanged by the seam.

use op_collab_host::CollabHost;

use super::WidgetHostNative;

impl CollabHost for WidgetHostNative {
    fn mark_editor_state_dirty(&mut self) {
        WidgetHostNative::mark_editor_state_dirty(self);
    }

    fn enable_collaboration_ids(
        &mut self,
        namespace: op_editor_core::PeerNamespace,
    ) -> Result<(), op_editor_core::IdAllocError> {
        WidgetHostNative::enable_collaboration_ids(self, namespace)
    }

    fn disable_collaboration_ids(&mut self) {
        WidgetHostNative::disable_collaboration_ids(self);
    }
}
