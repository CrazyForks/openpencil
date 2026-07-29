use std::{
    fmt,
    net::SocketAddr,
    num::{NonZeroU32, NonZeroUsize},
    time::Duration,
};

pub const DEFAULT_LOCATOR_LISTEN: &str = "127.0.0.1:8092";
pub const MAX_CONFIGURED_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct LocatorHttpLimits {
    pub max_connections: NonZeroUsize,
    pub max_in_flight: NonZeroUsize,
    pub max_auth_in_flight: NonZeroUsize,
    pub max_requests_per_second: NonZeroU32,
    pub header_timeout: Duration,
    pub body_timeout: Duration,
    pub auth_timeout: Duration,
    pub shutdown_grace: Duration,
}

impl Default for LocatorHttpLimits {
    fn default() -> Self {
        Self {
            max_connections: NonZeroUsize::new(256).expect("non-zero"),
            max_in_flight: NonZeroUsize::new(64).expect("non-zero"),
            max_auth_in_flight: NonZeroUsize::new(16).expect("non-zero"),
            max_requests_per_second: NonZeroU32::new(100).expect("non-zero"),
            header_timeout: Duration::from_secs(5),
            body_timeout: Duration::from_secs(5),
            auth_timeout: Duration::from_secs(5),
            shutdown_grace: Duration::from_secs(10),
        }
    }
}

impl LocatorHttpLimits {
    pub fn validate(&self) -> Result<(), LocatorServerConfigError> {
        for (field, timeout) in [
            ("header_timeout", self.header_timeout),
            ("body_timeout", self.body_timeout),
            ("auth_timeout", self.auth_timeout),
            ("shutdown_grace", self.shutdown_grace),
        ] {
            if timeout.is_zero() || timeout > MAX_CONFIGURED_TIMEOUT {
                return Err(LocatorServerConfigError::InvalidTimeout { field });
            }
        }
        if self.max_auth_in_flight > self.max_in_flight {
            return Err(LocatorServerConfigError::InvalidConcurrency);
        }
        Ok(())
    }
}

impl fmt::Debug for LocatorHttpLimits {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocatorHttpLimits")
            .field("max_connections", &self.max_connections)
            .field("max_in_flight", &self.max_in_flight)
            .field("max_auth_in_flight", &self.max_auth_in_flight)
            .field("max_requests_per_second", &self.max_requests_per_second)
            .field("header_timeout", &self.header_timeout)
            .field("body_timeout", &self.body_timeout)
            .field("auth_timeout", &self.auth_timeout)
            .field("shutdown_grace", &self.shutdown_grace)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct LocatorServerConfig {
    pub listen: SocketAddr,
    pub limits: LocatorHttpLimits,
}

impl LocatorServerConfig {
    pub fn new(
        listen: SocketAddr,
        limits: LocatorHttpLimits,
    ) -> Result<Self, LocatorServerConfigError> {
        limits.validate()?;
        Ok(Self { listen, limits })
    }
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum LocatorServerConfigError {
    #[error("locator server timeout {field} is outside the allowed range")]
    InvalidTimeout { field: &'static str },
    #[error("locator authentication concurrency exceeds total request concurrency")]
    InvalidConcurrency,
}
