use std::io;

use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelayBridgePhase {
    Starting,
    Waiting,
    Active,
    Degraded,
    Stopped,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelayBridgeStatus {
    pub phase: RelayBridgePhase,
    pub waiting_lanes: usize,
    pub active_tunnels: usize,
    pub last_error: Option<RelayFailureKind>,
}

impl RelayBridgeStatus {
    pub(crate) const fn starting(waiting_lanes: usize) -> Self {
        Self {
            phase: RelayBridgePhase::Starting,
            waiting_lanes,
            active_tunnels: 0,
            last_error: None,
        }
    }

    pub(crate) const fn stopped() -> Self {
        Self {
            phase: RelayBridgePhase::Stopped,
            waiting_lanes: 0,
            active_tunnels: 0,
            last_error: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelayFailureKind {
    Authentication,
    Connect,
    ConnectTimeout,
    HelloTimeout,
    PairTimeout,
    Rejected,
    Protocol,
    TextFrame,
    BinaryFrameTooLarge,
    IdleTimeout,
    LifetimeExceeded,
    ByteLimitExceeded,
    LocalIo,
    RelayIo,
    Closed,
}

#[derive(Debug, Error)]
pub enum RelayClientError {
    #[error("owner relay lane count must be between 1 and {max}")]
    InvalidLaneCount { max: usize },
    #[error("local collaboration socket must be a nonzero loopback address")]
    InvalidLocalSocket,
    #[error("unauthenticated development relay must use a numeric loopback endpoint")]
    DevelopmentEndpointNotLoopback,
    #[error("relay did not accept an owner lane before the readiness deadline")]
    ReadyTimeout,
    #[error("relay owner bridge stopped before a lane became ready")]
    StoppedBeforeReady,
    #[error("failed to bind the guest loopback bridge")]
    BindLoopback { kind: io::ErrorKind },
    #[error("failed to read the guest loopback bridge address")]
    ReadLocalAddress { kind: io::ErrorKind },
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum RelayStopError {
    #[error("relay bridge did not stop before its deadline")]
    Timeout,
    #[error("relay bridge task failed")]
    TaskFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TunnelError {
    Cancelled,
    Failure(RelayFailureKind),
}

impl TunnelError {
    pub(crate) fn failure_kind(self) -> Option<RelayFailureKind> {
        match self {
            Self::Cancelled => None,
            Self::Failure(kind) => Some(kind),
        }
    }
}
