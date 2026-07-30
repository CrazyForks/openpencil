use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::num::NonZeroU64;
use std::sync::Arc;

use op_auth_bridge::OpaqueCollabTicket;
use op_collab::{Epoch, SessionId};
#[cfg(any(test, debug_assertions))]
use op_collab_relay_client::DEFAULT_OWNER_LANE_COUNT;
use op_collab_relay_client::{RelayEndpoint, RelayGuestBridge, RelayHandshake, RelayOwnerBridge};
use op_collab_relay_control_plane::{
    OwnerPublishDraft, RelayLocatorHttpClient, RelayPublishLifetime,
};
use op_collab_relay_protocol::{
    ExpectedDiscoveryId, LocatorKeyId, LocatorSignature, OwnerNoiseStatic, RelayInviteV1,
    RelayLocatorVerifier, RelayRegion, RouteCapability, RouteId, UnsignedRelayLocatorV1,
    VerifiedRelayRoute, MAX_PAIRING_LIFETIME_SECS,
};
use op_collab_transport::{DeviceStaticKey, ServerPrelude};
use op_editor_core::{CollabConnectionPathUi, CollabInviteCode, CollabRelayRegion};

use super::auth::{unix_time_ms, LocalAdmission};
#[cfg(test)]
use super::relay_bootstrap::RelayBootstrap;
use super::relay_bootstrap::{
    provider_from_environment, Ed25519LocatorVerifier, RelayBootstrapProvider, RelayBootstrapRegion,
};
use super::types::{CollabRuntimeError, CollabRuntimeFailure};

const RELAY_HOME_REGION_ENV: &str = "OPENPENCIL_COLLAB_RELAY_HOME_REGION";
#[cfg(any(test, debug_assertions))]
const RELAY_DEV_UNSIGNED_ENV: &str = "OPENPENCIL_COLLAB_RELAY_DEV_UNSIGNED";
const DEBUG_LOCATOR_KEY_ID: &str = "openpencil-debug-unsigned";
const OWNER_RELAY_READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(22);

#[derive(Clone)]
pub(super) enum GuestConnectionRoute {
    Lan {
        addresses: Vec<SocketAddr>,
        discovery_id: Option<String>,
        expected_remote_static: Option<[u8; 32]>,
    },
    Relay(Box<RelayGuestRequest>),
}

impl GuestConnectionRoute {
    pub(super) fn lan(
        addresses: Vec<SocketAddr>,
        discovery_id: Option<String>,
        expected_remote_static: Option<[u8; 32]>,
    ) -> Self {
        Self::Lan {
            addresses,
            discovery_id,
            expected_remote_static,
        }
    }

    pub(super) fn retry_with_owner_static(&self, owner_static: [u8; 32]) -> Self {
        match self {
            Self::Lan { addresses, .. } => Self::Lan {
                addresses: addresses.clone(),
                discovery_id: None,
                expected_remote_static: Some(owner_static),
            },
            Self::Relay(request) => Self::Relay(request.clone()),
        }
    }

    pub(super) fn status_endpoint(&self) -> Option<SocketAddr> {
        match self {
            Self::Lan { addresses, .. } => addresses.first().copied(),
            Self::Relay(_) => None,
        }
    }

    pub(super) fn connection_path(&self) -> CollabConnectionPathUi {
        match self {
            Self::Lan { .. } => CollabConnectionPathUi::Lan,
            Self::Relay(request) => CollabConnectionPathUi::Relay {
                home_region: ui_region(request.home_region),
            },
        }
    }
}

#[derive(Clone)]
pub(super) struct RelayGuestRequest {
    invite: RelayInviteV1,
    home_region: RelayRegion,
    provider: std::sync::Arc<dyn RelayBootstrapProvider>,
}

pub(super) struct RelayOwnerRequest {
    home_region: RelayRegion,
    provider: std::sync::Arc<dyn RelayBootstrapProvider>,
    control_plane: std::sync::Arc<dyn RelayLocatorControlPlane>,
}

/// Authenticated control-plane boundary for owner route publication.
///
/// The draft keeps the bearer route capability on-device. Implementations send
/// only its bounded publish request and the short-lived collaboration ticket;
/// the service independently verifies ticket-to-owner-key binding and delegates
/// signing to its HSM/KMS. The desktop never receives a signing key.
pub(crate) trait RelayLocatorControlPlane: Send + Sync {
    fn publish_route(
        &self,
        draft: OwnerPublishDraft,
        ticket: &OpaqueCollabTicket,
        region: &RelayBootstrapRegion,
    ) -> Result<VerifiedRelayRoute, CollabRuntimeFailure>;
}

pub(crate) struct EnvironmentRelayLocatorControlPlane;

impl RelayLocatorControlPlane for EnvironmentRelayLocatorControlPlane {
    fn publish_route(
        &self,
        draft: OwnerPublishDraft,
        ticket: &OpaqueCollabTicket,
        region: &RelayBootstrapRegion,
    ) -> Result<VerifiedRelayRoute, CollabRuntimeFailure> {
        let verifier = region.locator_verifier.clone();
        let client = locator_http_client(region)?;
        let published = client
            .publish(draft, ticket)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        let now = unix_time_ms().map_err(|_| CollabRuntimeFailure::RelayUnavailable)? / 1_000;
        published
            .invite()
            .verify(&verifier, now)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
    }
}

pub(super) struct OwnerRelayRuntime {
    listener: TcpListener,
    prelude: std::sync::Arc<ServerPrelude>,
    invite: CollabInviteCode,
    path: CollabConnectionPathUi,
    bridge: RelayOwnerBridge,
}

impl OwnerRelayRuntime {
    pub(super) fn start(
        request: RelayOwnerRequest,
        key: std::sync::Arc<DeviceStaticKey>,
        local: std::sync::Arc<std::sync::RwLock<LocalAdmission>>,
        session_id: &SessionId,
        epoch: Epoch,
    ) -> Result<Self, CollabRuntimeFailure> {
        // The owner network worker resolves one signed bootstrap snapshot and
        // uses that same region entry for locator verification, relay pinning,
        // and the bridge endpoint. No HTTP runs on the UI/event-loop thread.
        let bootstrap = request.provider.load()?;
        let region = bootstrap.region(request.home_region)?;
        let endpoint = region.relay_endpoint.clone();
        let development_unsigned = development_unsigned_allowed(&endpoint);
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        listener
            .set_nonblocking(true)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        let local_addr = listener
            .local_addr()
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        let discovery_id = random_relay_discovery_id()?;
        let prelude = std::sync::Arc::new(
            ServerPrelude::new(discovery_id.clone(), session_id.clone(), epoch)
                .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        );
        let route = owner_route(
            *key.public_key(),
            discovery_id,
            epoch,
            development_unsigned,
            request.control_plane.as_ref(),
            region,
            &local,
        )?;
        let authenticator = if development_unsigned {
            None
        } else {
            Some(
                LocalAdmission::challenge_bound_relay_authenticator(
                    std::sync::Arc::clone(&local),
                    std::sync::Arc::clone(&key),
                    Arc::clone(&region.relay_x25519_keys),
                )
                .map_err(|error| error.failure)?,
            )
        };
        let invite = CollabInviteCode::new(RelayInviteV1::new(&route).to_fragment())
            .ok_or(CollabRuntimeFailure::RelayUnavailable)?;
        let auth = LocalAdmission::relay_auth_extension(*key.public_key())
            .map_err(|error| error.failure)?;
        let handshake = RelayHandshake::new(route, auth);
        let bridge = op_host_services::chat_runtime::block_on_anywhere(async move {
            let bridge = if development_unsigned {
                start_development_owner_bridge(endpoint, handshake, local_addr).await?
            } else {
                RelayOwnerBridge::start_default_lanes(
                    endpoint,
                    handshake,
                    local_addr,
                    authenticator.ok_or(CollabRuntimeFailure::RelayUnavailable)?,
                )
                .await
                .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?
            };
            bridge
                .wait_until_ready(OWNER_RELAY_READY_TIMEOUT)
                .await
                .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
            Ok::<_, CollabRuntimeFailure>(bridge)
        })?;
        Ok(Self {
            listener,
            prelude,
            invite,
            path: CollabConnectionPathUi::Relay {
                home_region: ui_region(request.home_region),
            },
            bridge,
        })
    }

    pub(super) fn accept(&self) -> std::io::Result<(TcpStream, SocketAddr)> {
        self.listener.accept()
    }

    pub(super) fn prelude(&self) -> std::sync::Arc<ServerPrelude> {
        std::sync::Arc::clone(&self.prelude)
    }

    pub(super) fn invite(&self) -> CollabInviteCode {
        self.invite.clone()
    }

    pub(super) const fn path(&self) -> CollabConnectionPathUi {
        self.path
    }
}

impl std::fmt::Debug for OwnerRelayRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OwnerRelayRuntime")
            .field("path", &self.path)
            .field("bridge_status", &self.bridge.status())
            .finish()
    }
}

pub(super) struct GuestRelayRuntime {
    local_addr: SocketAddr,
    expected_discovery_id: String,
    expected_remote_static: [u8; 32],
    bridge: RelayGuestBridge,
}

impl GuestRelayRuntime {
    pub(super) fn start(
        request: &RelayGuestRequest,
        key: std::sync::Arc<DeviceStaticKey>,
        local: std::sync::Arc<std::sync::RwLock<LocalAdmission>>,
    ) -> Result<Self, CollabRuntimeFailure> {
        // Resolve and verify the invite on the guest network worker. The UI
        // only parses enough of the bounded invite to select its claimed
        // region and render status.
        let bootstrap = request.provider.load()?;
        let region = bootstrap.region(request.home_region)?;
        let endpoint = region.relay_endpoint.clone();
        let development_unsigned = development_unsigned_allowed(&endpoint);
        let now = unix_time_ms().map_err(|_| CollabRuntimeFailure::RelayUnavailable)? / 1_000;
        let route = if development_unsigned {
            request
                .invite
                .verify(&AcceptAllDevelopmentLocator, now)
                .map_err(|_| CollabRuntimeFailure::RelayInviteUnavailable)?
        } else {
            request
                .invite
                .verify(&region.locator_verifier, now)
                .map_err(|_| CollabRuntimeFailure::RelayInviteUnavailable)?
        };
        if route.locator().claims().home_region() != request.home_region {
            return Err(CollabRuntimeFailure::RelayInviteUnavailable);
        }
        let expected_discovery_id = route
            .locator()
            .claims()
            .expected_discovery_id()
            .as_str()
            .to_owned();
        let expected_remote_static = *route.locator().claims().owner_noise_static().as_bytes();
        let auth = LocalAdmission::relay_auth_extension(*key.public_key())
            .map_err(|error| error.failure)?;
        let handshake = RelayHandshake::new(route, auth);
        let authenticator = if development_unsigned {
            None
        } else {
            Some(
                LocalAdmission::challenge_bound_relay_authenticator(
                    local,
                    key,
                    Arc::clone(&region.relay_x25519_keys),
                )
                .map_err(|error| error.failure)?,
            )
        };
        let bridge = op_host_services::chat_runtime::block_on_anywhere(async move {
            if development_unsigned {
                start_development_guest_bridge(endpoint, handshake).await
            } else {
                RelayGuestBridge::start(
                    endpoint,
                    handshake,
                    authenticator.ok_or(CollabRuntimeFailure::RelayUnavailable)?,
                )
                .await
                .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
            }
        })?;
        let local_addr = bridge.local_addr();
        Ok(Self {
            local_addr,
            expected_discovery_id,
            expected_remote_static,
            bridge,
        })
    }

    pub(super) const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl std::fmt::Debug for GuestRelayRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GuestRelayRuntime")
            .field("local_addr", &self.local_addr)
            .field("bridge_status", &self.bridge.status())
            .finish()
    }
}

pub(super) fn owner_request_from_environment(
    control_plane: std::sync::Arc<dyn RelayLocatorControlPlane>,
) -> Result<Option<RelayOwnerRequest>, CollabRuntimeError> {
    let Some(provider) = provider_from_environment()? else {
        return Ok(None);
    };
    let home_region = parse_home_region(std::env::var(RELAY_HOME_REGION_ENV).ok().as_deref())?;
    Ok(Some(RelayOwnerRequest {
        home_region,
        provider,
        control_plane,
    }))
}

pub(super) fn guest_route_from_invite(
    invite: &str,
) -> Result<GuestConnectionRoute, CollabRuntimeError> {
    let invite = RelayInviteV1::from_fragment(invite)
        .map_err(|_| runtime_error(CollabRuntimeFailure::RelayInviteUnavailable))?;
    let provider = provider_from_environment()?
        .ok_or_else(|| runtime_error(CollabRuntimeFailure::RelayUnavailable))?;
    Ok(guest_route_from_parsed_invite(invite, provider))
}

fn guest_route_from_parsed_invite(
    invite: RelayInviteV1,
    provider: Arc<dyn RelayBootstrapProvider>,
) -> GuestConnectionRoute {
    let region = invite.locator().claims().home_region();
    GuestConnectionRoute::Relay(Box::new(RelayGuestRequest {
        invite,
        home_region: region,
        provider,
    }))
}

pub(super) fn relay_guest_target(
    relay: &GuestRelayRuntime,
) -> (Vec<SocketAddr>, Option<String>, Option<[u8; 32]>) {
    (
        vec![relay.local_addr()],
        Some(relay.expected_discovery_id.clone()),
        Some(relay.expected_remote_static),
    )
}

fn parse_home_region(value: Option<&str>) -> Result<RelayRegion, CollabRuntimeError> {
    match value {
        Some("cn") => Ok(RelayRegion::Cn),
        Some("global") => Ok(RelayRegion::Global),
        _ => Err(runtime_error(CollabRuntimeFailure::RelayRegionUnavailable)),
    }
}

#[cfg(any(test, debug_assertions))]
fn locator_http_client(
    region: &RelayBootstrapRegion,
) -> Result<RelayLocatorHttpClient<Ed25519LocatorVerifier>, CollabRuntimeFailure> {
    if let Ok(client) =
        RelayLocatorHttpClient::new(&region.locator_url, region.locator_verifier.clone())
    {
        return Ok(client);
    }
    RelayLocatorHttpClient::new_loopback_http_for_development(
        &region.locator_url,
        region.locator_verifier.clone(),
        region.development_http,
    )
    .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
}

#[cfg(not(any(test, debug_assertions)))]
fn locator_http_client(
    region: &RelayBootstrapRegion,
) -> Result<RelayLocatorHttpClient<Ed25519LocatorVerifier>, CollabRuntimeFailure> {
    RelayLocatorHttpClient::new(&region.locator_url, region.locator_verifier.clone())
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
}

#[derive(Clone, Copy)]
struct AcceptAllDevelopmentLocator;

impl RelayLocatorVerifier for AcceptAllDevelopmentLocator {
    fn verify(
        &self,
        _key_id: &LocatorKeyId,
        _canonical_signing_bytes: &[u8],
        _signature: &[u8; 64],
    ) -> bool {
        true
    }
}

fn development_unsigned_allowed(endpoint: &RelayEndpoint) -> bool {
    development_unsigned_opt_in(
        cfg!(any(test, debug_assertions)),
        !endpoint.is_encrypted(),
        development_unsigned_environment_value(),
    )
}

#[cfg(any(test, debug_assertions))]
fn development_unsigned_environment_value() -> Option<String> {
    std::env::var(RELAY_DEV_UNSIGNED_ENV).ok()
}

#[cfg(not(any(test, debug_assertions)))]
fn development_unsigned_environment_value() -> Option<String> {
    None
}

fn development_unsigned_opt_in(
    debug_build: bool,
    loopback_ws: bool,
    value: Option<String>,
) -> bool {
    debug_build && loopback_ws && value.as_deref() == Some("1")
}

fn owner_route(
    owner_static: [u8; 32],
    discovery_id: String,
    epoch: Epoch,
    development_unsigned: bool,
    control_plane: &dyn RelayLocatorControlPlane,
    region: &RelayBootstrapRegion,
    local: &std::sync::RwLock<LocalAdmission>,
) -> Result<VerifiedRelayRoute, CollabRuntimeFailure> {
    if !development_unsigned {
        return publish_production_route(owner_static, discovery_id, control_plane, region, local);
    }
    let now = unix_time_ms().map_err(|_| CollabRuntimeFailure::RelayUnavailable)? / 1_000;
    let not_before = now.saturating_sub(1).max(1);
    let expires_at = now
        .checked_add(MAX_PAIRING_LIFETIME_SECS.saturating_sub(60))
        .ok_or(CollabRuntimeFailure::RelayUnavailable)?;
    let key_id = LocatorKeyId::new(DEBUG_LOCATOR_KEY_ID.to_owned())
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    let claims = UnsignedRelayLocatorV1::new(
        region.region,
        RouteId::generate().map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        NonZeroU64::new(epoch.0).unwrap_or(NonZeroU64::MIN),
        OwnerNoiseStatic::new(owner_static).map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        ExpectedDiscoveryId::new(discovery_id)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        not_before,
        expires_at,
        key_id.clone(),
    )
    .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    let locator = claims.attach_signature(
        LocatorSignature::new([0xA5; 64]).map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
    );
    let verified = locator
        .verify(&AcceptAllDevelopmentLocator, now)
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    Ok(VerifiedRelayRoute::new(
        verified,
        RouteCapability::generate().map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
    ))
}

fn publish_production_route(
    owner_static: [u8; 32],
    discovery_id: String,
    control_plane: &dyn RelayLocatorControlPlane,
    region: &RelayBootstrapRegion,
    local: &std::sync::RwLock<LocalAdmission>,
) -> Result<VerifiedRelayRoute, CollabRuntimeFailure> {
    let draft = OwnerPublishDraft::generate(
        region.region,
        OwnerNoiseStatic::new(owner_static).map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        ExpectedDiscoveryId::new(discovery_id)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        RelayPublishLifetime::new(MAX_PAIRING_LIFETIME_SECS.saturating_sub(60))
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
    )
    .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    let local = local
        .read()
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    control_plane.publish_route(draft, local.relay_ticket(), region)
}

fn random_relay_discovery_id() -> Result<String, CollabRuntimeFailure> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    let mut encoded = String::with_capacity(bytes.len() * 2);
    use std::fmt::Write as _;
    for byte in bytes {
        write!(encoded, "{byte:02x}").map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    }
    Ok(encoded)
}

#[cfg(any(test, debug_assertions))]
async fn start_development_owner_bridge(
    endpoint: RelayEndpoint,
    handshake: RelayHandshake,
    local_addr: SocketAddr,
) -> Result<RelayOwnerBridge, CollabRuntimeFailure> {
    RelayOwnerBridge::start_unauthenticated_for_development(
        endpoint,
        handshake,
        local_addr,
        DEFAULT_OWNER_LANE_COUNT,
    )
    .await
    .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
}

#[cfg(not(any(test, debug_assertions)))]
async fn start_development_owner_bridge(
    _endpoint: RelayEndpoint,
    _handshake: RelayHandshake,
    _local_addr: SocketAddr,
) -> Result<RelayOwnerBridge, CollabRuntimeFailure> {
    Err(CollabRuntimeFailure::RelayUnavailable)
}

#[cfg(any(test, debug_assertions))]
async fn start_development_guest_bridge(
    endpoint: RelayEndpoint,
    handshake: RelayHandshake,
) -> Result<RelayGuestBridge, CollabRuntimeFailure> {
    RelayGuestBridge::start_unauthenticated_for_development(endpoint, handshake)
        .await
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
}

#[cfg(not(any(test, debug_assertions)))]
async fn start_development_guest_bridge(
    _endpoint: RelayEndpoint,
    _handshake: RelayHandshake,
) -> Result<RelayGuestBridge, CollabRuntimeFailure> {
    Err(CollabRuntimeFailure::RelayUnavailable)
}

const fn ui_region(region: RelayRegion) -> CollabRelayRegion {
    match region {
        RelayRegion::Cn => CollabRelayRegion::China,
        RelayRegion::Global => CollabRelayRegion::Global,
    }
}

const fn runtime_error(failure: CollabRuntimeFailure) -> CollabRuntimeError {
    CollabRuntimeError::new(failure)
}

#[cfg(test)]
#[path = "relay_tests.rs"]
mod tests;
