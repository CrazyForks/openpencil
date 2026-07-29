use std::time::Duration;

pub const DEFAULT_OWNER_LANE_COUNT: usize = 4;
pub const MAX_OWNER_LANE_COUNT: usize = 8;
pub const MAX_RELAY_BINARY_BYTES: usize = 64 * 1024;
pub const MAX_RELAY_CONNECTION_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(crate) struct RelayLimits {
    pub connect: Duration,
    pub hello: Duration,
    pub pair: Duration,
    pub idle: Duration,
    pub lifetime: Duration,
    pub retry: Duration,
    pub stop: Duration,
    pub max_binary_bytes: usize,
    pub max_connection_bytes: u64,
}

impl Default for RelayLimits {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(10),
            hello: Duration::from_secs(10),
            pair: Duration::from_secs(5 * 60),
            idle: Duration::from_secs(2 * 60),
            lifetime: Duration::from_secs(24 * 60 * 60),
            retry: Duration::from_secs(1),
            stop: Duration::from_secs(5),
            max_binary_bytes: MAX_RELAY_BINARY_BYTES,
            max_connection_bytes: MAX_RELAY_CONNECTION_BYTES,
        }
    }
}

impl std::fmt::Debug for RelayLimits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelayLimits")
            .field("connect", &self.connect)
            .field("hello", &self.hello)
            .field("pair", &self.pair)
            .field("idle", &self.idle)
            .field("lifetime", &self.lifetime)
            .field("retry", &self.retry)
            .field("stop", &self.stop)
            .field("max_binary_bytes", &self.max_binary_bytes)
            .field("max_connection_bytes", &self.max_connection_bytes)
            .finish()
    }
}
