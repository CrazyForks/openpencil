//! Open locator issuance core for the OpenPencil public collaboration relay.
//!
//! The owner generates the route id, generation, and bearer capability
//! locally. Only the bounded [`OwnerPublishRequest`] is sent over authenticated
//! HTTPS. A control plane verifies the collaboration ticket, binds its device
//! DH key to the requested owner Noise key, and delegates signing to an
//! external HSM/KMS through [`RelayLocatorSigner`]. The owner then verifies the
//! signed response against both pinned locator keys and its exact pending
//! request before combining it with the locally held capability.

mod error;
mod http_client;
mod issuer;
mod publish;
mod service;

pub use error::{RelayLocatorIssueError, RelayLocatorSignerError};
pub use http_client::{
    RelayLocatorHttpClient, CONTROL_PLANE_CONNECT_TIMEOUT, CONTROL_PLANE_REQUEST_TIMEOUT,
    MAX_PUBLISH_AUTHORIZATION_BYTES, OWNER_PUBLISH_CONTENT_TYPE, RELAY_LOCATOR_PUBLISH_PATH,
    SIGNED_LOCATOR_CONTENT_TYPE,
};
pub use issuer::{
    RelayLocatorIssuer, RelayLocatorSigner, SignedLocatorResponse, TicketVerifiedOwnerBinding,
};
pub use publish::{
    OwnerPublishDraft, OwnerPublishRequest, RelayPublishLifetime, SecretInviteCode,
    SignedInviteResponse, MAX_ISSUER_CLOCK_SKEW_SECS, OWNER_PUBLISH_REQUEST_BYTES,
};
pub use service::{
    RegionBoundOwnerPublishPolicy, RelayLocatorPublishService, RelayLocatorPublishServiceError,
    RelayOwnerPublishPolicy, RelayOwnerPublishPolicyContext,
};

#[cfg(test)]
mod tests;
