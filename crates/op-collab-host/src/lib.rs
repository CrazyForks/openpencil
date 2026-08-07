//! Host-agnostic collaboration runtime.
//!
//! Owner / guest actors, the admission queue, the relay bootstrap + locator
//! control plane, and the ticket verifier — everything between the editor
//! state and the wire. The runtime is driven by a synchronous caller (a GUI
//! frame pump, a daemon tick) and bridges to its network workers over bounded
//! channels.
//!
//! Two seams keep it free of any concrete host:
//!
//! * [`CollabHost`] — the editor surface it mutates. `HeadlessCollabHost` is
//!   the daemon/test implementation; GUI hosts implement it over their paint
//!   caches.
//! * [`BlockingExecutor`] — the async → sync bridge, installed once per
//!   process by the embedding host.

mod blocking;
mod host;
mod jwks;
mod runtime;

pub use blocking::{install_blocking_executor, BlockingExecutor};
pub use host::{CollabHost, CollabWakeNotifier, HeadlessCollabHost};
pub use runtime::types::{CollabRuntimeFailure, CollabStatusEvent};
pub use runtime::CollabRuntime;

/// Serialize the tests that rotate the process-global collaboration-avatar
/// generation in `op_editor_ui::collab_avatar_runtime`.
#[cfg(test)]
pub(crate) fn lock_avatar_test_registry() -> std::sync::MutexGuard<'static, ()> {
    static AVATAR_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    AVATAR_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}
