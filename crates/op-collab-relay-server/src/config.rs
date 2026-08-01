use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    num::ParseIntError,
    time::Duration,
};

const LISTEN_ENV: &str = "OPENPENCIL_COLLAB_RELAY_LISTEN";
const MAX_PENDING_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_PENDING";
const MAX_PENDING_PER_SOURCE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_PENDING_PER_SOURCE";
const MAX_AUTH_IN_FLIGHT_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_AUTH_IN_FLIGHT";
const MAX_REAUTH_IN_FLIGHT_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_REAUTH_IN_FLIGHT";
const MAX_ACTIVE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_ACTIVE";
const MAX_WAITING_PER_ROUTE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_WAITING_PER_ROUTE";
const RELAY_QUEUE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_QUEUE_CAPACITY";
const MAX_QUEUED_BYTES_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_QUEUED_BYTES";
const MAX_QUEUED_BYTES_PER_ROUTE_ENV: &str = "OPENPENCIL_COLLAB_RELAY_MAX_QUEUED_BYTES_PER_ROUTE";

#[derive(Clone, Debug)]
pub struct RelayConfig {
    pub listen: SocketAddr,
    pub handshake_timeout: Duration,
    pub waiting_timeout: Duration,
    pub idle_timeout: Duration,
    pub tunnel_lifetime: Duration,
    pub max_message_bytes: usize,
    pub max_pending: usize,
    /// How many un-paired connections one source address may hold at once.
    pub max_pending_per_source: usize,
    pub max_auth_in_flight: usize,
    /// Renewal budget for already-authenticated tunnels.
    ///
    /// Kept separate from `max_auth_in_flight` so a flood of unauthenticated
    /// connections cannot starve the reauthentication of live sessions, which
    /// closes them with a policy error when it cannot complete in time.
    pub max_reauth_in_flight: usize,
    pub max_active_pairs: usize,
    pub max_waiting_per_route: usize,
    pub relay_queue_capacity: usize,
    pub max_queued_bytes: usize,
    pub max_queued_bytes_per_route: usize,
    unauthenticated_dev: bool,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            listen: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8091),
            handshake_timeout: Duration::from_secs(5),
            waiting_timeout: Duration::from_secs(60),
            idle_timeout: Duration::from_secs(90),
            tunnel_lifetime: Duration::from_secs(12 * 60 * 60),
            max_message_bytes: 64 * 1024,
            max_pending: 1_024,
            max_pending_per_source: 16,
            max_auth_in_flight: 128,
            max_reauth_in_flight: 128,
            max_active_pairs: 10_000,
            max_waiting_per_route: 4,
            relay_queue_capacity: 32,
            max_queued_bytes: 64 * 1024 * 1024,
            max_queued_bytes_per_route: 1024 * 1024,
            unauthenticated_dev: false,
        }
    }
}

impl RelayConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();
        if let Some(value) = env::var_os(LISTEN_ENV) {
            let value = value
                .into_string()
                .map_err(|_| ConfigError::NonUnicode(LISTEN_ENV))?;
            config.listen = value
                .parse()
                .map_err(|source| ConfigError::Listen { value, source })?;
        }
        config.max_pending = parse_positive_usize(MAX_PENDING_ENV, config.max_pending)?;
        config.max_pending_per_source =
            parse_positive_usize(MAX_PENDING_PER_SOURCE_ENV, config.max_pending_per_source)?;
        config.max_auth_in_flight =
            parse_positive_usize(MAX_AUTH_IN_FLIGHT_ENV, config.max_auth_in_flight)?;
        config.max_reauth_in_flight =
            parse_positive_usize(MAX_REAUTH_IN_FLIGHT_ENV, config.max_reauth_in_flight)?;
        config.max_active_pairs = parse_positive_usize(MAX_ACTIVE_ENV, config.max_active_pairs)?;
        config.max_waiting_per_route =
            parse_positive_usize(MAX_WAITING_PER_ROUTE_ENV, config.max_waiting_per_route)?;
        config.relay_queue_capacity =
            parse_positive_usize(RELAY_QUEUE_ENV, config.relay_queue_capacity)?;
        config.max_queued_bytes =
            parse_positive_usize(MAX_QUEUED_BYTES_ENV, config.max_queued_bytes)?;
        config.max_queued_bytes_per_route = parse_positive_usize(
            MAX_QUEUED_BYTES_PER_ROUTE_ENV,
            config.max_queued_bytes_per_route,
        )?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        for (name, value) in [
            ("max_message_bytes", self.max_message_bytes),
            ("max_pending", self.max_pending),
            ("max_pending_per_source", self.max_pending_per_source),
            ("max_auth_in_flight", self.max_auth_in_flight),
            ("max_reauth_in_flight", self.max_reauth_in_flight),
            ("max_active_pairs", self.max_active_pairs),
            ("max_waiting_per_route", self.max_waiting_per_route),
            ("relay_queue_capacity", self.relay_queue_capacity),
            ("max_queued_bytes", self.max_queued_bytes),
            (
                "max_queued_bytes_per_route",
                self.max_queued_bytes_per_route,
            ),
        ] {
            if value == 0 {
                return Err(ConfigError::Zero(name));
            }
        }
        if self.max_message_bytes > 64 * 1024 {
            return Err(ConfigError::TooLarge {
                name: "max_message_bytes",
                maximum: 64 * 1024,
            });
        }
        if self.max_queued_bytes > u32::MAX as usize {
            return Err(ConfigError::TooLarge {
                name: "max_queued_bytes",
                maximum: u32::MAX as usize,
            });
        }
        if self.max_queued_bytes_per_route > self.max_queued_bytes {
            return Err(ConfigError::RouteBudgetExceedsGlobal);
        }
        if self.max_pending_per_source > self.max_pending {
            return Err(ConfigError::SourceBudgetExceedsGlobal);
        }
        for (name, value) in [
            ("handshake_timeout", self.handshake_timeout),
            ("waiting_timeout", self.waiting_timeout),
            ("idle_timeout", self.idle_timeout),
            ("tunnel_lifetime", self.tunnel_lifetime),
        ] {
            if value.is_zero() {
                return Err(ConfigError::ZeroDuration(name));
            }
        }
        Ok(())
    }

    /// Enable capability-only pairing for local development and tests.
    ///
    /// This mode proves neither possession of a collaboration ticket nor a
    /// device key and therefore must never be exposed as a production relay.
    pub fn allow_unauthenticated_dev(mut self) -> Self {
        self.unauthenticated_dev = true;
        self
    }

    pub(crate) fn unauthenticated_dev(&self) -> bool {
        self.unauthenticated_dev
    }
}

fn parse_positive_usize(name: &'static str, default: usize) -> Result<usize, ConfigError> {
    let Some(value) = env::var_os(name) else {
        return Ok(default);
    };
    let value = value
        .into_string()
        .map_err(|_| ConfigError::NonUnicode(name))?;
    value.parse().map_err(|source| ConfigError::Number {
        name,
        value,
        source,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} is not valid Unicode")]
    NonUnicode(&'static str),
    #[error("{name} must be a positive integer, got {value}")]
    Number {
        name: &'static str,
        value: String,
        #[source]
        source: ParseIntError,
    },
    #[error("{LISTEN_ENV} must be an IP socket address, got {value}")]
    Listen {
        value: String,
        #[source]
        source: std::net::AddrParseError,
    },
    #[error("{0} must be greater than zero")]
    Zero(&'static str),
    #[error("{0} must be a non-zero duration")]
    ZeroDuration(&'static str),
    #[error("{name} exceeds the supported maximum of {maximum}")]
    TooLarge { name: &'static str, maximum: usize },
    #[error("max_queued_bytes_per_route must not exceed max_queued_bytes")]
    RouteBudgetExceedsGlobal,
    #[error("max_pending_per_source must not exceed max_pending")]
    SourceBudgetExceedsGlobal,
}
