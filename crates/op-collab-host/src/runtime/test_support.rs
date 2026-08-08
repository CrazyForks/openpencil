//! Controlled collaboration fixtures for downstream test suites.
//!
//! `op-host-services` needs to drive the REAL begin → install → reject/fail
//! paths to prove what its ingest does with a session's verdict. Without a
//! fixture its tests can only assert against a hand-built `LocalEditOutcome`,
//! which passes whether or not the wiring behind it is correct.
//!
//! Standing up an activated owner session needs `crate::runtime`'s private
//! actor and channel internals, so this module lives inside `runtime` and
//! re-exports exactly two capabilities:
//!
//! 1. [`owner_session`] — an activated owner runtime, ready for
//!    `begin_local_edit`.
//! 2. [`owner_session_with_saturated_command_lane`] — the same, with the
//!    outbound command lane already full, so the next commit cannot be
//!    delivered and the runtime falls back to standalone.
//!
//! ## Not a production surface
//!
//! Gated behind `#[cfg(any(test, feature = "test-support"))]`, and the feature
//! is turned on only by a `dev-dependencies` entry. A normal build of this
//! crate compiles none of it, so it cannot reach the shipped API.

use std::sync::mpsc::Receiver;

use op_collab::{ConnectionKey, Epoch, Role, SessionId, VerifiedAuthMetadata};

use super::actor::{set_owner_ui, EditorActor, OwnerActor};
use super::network::owner_command_channel_with_capacity_for_test;
use super::types::OwnerNetworkCommand;
use super::CollabRuntime;
use crate::host::HeadlessCollabHost;

/// Session id every fixture runs under.
const FIXTURE_SESSION: &str = "collab-host-test-support";

/// Enough room for one command, which the saturating constructor then uses.
const FIXTURE_COMMAND_CAPACITY: usize = 1;

/// An activated owner session.
///
/// The caller normally moves `runtime` into whatever state machine it is
/// testing; `host` is the editor the session was activated against and is kept
/// alive because the actor's projection points at it.
pub struct OwnerFixture {
    /// The runtime, carrying an activated `OwnerActor` with one admitted peer.
    pub runtime: CollabRuntime,
    /// The headless editor the session was activated over.
    pub host: HeadlessCollabHost,
    /// The admitted peer, for callers that need to name it.
    pub peer: ConnectionKey,
    /// Held, not dropped: a dropped receiver makes every send fail outright,
    /// which is a *different* failure from "the lane is full". Nothing drains
    /// it, so the channel capacity is the whole budget.
    _commands: Receiver<OwnerNetworkCommand>,
}

/// Build an owner runtime with one admitted editor peer, ready for
/// `begin_local_edit`.
///
/// The session's diff runs between the document the caller's host held when
/// the capture opened and the document it holds when the capture closes — not
/// against this fixture's host — so a caller may install the runtime over its
/// own editor state.
pub fn owner_session() -> OwnerFixture {
    let mut host = HeadlessCollabHost::new();
    let mut owner = OwnerActor::new(
        SessionId::from(FIXTURE_SESSION),
        Epoch(1),
        fixture_auth(0),
        &mut host,
    )
    .expect("owner actor");
    let peer = ConnectionKey::new(2).expect("non-zero connection");
    let grant = owner
        .grant_new_peer(fixture_auth(1), Role::Editor)
        .expect("peer grant");
    owner
        .session
        .activate_peer(peer, grant, &host)
        .expect("activate peer");
    owner.connections.insert(peer);
    set_owner_ui(&mut host, &owner);

    let (network, commands) =
        owner_command_channel_with_capacity_for_test(FIXTURE_COMMAND_CAPACITY);
    let mut runtime = CollabRuntime::new();
    runtime.network = Some(network);
    runtime.actor = Some(EditorActor::Owner(Box::new(owner)));
    OwnerFixture {
        runtime,
        host,
        peer,
        _commands: commands,
    }
}

/// [`owner_session`] with the outbound command lane already full.
///
/// The next commit cannot be handed to the network worker, so the runtime
/// retires the session and `finish_local_edit` reports
/// `Failed { document_rolled_back: false }` — the standalone fallback, which
/// deliberately KEEPS the edit because the user's work is still theirs even
/// though the session is gone. A caller that undoes side effects on failure
/// must be able to reproduce this exact case.
pub fn owner_session_with_saturated_command_lane() -> OwnerFixture {
    let fixture = owner_session();
    fixture
        .runtime
        .send_owner(OwnerNetworkCommand::Close {
            connection: ConnectionKey::new(99).expect("non-zero connection"),
        })
        .expect("the first send fills the lane");
    fixture
}

fn fixture_auth(index: usize) -> VerifiedAuthMetadata {
    VerifiedAuthMetadata {
        issuer: "https://issuer.example".into(),
        subject: format!("subject-{index}"),
        device_id: format!("device-{index}"),
        proof_binding: format!("binding-{index}"),
        expires_at_unix_ms: 10_000,
        display_name: None,
        avatar_url: None,
    }
}
