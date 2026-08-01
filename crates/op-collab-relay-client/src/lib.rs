//! Bounded TCP-to-WebSocket bridge for OpenPencil collaboration.
//!
//! The relay sees only bytes produced by the existing inner Noise/TCP
//! transport. This crate does not parse, decrypt, log, or reinterpret them.

mod auth;
mod bridge;
mod endpoint;
mod error;
mod limits;
mod reauth_budget;
mod session;

pub use auth::{
    ChallengeBoundRelayAuthenticator, PinnedRelayX25519Keys, RelayAuthAttempt, RelayAuthError,
    RelayAuthenticator, RelayClientX25519Agreement, RelayCredential, RelayCredentialProvider,
    RelayServerX25519PublicKey, MAX_PINNED_RELAY_X25519_KEYS,
};
pub use bridge::{RelayGuestBridge, RelayHandshake, RelayOwnerBridge};
pub use endpoint::{RelayEndpoint, RelayEndpointError};
pub use error::{
    RelayBridgePhase, RelayBridgeStatus, RelayClientError, RelayFailureKind, RelayStopError,
};
pub use limits::{
    DEFAULT_OWNER_LANE_COUNT, MAX_OWNER_LANE_COUNT, MAX_RELAY_BINARY_BYTES,
    MAX_RELAY_CONNECTION_BYTES,
};
