use std::collections::{HashMap, HashSet};
use std::io::Read as _;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use ed25519_dalek::{Signature, VerifyingKey};
use op_collab_relay_client::{
    PinnedRelayX25519Keys, RelayEndpoint, RelayServerX25519PublicKey, MAX_PINNED_RELAY_X25519_KEYS,
};
use op_collab_relay_protocol::{
    LocatorKeyId, RelayChallengeKeyId, RelayLocatorVerifier, RelayRegion,
};
use op_collab_transport::DeviceStaticKey;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use reqwest::redirect::Policy;
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use super::types::{CollabRuntimeError, CollabRuntimeFailure};

#[path = "relay_bootstrap_url.rs"]
mod bootstrap_url;

#[path = "relay_bootstrap_cache.rs"]
mod bootstrap_cache;

use bootstrap_cache::{read_cache, write_cache, BootstrapCache};

pub(super) const BOOTSTRAP_URL_ENV: &str = "OPENPENCIL_COLLAB_BOOTSTRAP_URL";
#[cfg(any(test, debug_assertions))]
const BOOTSTRAP_DEV_HTTP_ENV: &str = "OPENPENCIL_COLLAB_BOOTSTRAP_DEV_HTTP";
#[cfg(any(test, debug_assertions))]
const BOOTSTRAP_DEV_ROOT_KEYS_ENV: &str = "OPENPENCIL_COLLAB_BOOTSTRAP_DEV_ROOT_KEYS";

const BOOTSTRAP_PATH: &str = "/api/v1/collaboration/bootstrap";
const BOOTSTRAP_CONTEXT: &[u8] = b"openpencil/op-hub/collaboration-bootstrap/v1\0";
const BOOTSTRAP_CACHE_FILE: &str = "collaboration-bootstrap-v1.json";
const BOOTSTRAP_CONTENT_TYPE: &str = "application/json";
const BOOTSTRAP_VERSION: u64 = 1;
const BUILTIN_ROOT_KID: &str = "openpencil-collab-union-root-v2";
const BUILTIN_ROOT_X: &str = "5SVj-_jnJbuZlpDoD3M9x1eZAPDFLSq5jRb-c0xUh5A";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_PAYLOAD_BYTES: usize = 32 * 1024;
const MAX_CACHE_BYTES: u64 = (MAX_RESPONSE_BYTES as u64 * 2) + 4_096;
#[cfg(any(test, debug_assertions))]
const MAX_ENV_BYTES: usize = 8 * 1024;
const MAX_ETAG_BYTES: usize = 256;
#[cfg(any(test, debug_assertions))]
const MAX_ROOT_KEYS: usize = 8;
const MAX_REGION_KEYS: usize = 8;
const MAX_KEY_ID_BYTES: usize = 64;
const MAX_RELAY_KEY_ID_BYTES: usize = 30;
const MAX_URL_BYTES: usize = 2_048;
const MAX_CLOCK_SKEW_SECS: u64 = 300;
const MAX_VALIDITY_SECS: u64 = 7 * 24 * 60 * 60;
const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;
const MAX_UNIX_SECOND: u64 = 253_402_300_799;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

static CACHE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(super) trait RelayBootstrapProvider: Send + Sync {
    fn load(&self) -> Result<Arc<RelayBootstrap>, CollabRuntimeFailure>;
}

#[derive(Debug)]
pub(super) struct RelayBootstrap {
    generation: u64,
    signed_payload: Box<[u8]>,
    regions: Vec<RelayBootstrapRegion>,
}

impl RelayBootstrap {
    pub(super) fn region(
        &self,
        region: RelayRegion,
    ) -> Result<&RelayBootstrapRegion, CollabRuntimeFailure> {
        self.regions
            .iter()
            .find(|candidate| candidate.region == region)
            .ok_or(CollabRuntimeFailure::RelayRegionUnavailable)
    }
}

#[derive(Debug)]
pub(crate) struct RelayBootstrapRegion {
    pub(super) region: RelayRegion,
    pub(super) relay_endpoint: RelayEndpoint,
    pub(super) locator_url: String,
    pub(super) locator_verifier: Ed25519LocatorVerifier,
    pub(super) relay_x25519_keys: Arc<PinnedRelayX25519Keys>,
    #[cfg(any(test, debug_assertions))]
    pub(super) development_http: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Ed25519LocatorVerifier {
    keys: HashMap<String, VerifyingKey>,
}

impl RelayLocatorVerifier for Ed25519LocatorVerifier {
    fn verify(
        &self,
        key_id: &LocatorKeyId,
        canonical_signing_bytes: &[u8],
        signature: &[u8; 64],
    ) -> bool {
        self.keys.get(key_id.as_str()).is_some_and(|key| {
            key.verify_strict(canonical_signing_bytes, &Signature::from_bytes(signature))
                .is_ok()
        })
    }
}

pub(super) fn provider_from_environment(
) -> Result<Option<Arc<dyn RelayBootstrapProvider>>, CollabRuntimeError> {
    let Some(raw) = std::env::var_os(BOOTSTRAP_URL_ENV) else {
        return Ok(None);
    };
    let raw = raw
        .to_str()
        .filter(|value| !value.is_empty() && value.len() <= MAX_URL_BYTES)
        .ok_or_else(relay_runtime_error)?;
    let provider =
        EnvironmentRelayBootstrapProvider::new(raw).map_err(|_| relay_runtime_error())?;
    Ok(Some(Arc::new(provider)))
}

struct EnvironmentRelayBootstrapProvider {
    endpoint: Url,
    roots: HashMap<String, VerifyingKey>,
    development_http: bool,
    cache_path: PathBuf,
}

impl EnvironmentRelayBootstrapProvider {
    fn new(endpoint: &str) -> Result<Self, BootstrapError> {
        let (endpoint, development_http) = parse_bootstrap_endpoint(endpoint)?;
        let mut roots = builtin_roots()?;
        if development_http {
            add_development_roots(&mut roots)?;
        }
        let cache_path = op_config_store::ConfigStore::user()
            .and_then(|store| store.path(BOOTSTRAP_CACHE_FILE))
            .map_err(|_| BootstrapError::Cache)?;
        Ok(Self {
            endpoint,
            roots,
            development_http,
            cache_path,
        })
    }

    fn load_inner(&self, now: u64) -> Result<Arc<RelayBootstrap>, BootstrapError> {
        let _guard = CACHE_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| BootstrapError::Cache)?;
        // Only an actually absent cache is an empty floor. A corrupt,
        // unreadable, or non-regular cache must stop bootstrap resolution;
        // otherwise fetching without it could silently accept a generation
        // below the floor established by an earlier run.
        let cached = read_cache(&self.cache_path, self.endpoint.as_str())?;
        let cached_verified = cached.as_ref().and_then(|cached| {
            verify_bootstrap(
                cached.body.as_bytes(),
                &self.roots,
                now,
                self.development_http,
                true,
            )
            .ok()
        });
        let cached_signed = cached.as_ref().and_then(|cached| {
            verify_bootstrap(
                cached.body.as_bytes(),
                &self.roots,
                now,
                self.development_http,
                false,
            )
            .ok()
        });
        let response = match self.fetch(cached.as_ref()) {
            Ok(response) => response,
            Err(_) => {
                return cached_verified
                    .map(Arc::new)
                    .ok_or(BootstrapError::Transport)
            }
        };
        if response.status() == StatusCode::NOT_MODIFIED {
            validate_not_modified(response.headers(), cached.as_ref())?;
            return cached_verified
                .map(Arc::new)
                .ok_or(BootstrapError::InvalidResponse);
        }
        if response.status() != StatusCode::OK {
            return cached_verified
                .map(Arc::new)
                .ok_or(BootstrapError::Transport);
        }
        validate_content_type(response.headers())?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(BootstrapError::ResponseTooLarge);
        }
        let response_headers = response.headers().clone();
        let mut body = Vec::with_capacity(MAX_RESPONSE_BYTES);
        response
            .take((MAX_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut body)
            .map_err(|_| BootstrapError::Transport)?;
        if body.len() > MAX_RESPONSE_BYTES {
            return Err(BootstrapError::ResponseTooLarge);
        }
        let etag = Some(response_etag(&response_headers, &body)?);
        let verified = verify_bootstrap(&body, &self.roots, now, self.development_http, true)?;
        if let Some(previous) = cached_signed.as_ref() {
            reject_rollback(previous, &verified)?;
        }
        // The persisted document is what arms the anti-rollback generation
        // floor on the next start: `cached_signed` above is read back from
        // exactly this file. Discarding a write failure would let that
        // security property disappear on an unwritable configuration
        // directory with no signal at all, so the failure is propagated.
        //
        // It is propagated rather than logged because this module — and the
        // whole desktop `collab_runtime` — deliberately contains no
        // `tracing` / `println!` call: collaboration identities, endpoints,
        // and ticket material flow through here, and the absence of a logging
        // sink is the boundary that keeps them out of log files. The typed
        // `BootstrapError::CachePersist` carries the reason without carrying
        // any secret, identity, or path material, and the caller collapses it
        // into `CollabRuntimeFailure::RelayUnavailable`.
        //
        // Verification is untouched: this runs only after `verify_bootstrap`
        // and `reject_rollback` have already accepted the document, so the
        // verifier stays exactly as fail-closed as before.
        let body = String::from_utf8(body).map_err(|_| BootstrapError::CachePersist)?;
        write_cache(
            &self.cache_path,
            &BootstrapCache {
                endpoint: self.endpoint.as_str().to_owned(),
                etag,
                body,
            },
        )?;
        Ok(Arc::new(verified))
    }

    fn fetch(
        &self,
        cached: Option<&BootstrapCache>,
    ) -> Result<reqwest::blocking::Response, BootstrapError> {
        let client = Client::builder()
            .redirect(Policy::none())
            .no_proxy()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .https_only(!self.development_http)
            .build()
            .map_err(|_| BootstrapError::Transport)?;
        let mut request = client
            .get(self.endpoint.clone())
            .header(ACCEPT, BOOTSTRAP_CONTENT_TYPE);
        if let Some(etag) = cached.and_then(|cached| cached.etag.as_deref()) {
            let value = HeaderValue::from_str(etag).map_err(|_| BootstrapError::Cache)?;
            request = request.header(IF_NONE_MATCH, value);
        }
        request.send().map_err(|_| BootstrapError::Transport)
    }
}

impl RelayBootstrapProvider for EnvironmentRelayBootstrapProvider {
    fn load(&self) -> Result<Arc<RelayBootstrap>, CollabRuntimeFailure> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?
            .as_secs();
        self.load_inner(now)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
    }
}

fn parse_bootstrap_endpoint(value: &str) -> Result<(Url, bool), BootstrapError> {
    parse_bootstrap_endpoint_with_policy(value, development_http_enabled())
}

fn parse_bootstrap_endpoint_with_policy(
    value: &str,
    allow_development_http: bool,
) -> Result<(Url, bool), BootstrapError> {
    let endpoint =
        bootstrap_url::parse(value, BOOTSTRAP_PATH).ok_or(BootstrapError::InvalidEndpoint)?;
    if endpoint.scheme() == "https" {
        return Ok((endpoint, false));
    }
    if allow_development_http
        && endpoint.scheme() == "http"
        && endpoint_has_numeric_loopback(&endpoint)
    {
        return Ok((endpoint, true));
    }
    Err(BootstrapError::InvalidEndpoint)
}

fn verify_bootstrap(
    body: &[u8],
    roots: &HashMap<String, VerifyingKey>,
    now: u64,
    development_http: bool,
    require_current: bool,
) -> Result<RelayBootstrap, BootstrapError> {
    if body.is_empty() || body.len() > MAX_RESPONSE_BYTES {
        return Err(BootstrapError::InvalidResponse);
    }
    let envelope: BootstrapEnvelope =
        serde_json::from_slice(body).map_err(|_| BootstrapError::InvalidResponse)?;
    if serde_json::to_vec(&envelope).map_err(|_| BootstrapError::InvalidResponse)? != body {
        return Err(BootstrapError::InvalidResponse);
    }
    if envelope.version != BOOTSTRAP_VERSION || !valid_key_id(&envelope.kid) {
        return Err(BootstrapError::InvalidResponse);
    }
    let root = roots
        .get(&envelope.kid)
        .ok_or(BootstrapError::UnknownRoot)?;
    let signature = decode_fixed::<64>(&envelope.signature)?;
    let payload_bytes = decode_bounded(&envelope.payload, MAX_PAYLOAD_BYTES)?;
    let mut signing_bytes = Vec::with_capacity(BOOTSTRAP_CONTEXT.len() + payload_bytes.len());
    signing_bytes.extend_from_slice(BOOTSTRAP_CONTEXT);
    signing_bytes.extend_from_slice(&payload_bytes);
    root.verify_strict(&signing_bytes, &Signature::from_bytes(&signature))
        .map_err(|_| BootstrapError::InvalidSignature)?;
    let payload: BootstrapPayload =
        serde_json::from_slice(&payload_bytes).map_err(|_| BootstrapError::InvalidPayload)?;
    if serde_json::to_vec(&payload).map_err(|_| BootstrapError::InvalidPayload)? != payload_bytes {
        return Err(BootstrapError::InvalidPayload);
    }
    validate_payload(&payload, now, require_current)?;
    let mut seen_regions = HashSet::new();
    let mut regions = Vec::with_capacity(payload.regions.len());
    for wire in payload.regions {
        let region = parse_region(&wire.region)?;
        if !seen_regions.insert(region) {
            return Err(BootstrapError::InvalidPayload);
        }
        regions.push(validate_region(wire, region, development_http)?);
    }
    Ok(RelayBootstrap {
        generation: payload.generation,
        signed_payload: payload_bytes.into_boxed_slice(),
        regions,
    })
}

fn validate_payload(
    payload: &BootstrapPayload,
    now: u64,
    require_current: bool,
) -> Result<(), BootstrapError> {
    if payload.version != BOOTSTRAP_VERSION
        || payload.generation == 0
        || payload.generation > MAX_SAFE_INTEGER
        || payload.regions.len() != 2
        || payload.regions[0].region != "cn"
        || payload.regions[1].region != "global"
        || payload.not_before_unix == 0
        || payload.not_before_unix > MAX_UNIX_SECOND
        || payload.not_after_unix > MAX_UNIX_SECOND
        || payload.not_after_unix <= payload.not_before_unix
        || payload.not_after_unix - payload.not_before_unix > MAX_VALIDITY_SECS
        || payload.regions[0].relay_url == payload.regions[1].relay_url
        || payload.regions[0].locator_url == payload.regions[1].locator_url
    {
        return Err(BootstrapError::InvalidPayload);
    }
    validate_global_key_registry(&payload.regions, |region| &region.locator_keys)?;
    validate_global_key_registry(&payload.regions, |region| &region.relay_x25519_keys)?;
    if require_current
        && (payload.not_before_unix > now.saturating_add(MAX_CLOCK_SKEW_SECS)
            || payload.not_after_unix <= now)
    {
        return Err(BootstrapError::Expired);
    }
    Ok(())
}

fn validate_region(
    wire: BootstrapRegion,
    region: RelayRegion,
    development_http: bool,
) -> Result<RelayBootstrapRegion, BootstrapError> {
    let parsed_relay = bootstrap_url::parse(&wire.relay_url, "/v1/tunnel")
        .ok_or(BootstrapError::InvalidPayload)?;
    let relay_endpoint =
        RelayEndpoint::parse(&wire.relay_url).map_err(|_| BootstrapError::InvalidPayload)?;
    if parsed_relay.scheme() != "wss"
        && !(development_http
            && parsed_relay.scheme() == "ws"
            && endpoint_has_numeric_loopback(&parsed_relay))
    {
        return Err(BootstrapError::InvalidPayload);
    }
    validate_locator_url(&wire.locator_url, development_http)?;
    let locator_verifier = locator_verifier(wire.locator_keys)?;
    let relay_x25519_keys = Arc::new(relay_x25519_keys(wire.relay_x25519_keys)?);
    Ok(RelayBootstrapRegion {
        region,
        relay_endpoint,
        locator_url: wire.locator_url,
        locator_verifier,
        relay_x25519_keys,
        #[cfg(any(test, debug_assertions))]
        development_http,
    })
}

fn locator_verifier(keys: Vec<BootstrapKey>) -> Result<Ed25519LocatorVerifier, BootstrapError> {
    if keys.is_empty() || keys.len() > MAX_REGION_KEYS || !strictly_sorted_kids(&keys) {
        return Err(BootstrapError::InvalidPayload);
    }
    let mut parsed = HashMap::with_capacity(keys.len());
    for key in keys {
        if !valid_key_id(&key.kid) {
            return Err(BootstrapError::InvalidPayload);
        }
        LocatorKeyId::new(key.kid.clone()).map_err(|_| BootstrapError::InvalidPayload)?;
        let bytes = decode_fixed::<32>(&key.x)?;
        let key_value = canonical_ed25519_key(bytes, BootstrapError::InvalidPayload)?;
        if parsed.insert(key.kid, key_value).is_some() {
            return Err(BootstrapError::InvalidPayload);
        }
    }
    Ok(Ed25519LocatorVerifier { keys: parsed })
}

fn canonical_ed25519_key(
    bytes: [u8; 32],
    error: BootstrapError,
) -> Result<VerifyingKey, BootstrapError> {
    let mut y = bytes;
    y[31] &= 0x7f;
    if !canonical_curve25519_field_element(&y) {
        return Err(error);
    }
    let key = VerifyingKey::from_bytes(&bytes).map_err(|_| error)?;
    (!key.is_weak()).then_some(key).ok_or(error)
}

fn relay_x25519_keys(keys: Vec<BootstrapKey>) -> Result<PinnedRelayX25519Keys, BootstrapError> {
    if keys.is_empty()
        || keys.len() > MAX_REGION_KEYS
        || keys.len() > MAX_PINNED_RELAY_X25519_KEYS
        || !strictly_sorted_kids(&keys)
    {
        return Err(BootstrapError::InvalidPayload);
    }
    let mut seen = HashSet::with_capacity(keys.len());
    let mut parsed = Vec::with_capacity(keys.len());
    for key in keys {
        if !valid_relay_key_id(&key.kid) || !seen.insert(key.kid.clone()) {
            return Err(BootstrapError::InvalidPayload);
        }
        let kid = RelayChallengeKeyId::new(key.kid).map_err(|_| BootstrapError::InvalidPayload)?;
        let public_key = decode_fixed::<32>(&key.x)?;
        if !canonical_curve25519_field_element(&public_key) {
            return Err(BootstrapError::InvalidPayload);
        }
        let probe = DeviceStaticKey::from_private([0x42; 32])
            .map_err(|_| BootstrapError::InvalidPayload)?;
        probe
            .agree_x25519(&public_key)
            .map_err(|_| BootstrapError::InvalidPayload)?;
        parsed.push(
            RelayServerX25519PublicKey::new(kid, public_key)
                .map_err(|_| BootstrapError::InvalidPayload)?,
        );
    }
    PinnedRelayX25519Keys::new(parsed).map_err(|_| BootstrapError::InvalidPayload)
}

fn strictly_sorted_kids(keys: &[BootstrapKey]) -> bool {
    keys.windows(2)
        .all(|pair| pair[0].kid.as_bytes() < pair[1].kid.as_bytes())
}

fn validate_global_key_registry(
    regions: &[BootstrapRegion],
    select: fn(&BootstrapRegion) -> &[BootstrapKey],
) -> Result<(), BootstrapError> {
    let mut seen_kids = HashSet::<&str>::new();
    let mut seen_public_keys = HashSet::<&str>::new();
    for key in regions.iter().flat_map(select) {
        if !seen_kids.insert(key.kid.as_str()) || !seen_public_keys.insert(key.x.as_str()) {
            return Err(BootstrapError::InvalidPayload);
        }
    }
    Ok(())
}

fn canonical_curve25519_field_element(public_key: &[u8; 32]) -> bool {
    // Both Edwards and Montgomery decoders reduce/mask for compatibility.
    // Signed bootstrap pins require the unique encoding below 2^255 - 19.
    const FIELD_MODULUS: [u8; 32] = [
        0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ];
    for index in (0..public_key.len()).rev() {
        match public_key[index].cmp(&FIELD_MODULUS[index]) {
            std::cmp::Ordering::Less => return true,
            std::cmp::Ordering::Greater => return false,
            std::cmp::Ordering::Equal => {}
        }
    }
    false
}

fn validate_locator_url(value: &str, development_http: bool) -> Result<(), BootstrapError> {
    let endpoint = bootstrap_url::parse(
        value,
        op_collab_relay_control_plane::RELAY_LOCATOR_PUBLISH_PATH,
    )
    .ok_or(BootstrapError::InvalidPayload)?;
    let secure = endpoint.scheme() == "https";
    let development =
        development_http && endpoint.scheme() == "http" && endpoint_has_numeric_loopback(&endpoint);
    (secure || development)
        .then_some(())
        .ok_or(BootstrapError::InvalidPayload)
}

fn parse_region(value: &str) -> Result<RelayRegion, BootstrapError> {
    match value {
        "cn" => Ok(RelayRegion::Cn),
        "global" => Ok(RelayRegion::Global),
        _ => Err(BootstrapError::InvalidPayload),
    }
}

fn reject_rollback(
    previous: &RelayBootstrap,
    current: &RelayBootstrap,
) -> Result<(), BootstrapError> {
    if current.generation < previous.generation
        || (current.generation == previous.generation
            && current.signed_payload != previous.signed_payload)
    {
        return Err(BootstrapError::Rollback);
    }
    Ok(())
}

fn builtin_roots() -> Result<HashMap<String, VerifyingKey>, BootstrapError> {
    let bytes = decode_fixed::<32>(BUILTIN_ROOT_X)?;
    let key = canonical_ed25519_key(bytes, BootstrapError::InvalidRoot)?;
    Ok(HashMap::from([(BUILTIN_ROOT_KID.to_owned(), key)]))
}

#[cfg(any(test, debug_assertions))]
fn add_development_roots(roots: &mut HashMap<String, VerifyingKey>) -> Result<(), BootstrapError> {
    let Some(raw) = std::env::var(BOOTSTRAP_DEV_ROOT_KEYS_ENV).ok() else {
        return Ok(());
    };
    if raw.is_empty() || raw.len() > MAX_ENV_BYTES {
        return Err(BootstrapError::InvalidRoot);
    }
    for entry in raw.split([',', ';']) {
        let (kid, encoded) = entry.split_once('=').ok_or(BootstrapError::InvalidRoot)?;
        if !valid_key_id(kid) || roots.len() >= MAX_ROOT_KEYS {
            return Err(BootstrapError::InvalidRoot);
        }
        let bytes = decode_fixed::<32>(encoded)?;
        let key = canonical_ed25519_key(bytes, BootstrapError::InvalidRoot)?;
        if roots.insert(kid.to_owned(), key).is_some() {
            return Err(BootstrapError::InvalidRoot);
        }
    }
    Ok(())
}

#[cfg(not(any(test, debug_assertions)))]
fn add_development_roots(_roots: &mut HashMap<String, VerifyingKey>) -> Result<(), BootstrapError> {
    Ok(())
}

fn decode_fixed<const N: usize>(encoded: &str) -> Result<[u8; N], BootstrapError> {
    if encoded.is_empty() || encoded.contains('=') {
        return Err(BootstrapError::InvalidBase64);
    }
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| BootstrapError::InvalidBase64)?;
    if URL_SAFE_NO_PAD.encode(&decoded) != encoded {
        return Err(BootstrapError::InvalidBase64);
    }
    decoded
        .try_into()
        .map_err(|_| BootstrapError::InvalidBase64)
}

fn decode_bounded(encoded: &str, maximum: usize) -> Result<Vec<u8>, BootstrapError> {
    if encoded.is_empty() || encoded.contains('=') || encoded.len() > MAX_RESPONSE_BYTES {
        return Err(BootstrapError::InvalidBase64);
    }
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| BootstrapError::InvalidBase64)?;
    if decoded.is_empty() || decoded.len() > maximum || URL_SAFE_NO_PAD.encode(&decoded) != encoded
    {
        return Err(BootstrapError::InvalidBase64);
    }
    Ok(decoded)
}

fn valid_key_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_KEY_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_relay_key_id(value: &str) -> bool {
    value.len() <= MAX_RELAY_KEY_ID_BYTES && valid_key_id(value)
}

fn endpoint_has_numeric_loopback(endpoint: &Url) -> bool {
    endpoint
        .host_str()
        .map(|host| host.trim_matches(['[', ']']))
        .and_then(|host| host.parse::<IpAddr>().ok())
        .is_some_and(|address| address.is_loopback())
}

#[cfg(any(test, debug_assertions))]
fn development_http_enabled() -> bool {
    std::env::var(BOOTSTRAP_DEV_HTTP_ENV).ok().as_deref() == Some("1")
}

#[cfg(not(any(test, debug_assertions)))]
const fn development_http_enabled() -> bool {
    false
}

fn validate_content_type(headers: &HeaderMap) -> Result<(), BootstrapError> {
    let value = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or(BootstrapError::InvalidResponse)?;
    let valid = value.eq_ignore_ascii_case("application/json; charset=utf-8");
    valid.then_some(()).ok_or(BootstrapError::InvalidResponse)
}

fn response_etag(headers: &HeaderMap, body: &[u8]) -> Result<String, BootstrapError> {
    let value = headers
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .ok_or(BootstrapError::InvalidResponse)?;
    let expected = strong_etag(body);
    (value == expected)
        .then_some(expected)
        .ok_or(BootstrapError::InvalidResponse)
}

fn validate_not_modified(
    headers: &HeaderMap,
    cached: Option<&BootstrapCache>,
) -> Result<(), BootstrapError> {
    let cached = cached.ok_or(BootstrapError::InvalidResponse)?;
    let expected = cached
        .etag
        .as_deref()
        .ok_or(BootstrapError::InvalidResponse)?;
    let actual = headers
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .ok_or(BootstrapError::InvalidResponse)?;
    (actual == expected)
        .then_some(())
        .ok_or(BootstrapError::InvalidResponse)
}

fn strong_etag(body: &[u8]) -> String {
    let digest = Sha256::digest(body);
    let mut etag = String::with_capacity(66);
    etag.push('"');
    for byte in digest {
        use std::fmt::Write as _;
        write!(etag, "{byte:02x}").expect("writing to String cannot fail");
    }
    etag.push('"');
    etag
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BootstrapEnvelope {
    version: u64,
    kid: String,
    payload: String,
    signature: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BootstrapPayload {
    version: u64,
    generation: u64,
    not_before_unix: u64,
    not_after_unix: u64,
    regions: Vec<BootstrapRegion>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BootstrapRegion {
    region: String,
    relay_url: String,
    locator_url: String,
    locator_keys: Vec<BootstrapKey>,
    relay_x25519_keys: Vec<BootstrapKey>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BootstrapKey {
    kid: String,
    x: String,
}

fn relay_runtime_error() -> CollabRuntimeError {
    CollabRuntimeError::new(CollabRuntimeFailure::RelayUnavailable)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BootstrapError {
    Cache,
    CachePersist,
    Expired,
    InvalidBase64,
    InvalidEndpoint,
    InvalidPayload,
    InvalidResponse,
    InvalidRoot,
    InvalidSignature,
    ResponseTooLarge,
    Rollback,
    Transport,
    UnknownRoot,
}

#[cfg(test)]
#[path = "relay_bootstrap_tests.rs"]
mod tests;
