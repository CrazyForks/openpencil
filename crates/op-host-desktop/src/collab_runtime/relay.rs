use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::num::NonZeroU64;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use ed25519_dalek::{Signature, VerifyingKey};
use op_auth_bridge::OpaqueCollabTicket;
use op_collab::{Epoch, SessionId};
use op_collab_relay_client::{
    PinnedRelayX25519Keys, RelayEndpoint, RelayGuestBridge, RelayHandshake, RelayOwnerBridge,
    RelayServerX25519PublicKey, DEFAULT_OWNER_LANE_COUNT, MAX_PINNED_RELAY_X25519_KEYS,
};
use op_collab_relay_control_plane::{
    OwnerPublishDraft, RelayLocatorHttpClient, RelayPublishLifetime,
};
use op_collab_relay_protocol::{
    ExpectedDiscoveryId, LocatorKeyId, LocatorSignature, OwnerNoiseStatic, RelayChallengeKeyId,
    RelayInviteV1, RelayLocatorVerifier, RelayRegion, RouteCapability, RouteId,
    UnsignedRelayLocatorV1, VerifiedRelayRoute, MAX_PAIRING_LIFETIME_SECS,
};
use op_collab_transport::{DeviceStaticKey, ServerPrelude};
use op_editor_core::{CollabConnectionPathUi, CollabInviteCode, CollabRelayRegion};

use super::auth::{unix_time_ms, LocalAdmission};
use super::types::{CollabRuntimeError, CollabRuntimeFailure};

const RELAY_CN_URL_ENV: &str = "OPENPENCIL_COLLAB_RELAY_CN_URL";
const RELAY_GLOBAL_URL_ENV: &str = "OPENPENCIL_COLLAB_RELAY_GLOBAL_URL";
const RELAY_HOME_REGION_ENV: &str = "OPENPENCIL_COLLAB_RELAY_HOME_REGION";
const RELAY_LOCATOR_KEYS_ENV: &str = "OPENPENCIL_COLLAB_RELAY_LOCATOR_KEYS";
const RELAY_LOCATOR_URL_ENV: &str = "OPENPENCIL_COLLAB_RELAY_LOCATOR_URL";
const RELAY_X25519_KEYS_ENV: &str = "OPENPENCIL_COLLAB_RELAY_X25519_KEYS";
#[cfg(any(test, debug_assertions))]
const RELAY_DEV_UNSIGNED_ENV: &str = "OPENPENCIL_COLLAB_RELAY_DEV_UNSIGNED";
#[cfg(any(test, debug_assertions))]
const RELAY_LOCATOR_DEV_HTTP_ENV: &str = "OPENPENCIL_COLLAB_RELAY_LOCATOR_DEV_HTTP";
const MAX_RELAY_ENV_BYTES: usize = 8 * 1024;
const MAX_RELAY_LOCATOR_KEYS: usize = 32;
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
    endpoint: RelayEndpoint,
    route: VerifiedRelayRoute,
    home_region: RelayRegion,
    development_unsigned: bool,
}

pub(super) struct RelayOwnerRequest {
    endpoint: RelayEndpoint,
    home_region: RelayRegion,
    development_unsigned: bool,
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
    ) -> Result<VerifiedRelayRoute, CollabRuntimeFailure>;
}

pub(crate) struct EnvironmentRelayLocatorControlPlane;

impl RelayLocatorControlPlane for EnvironmentRelayLocatorControlPlane {
    fn publish_route(
        &self,
        draft: OwnerPublishDraft,
        ticket: &OpaqueCollabTicket,
    ) -> Result<VerifiedRelayRoute, CollabRuntimeFailure> {
        let endpoint = bounded_environment_value(RELAY_LOCATOR_URL_ENV)?;
        let verifier = locator_verifier_from_environment().map_err(|error| error.failure)?;
        let client = locator_http_client(&endpoint, verifier.clone())?;
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
            request.home_region,
            *key.public_key(),
            discovery_id,
            epoch,
            request.development_unsigned,
            request.control_plane.as_ref(),
            &local,
        )?;
        let authenticator = if request.development_unsigned {
            None
        } else {
            Some(
                LocalAdmission::challenge_bound_relay_authenticator(
                    std::sync::Arc::clone(&local),
                    std::sync::Arc::clone(&key),
                    relay_x25519_keys_from_environment()?,
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
            let bridge = if request.development_unsigned {
                start_development_owner_bridge(request.endpoint, handshake, local_addr).await?
            } else {
                RelayOwnerBridge::start_default_lanes(
                    request.endpoint,
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
    bridge: RelayGuestBridge,
}

impl GuestRelayRuntime {
    pub(super) fn start(
        request: &RelayGuestRequest,
        key: std::sync::Arc<DeviceStaticKey>,
        local: std::sync::Arc<std::sync::RwLock<LocalAdmission>>,
    ) -> Result<Self, CollabRuntimeFailure> {
        let auth = LocalAdmission::relay_auth_extension(*key.public_key())
            .map_err(|error| error.failure)?;
        let handshake = RelayHandshake::new(request.route.clone(), auth);
        let endpoint = request.endpoint.clone();
        let development_unsigned = request.development_unsigned;
        let authenticator = if development_unsigned {
            None
        } else {
            Some(
                LocalAdmission::challenge_bound_relay_authenticator(
                    local,
                    key,
                    relay_x25519_keys_from_environment()?,
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
        Ok(Self { local_addr, bridge })
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
    if std::env::var_os(RELAY_CN_URL_ENV).is_none()
        && std::env::var_os(RELAY_GLOBAL_URL_ENV).is_none()
    {
        return Ok(None);
    }
    let home_region = parse_home_region(std::env::var(RELAY_HOME_REGION_ENV).ok().as_deref())?;
    let endpoint = endpoint_for_region_from_environment(home_region)?;
    let development_unsigned = development_unsigned_allowed(&endpoint);
    Ok(Some(RelayOwnerRequest {
        endpoint,
        home_region,
        development_unsigned,
        control_plane,
    }))
}

pub(super) fn guest_route_from_invite(
    invite: &str,
) -> Result<GuestConnectionRoute, CollabRuntimeError> {
    let invite = RelayInviteV1::from_fragment(invite)
        .map_err(|_| runtime_error(CollabRuntimeFailure::RelayInviteUnavailable))?;
    let now =
        unix_time_ms().map_err(|_| runtime_error(CollabRuntimeFailure::RelayUnavailable))? / 1_000;
    if development_unsigned_environment_value().as_deref() == Some("1") {
        let claimed_region = invite.locator().claims().home_region();
        let endpoint = endpoint_for_region_from_environment(claimed_region)?;
        if development_unsigned_allowed(&endpoint) {
            let route = invite
                .verify(&AcceptAllDevelopmentLocator, now)
                .map_err(|_| runtime_error(CollabRuntimeFailure::RelayInviteUnavailable))?;
            return Ok(GuestConnectionRoute::Relay(Box::new(RelayGuestRequest {
                endpoint,
                route,
                home_region: claimed_region,
                development_unsigned: true,
            })));
        }
    }
    let verifier = locator_verifier_from_environment()?;
    let route = invite
        .verify(&verifier, now)
        .map_err(|_| runtime_error(CollabRuntimeFailure::RelayInviteUnavailable))?;
    let region = route.locator().claims().home_region();
    let endpoint = endpoint_for_region_from_environment(region)?;
    Ok(GuestConnectionRoute::Relay(Box::new(RelayGuestRequest {
        endpoint,
        route,
        home_region: region,
        development_unsigned: false,
    })))
}

pub(super) fn relay_guest_target(
    request: &RelayGuestRequest,
    relay: &GuestRelayRuntime,
) -> (Vec<SocketAddr>, Option<String>, Option<[u8; 32]>) {
    let claims = request.route.locator().claims();
    (
        vec![relay.local_addr()],
        Some(claims.expected_discovery_id().as_str().to_owned()),
        Some(*claims.owner_noise_static().as_bytes()),
    )
}

fn endpoint_from_environment(
    name: &'static str,
) -> Result<Option<RelayEndpoint>, CollabRuntimeError> {
    let Some(value) = std::env::var_os(name) else {
        return Ok(None);
    };
    let value = value
        .to_str()
        .filter(|value| !value.is_empty() && value.len() <= MAX_RELAY_ENV_BYTES)
        .ok_or_else(|| runtime_error(CollabRuntimeFailure::RelayUnavailable))?;
    RelayEndpoint::parse(value)
        .map(Some)
        .map_err(|_| runtime_error(CollabRuntimeFailure::RelayUnavailable))
}

fn parse_home_region(value: Option<&str>) -> Result<RelayRegion, CollabRuntimeError> {
    match value {
        Some("cn") => Ok(RelayRegion::Cn),
        Some("global") => Ok(RelayRegion::Global),
        _ => Err(runtime_error(CollabRuntimeFailure::RelayRegionUnavailable)),
    }
}

#[cfg(test)]
fn endpoint_for_region(
    region: RelayRegion,
    cn: Option<&str>,
    global: Option<&str>,
) -> Result<RelayEndpoint, CollabRuntimeError> {
    let endpoint = match region {
        RelayRegion::Cn => cn,
        RelayRegion::Global => global,
    }
    .ok_or_else(|| runtime_error(CollabRuntimeFailure::RelayRegionUnavailable))?;
    RelayEndpoint::parse(endpoint)
        .map_err(|_| runtime_error(CollabRuntimeFailure::RelayUnavailable))
}

fn endpoint_for_region_from_environment(
    region: RelayRegion,
) -> Result<RelayEndpoint, CollabRuntimeError> {
    let name = match region {
        RelayRegion::Cn => RELAY_CN_URL_ENV,
        RelayRegion::Global => RELAY_GLOBAL_URL_ENV,
    };
    endpoint_from_environment(name)?
        .ok_or_else(|| runtime_error(CollabRuntimeFailure::RelayRegionUnavailable))
}

#[derive(Clone)]
struct Ed25519LocatorVerifier {
    keys: HashMap<String, VerifyingKey>,
}

impl RelayLocatorVerifier for Ed25519LocatorVerifier {
    fn verify(
        &self,
        key_id: &LocatorKeyId,
        canonical_signing_bytes: &[u8],
        signature: &[u8; 64],
    ) -> bool {
        let Some(key) = self.keys.get(key_id.as_str()) else {
            return false;
        };
        key.verify_strict(canonical_signing_bytes, &Signature::from_bytes(signature))
            .is_ok()
    }
}

fn locator_verifier_from_environment() -> Result<Ed25519LocatorVerifier, CollabRuntimeError> {
    let raw = std::env::var(RELAY_LOCATOR_KEYS_ENV)
        .ok()
        .filter(|raw| !raw.is_empty() && raw.len() <= MAX_RELAY_ENV_BYTES)
        .ok_or_else(|| runtime_error(CollabRuntimeFailure::RelayUnavailable))?;
    parse_locator_keys(&raw).map_err(|_| runtime_error(CollabRuntimeFailure::RelayUnavailable))
}

fn bounded_environment_value(name: &'static str) -> Result<String, CollabRuntimeFailure> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty() && value.len() <= MAX_RELAY_ENV_BYTES)
        .ok_or(CollabRuntimeFailure::RelayUnavailable)
}

#[cfg(any(test, debug_assertions))]
fn locator_http_client(
    endpoint: &str,
    verifier: Ed25519LocatorVerifier,
) -> Result<RelayLocatorHttpClient<Ed25519LocatorVerifier>, CollabRuntimeFailure> {
    if let Ok(client) = RelayLocatorHttpClient::new(endpoint, verifier.clone()) {
        return Ok(client);
    }
    let development_http = std::env::var(RELAY_LOCATOR_DEV_HTTP_ENV).ok().as_deref() == Some("1");
    RelayLocatorHttpClient::new_loopback_http_for_development(endpoint, verifier, development_http)
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
}

#[cfg(not(any(test, debug_assertions)))]
fn locator_http_client(
    endpoint: &str,
    verifier: Ed25519LocatorVerifier,
) -> Result<RelayLocatorHttpClient<Ed25519LocatorVerifier>, CollabRuntimeFailure> {
    RelayLocatorHttpClient::new(endpoint, verifier)
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
}

fn parse_locator_keys(raw: &str) -> Result<Ed25519LocatorVerifier, ()> {
    let mut keys = HashMap::new();
    for entry in raw.split([',', ';']) {
        let entry = entry.trim();
        if entry.is_empty() {
            return Err(());
        }
        let (key_id, encoded) = entry.split_once('=').ok_or(())?;
        LocatorKeyId::new(key_id.to_owned()).map_err(|_| ())?;
        if encoded.is_empty() || encoded.contains('=') {
            return Err(());
        }
        let decoded = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| ())?;
        if URL_SAFE_NO_PAD.encode(&decoded) != encoded {
            return Err(());
        }
        let bytes: [u8; 32] = decoded.try_into().map_err(|_| ())?;
        let key = VerifyingKey::from_bytes(&bytes).map_err(|_| ())?;
        if keys.insert(key_id.to_owned(), key).is_some() || keys.len() > MAX_RELAY_LOCATOR_KEYS {
            return Err(());
        }
    }
    (!keys.is_empty())
        .then_some(Ed25519LocatorVerifier { keys })
        .ok_or(())
}

fn relay_x25519_keys_from_environment(
) -> Result<std::sync::Arc<PinnedRelayX25519Keys>, CollabRuntimeFailure> {
    let raw = bounded_environment_value(RELAY_X25519_KEYS_ENV)?;
    parse_relay_x25519_keys(&raw).map(std::sync::Arc::new)
}

fn parse_relay_x25519_keys(raw: &str) -> Result<PinnedRelayX25519Keys, CollabRuntimeFailure> {
    let mut keys = Vec::new();
    for entry in raw.split([',', ';']) {
        let entry = entry.trim();
        let (key_id, encoded) = entry
            .split_once('=')
            .filter(|(key_id, encoded)| !key_id.is_empty() && !encoded.is_empty())
            .ok_or(CollabRuntimeFailure::RelayUnavailable)?;
        if encoded.contains('=') || keys.len() >= MAX_PINNED_RELAY_X25519_KEYS {
            return Err(CollabRuntimeFailure::RelayUnavailable);
        }
        let key_id = RelayChallengeKeyId::new(key_id.to_owned())
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        let decoded = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        if URL_SAFE_NO_PAD.encode(&decoded) != encoded {
            return Err(CollabRuntimeFailure::RelayUnavailable);
        }
        let public_key: [u8; 32] = decoded
            .try_into()
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        keys.push(
            RelayServerX25519PublicKey::new(key_id, public_key)
                .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        );
    }
    PinnedRelayX25519Keys::new(keys).map_err(|_| CollabRuntimeFailure::RelayUnavailable)
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
    home_region: RelayRegion,
    owner_static: [u8; 32],
    discovery_id: String,
    epoch: Epoch,
    development_unsigned: bool,
    control_plane: &dyn RelayLocatorControlPlane,
    local: &std::sync::RwLock<LocalAdmission>,
) -> Result<VerifiedRelayRoute, CollabRuntimeFailure> {
    if !development_unsigned {
        return publish_production_route(
            home_region,
            owner_static,
            discovery_id,
            control_plane,
            local,
        );
    }
    let now = unix_time_ms().map_err(|_| CollabRuntimeFailure::RelayUnavailable)? / 1_000;
    let not_before = now.saturating_sub(1).max(1);
    let expires_at = now
        .checked_add(MAX_PAIRING_LIFETIME_SECS.saturating_sub(60))
        .ok_or(CollabRuntimeFailure::RelayUnavailable)?;
    let key_id = LocatorKeyId::new(DEBUG_LOCATOR_KEY_ID.to_owned())
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
    let claims = UnsignedRelayLocatorV1::new(
        home_region,
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
    home_region: RelayRegion,
    owner_static: [u8; 32],
    discovery_id: String,
    control_plane: &dyn RelayLocatorControlPlane,
    local: &std::sync::RwLock<LocalAdmission>,
) -> Result<VerifiedRelayRoute, CollabRuntimeFailure> {
    let draft = OwnerPublishDraft::generate(
        home_region,
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
    control_plane.publish_route(draft, local.relay_ticket())
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
