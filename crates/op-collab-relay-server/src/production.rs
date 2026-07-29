use std::{
    env,
    ffi::OsString,
    fmt,
    num::NonZeroU64,
    path::{Path, PathBuf},
    sync::Arc,
};

use op_auth_bridge::{
    CollabJwksCacheLimits, CollabJwksFetchError, CollabTicketVerifier, CollabVerifierConfig,
    CollabVerifierConfigError, DEFAULT_MAX_COLLAB_JWKS_BYTES,
};
use op_collab_relay_protocol::RelayRegion;

use crate::{
    run_with_authenticator, CollabTicketRelayAuthenticator, PinnedEd25519LocatorVerifier,
    PinnedPolicyFileFetcher, PinnedVerifierError, PinnedX25519KeyError, PinnedX25519ProofBoundary,
    RelayConfig, RelayServerError,
};

pub const HOME_REGION_ENV: &str = "OPENPENCIL_COLLAB_RELAY_HOME_REGION";
pub const TICKET_POLICY_FILE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_TICKET_POLICY_FILE";
pub const LOCATOR_KEYS_FILE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_LOCATOR_KEYS_FILE";
pub const RELAY_X25519_KEYS_FILE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_X25519_KEYS_FILE";
pub const POLICY_MAX_AGE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_POLICY_MAX_AGE_SECONDS";

const DEFAULT_POLICY_MAX_AGE_SECONDS: u64 = 60;
const MAX_POLICY_MAX_AGE_SECONDS: u64 = 60 * 60;

pub struct ProductionRelayAuthConfig {
    home_region: RelayRegion,
    ticket_policy_file: PathBuf,
    locator_keys_file: PathBuf,
    relay_x25519_keys_file: Option<PathBuf>,
    policy_max_age_seconds: NonZeroU64,
    allow_ticket_binding_only: bool,
}

impl ProductionRelayAuthConfig {
    pub fn from_env(
        allow_ticket_binding_only: bool,
    ) -> Result<Self, ProductionRelayAuthConfigError> {
        let home_region = match required_unicode(HOME_REGION_ENV)?.as_str() {
            "cn" => RelayRegion::Cn,
            "global" => RelayRegion::Global,
            _ => return Err(ProductionRelayAuthConfigError::InvalidRegion),
        };
        let ticket_policy_file = required_absolute_path(TICKET_POLICY_FILE_ENV)?;
        let locator_keys_file = required_absolute_path(LOCATOR_KEYS_FILE_ENV)?;
        let relay_x25519_keys_file = optional_absolute_path(RELAY_X25519_KEYS_FILE_ENV)?;
        let policy_max_age_seconds = match env::var_os(POLICY_MAX_AGE_ENV) {
            Some(value) => {
                let value = value
                    .into_string()
                    .map_err(|_| ProductionRelayAuthConfigError::NonUnicode(POLICY_MAX_AGE_ENV))?;
                value
                    .parse::<u64>()
                    .ok()
                    .and_then(NonZeroU64::new)
                    .filter(|value| value.get() <= MAX_POLICY_MAX_AGE_SECONDS)
                    .ok_or(ProductionRelayAuthConfigError::InvalidPolicyMaxAge)?
            }
            None => NonZeroU64::new(DEFAULT_POLICY_MAX_AGE_SECONDS)
                .expect("default policy max age is non-zero"),
        };
        Self::new(
            home_region,
            ticket_policy_file,
            locator_keys_file,
            relay_x25519_keys_file,
            policy_max_age_seconds,
            allow_ticket_binding_only,
        )
    }

    pub fn new(
        home_region: RelayRegion,
        ticket_policy_file: PathBuf,
        locator_keys_file: PathBuf,
        relay_x25519_keys_file: Option<PathBuf>,
        policy_max_age_seconds: NonZeroU64,
        allow_ticket_binding_only: bool,
    ) -> Result<Self, ProductionRelayAuthConfigError> {
        if policy_max_age_seconds.get() > MAX_POLICY_MAX_AGE_SECONDS {
            return Err(ProductionRelayAuthConfigError::InvalidPolicyMaxAge);
        }
        for (name, path) in [
            (TICKET_POLICY_FILE_ENV, Some(ticket_policy_file.as_path())),
            (LOCATOR_KEYS_FILE_ENV, Some(locator_keys_file.as_path())),
            (
                RELAY_X25519_KEYS_FILE_ENV,
                relay_x25519_keys_file.as_deref(),
            ),
        ] {
            if path.is_some_and(|path| !path.is_absolute()) {
                return Err(ProductionRelayAuthConfigError::PathNotAbsolute(name));
            }
        }
        match (relay_x25519_keys_file.is_some(), allow_ticket_binding_only) {
            (false, false) => {
                return Err(ProductionRelayAuthConfigError::ChallengeProofRequired);
            }
            (true, true) => {
                return Err(ProductionRelayAuthConfigError::ConflictingProofPolicy);
            }
            _ => {}
        }
        Ok(Self {
            home_region,
            ticket_policy_file,
            locator_keys_file,
            relay_x25519_keys_file,
            policy_max_age_seconds,
            allow_ticket_binding_only,
        })
    }

    pub const fn home_region(&self) -> RelayRegion {
        self.home_region
    }

    pub const fn reduced_assurance(&self) -> bool {
        self.allow_ticket_binding_only
    }
}

impl fmt::Debug for ProductionRelayAuthConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProductionRelayAuthConfig")
            .field("home_region", &self.home_region)
            .field("ticket_policy_file", &"[REDACTED]")
            .field("locator_keys_file", &"[REDACTED]")
            .field(
                "relay_x25519_keys_file",
                &self.relay_x25519_keys_file.as_ref().map(|_| "[REDACTED]"),
            )
            .field("policy_max_age_seconds", &self.policy_max_age_seconds)
            .field("allow_ticket_binding_only", &self.allow_ticket_binding_only)
            .finish()
    }
}

pub async fn run_production(
    relay_config: RelayConfig,
    auth_config: ProductionRelayAuthConfig,
) -> Result<(), ProductionRelayError> {
    let locator_verifier = PinnedEd25519LocatorVerifier::from_file(&auth_config.locator_keys_file)
        .map_err(ProductionRelayError::LocatorKeys)?;
    let verifier_config = CollabVerifierConfig::production();
    let fetcher = PinnedPolicyFileFetcher::new(
        &verifier_config,
        &auth_config.ticket_policy_file,
        auth_config.policy_max_age_seconds,
    );
    fetcher
        .validate_source(DEFAULT_MAX_COLLAB_JWKS_BYTES)
        .map_err(ProductionRelayError::TicketPolicyFile)?;
    let ticket_verifier =
        CollabTicketVerifier::new(verifier_config, fetcher, CollabJwksCacheLimits::default())?;

    if let Some(path) = auth_config.relay_x25519_keys_file {
        let proof_boundary = PinnedX25519ProofBoundary::from_file(path)
            .map_err(ProductionRelayError::RelayX25519Keys)?;
        let authenticator = CollabTicketRelayAuthenticator::new(
            ticket_verifier,
            auth_config.home_region,
            locator_verifier,
            proof_boundary,
        );
        run_with_authenticator(relay_config, Arc::new(authenticator)).await?;
    } else {
        debug_assert!(auth_config.allow_ticket_binding_only);
        let authenticator = CollabTicketRelayAuthenticator::new_ticket_binding_only(
            ticket_verifier,
            auth_config.home_region,
            locator_verifier,
        );
        run_with_authenticator(relay_config, Arc::new(authenticator)).await?;
    }
    Ok(())
}

fn required_unicode(name: &'static str) -> Result<String, ProductionRelayAuthConfigError> {
    let value = env::var_os(name).ok_or(ProductionRelayAuthConfigError::Missing(name))?;
    let value = value
        .into_string()
        .map_err(|_| ProductionRelayAuthConfigError::NonUnicode(name))?;
    if value.is_empty() {
        return Err(ProductionRelayAuthConfigError::Missing(name));
    }
    Ok(value)
}

fn required_absolute_path(name: &'static str) -> Result<PathBuf, ProductionRelayAuthConfigError> {
    let value = env::var_os(name).ok_or(ProductionRelayAuthConfigError::Missing(name))?;
    absolute_path(name, value)?.ok_or(ProductionRelayAuthConfigError::Missing(name))
}

fn optional_absolute_path(
    name: &'static str,
) -> Result<Option<PathBuf>, ProductionRelayAuthConfigError> {
    env::var_os(name)
        .map(|value| absolute_path(name, value))
        .transpose()
        .map(Option::flatten)
}

fn absolute_path(
    name: &'static str,
    value: OsString,
) -> Result<Option<PathBuf>, ProductionRelayAuthConfigError> {
    if value.is_empty() {
        return Ok(None);
    }
    let path = Path::new(&value);
    if !path.is_absolute() {
        return Err(ProductionRelayAuthConfigError::PathNotAbsolute(name));
    }
    Ok(Some(path.to_path_buf()))
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProductionRelayAuthConfigError {
    #[error("required production relay setting {0} is missing")]
    Missing(&'static str),
    #[error("production relay setting {0} is not valid Unicode")]
    NonUnicode(&'static str),
    #[error("production relay home region must be `cn` or `global`")]
    InvalidRegion,
    #[error("production relay setting {0} must be an absolute path")]
    PathNotAbsolute(&'static str),
    #[error("production relay policy max age must be between 1 and 3600 seconds")]
    InvalidPolicyMaxAge,
    #[error(
        "a relay X25519 key file is required unless \
         --allow-ticket-binding-only is explicitly selected"
    )]
    ChallengeProofRequired,
    #[error(
        "--allow-ticket-binding-only cannot be combined with a configured relay X25519 key file"
    )]
    ConflictingProofPolicy,
}

#[derive(Debug, thiserror::Error)]
pub enum ProductionRelayError {
    #[error("pinned ticket policy file is unavailable or unsafe: {0}")]
    TicketPolicyFile(#[source] CollabJwksFetchError),
    #[error("pinned locator verifier configuration is invalid: {0}")]
    LocatorKeys(#[source] PinnedVerifierError),
    #[error("relay X25519 proof-key configuration is invalid: {0}")]
    RelayX25519Keys(#[source] PinnedX25519KeyError),
    #[error("collaboration ticket verifier configuration is invalid: {0}")]
    TicketVerifier(#[from] CollabVerifierConfigError),
    #[error(transparent)]
    Relay(#[from] RelayServerError),
}
