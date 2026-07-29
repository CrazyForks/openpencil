mod auth;
mod config;
mod connection;
mod connection_reauth;
mod error;
mod pinned_verifiers;
mod production;
mod registry;
mod server;
mod x25519_boundary;

pub use auth::{
    AuthenticatedRoute, CollabTicketRelayAuthenticator, RelayAuthenticator, RelayBearerCredential,
    RelayPossessionPolicy, RelayUpgradeChallenge, RequireRelayChallengeProof, TicketDhBindingOnly,
};
pub use config::{ConfigError, RelayConfig};
pub use error::RelayServerError;
pub use op_collab_policy_file::PinnedPolicyFileFetcher;
pub use pinned_verifiers::{
    PinnedEd25519LocatorVerifier, PinnedVerifierError, MAX_PINNED_VERIFIER_KEYS,
    MAX_PINNED_VERIFIER_KEY_FILE_BYTES, PINNED_VERIFIER_KEY_FILE_VERSION,
};
pub use production::{
    run_production, ProductionRelayAuthConfig, ProductionRelayAuthConfigError,
    ProductionRelayError, HOME_REGION_ENV, LOCATOR_KEYS_FILE_ENV, POLICY_MAX_AGE_ENV,
    RELAY_X25519_KEYS_FILE_ENV, TICKET_POLICY_FILE_ENV,
};
pub use server::{run, run_until, run_with_authenticator, run_with_authenticator_until};
pub use x25519_boundary::{
    PinnedX25519KeyError, PinnedX25519ProofBoundary, RelayServerX25519ProofBoundary,
    RelayX25519BoundaryError, MAX_RELAY_X25519_KEYS, MAX_RELAY_X25519_KEY_FILE_BYTES,
    RELAY_X25519_KEY_FILE_VERSION,
};

#[cfg(test)]
mod production_auth_tests;
#[cfg(test)]
mod production_config_tests;
#[cfg(test)]
mod tests;
