use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinSet;
use tokio::time::Instant;

use super::protocol::{establish_relay_with_ready_hook, RelayHandshake};
use super::BridgeHandle;
use crate::auth::{AuthMode, RelayAuthenticator, RelayCredentialProvider};
use crate::endpoint::RelayEndpoint;
use crate::error::{
    RelayBridgePhase, RelayBridgeStatus, RelayClientError, RelayFailureKind, RelayStopError,
    TunnelError,
};
use crate::limits::{RelayLimits, DEFAULT_OWNER_LANE_COUNT, MAX_OWNER_LANE_COUNT};
use crate::session::{cancelled, pump, sleep_or_cancel, ClientReauthContext};

/// A bounded pool of owner relay lanes.
pub struct RelayOwnerBridge {
    lane_count: usize,
    handle: BridgeHandle,
}

impl RelayOwnerBridge {
    pub async fn start(
        endpoint: RelayEndpoint,
        route: RelayHandshake,
        local_owner_socket: SocketAddr,
        lane_count: usize,
        authenticator: Arc<dyn RelayAuthenticator>,
    ) -> Result<Self, RelayClientError> {
        Self::start_inner(
            endpoint,
            route,
            local_owner_socket,
            lane_count,
            AuthMode::ChallengeBound(authenticator),
            RelayLimits::default(),
        )
        .await
    }

    pub async fn start_default_lanes(
        endpoint: RelayEndpoint,
        route: RelayHandshake,
        local_owner_socket: SocketAddr,
        authenticator: Arc<dyn RelayAuthenticator>,
    ) -> Result<Self, RelayClientError> {
        Self::start(
            endpoint,
            route,
            local_owner_socket,
            DEFAULT_OWNER_LANE_COUNT,
            authenticator,
        )
        .await
    }

    /// Starts explicit reduced-assurance ticket-to-DH owner lanes.
    pub async fn start_ticket_binding_only(
        endpoint: RelayEndpoint,
        route: RelayHandshake,
        local_owner_socket: SocketAddr,
        lane_count: usize,
        credentials: Arc<dyn RelayCredentialProvider>,
    ) -> Result<Self, RelayClientError> {
        Self::start_inner(
            endpoint,
            route,
            local_owner_socket,
            lane_count,
            AuthMode::TicketBindingOnly(credentials),
            RelayLimits::default(),
        )
        .await
    }

    pub async fn start_default_lanes_ticket_binding_only(
        endpoint: RelayEndpoint,
        route: RelayHandshake,
        local_owner_socket: SocketAddr,
        credentials: Arc<dyn RelayCredentialProvider>,
    ) -> Result<Self, RelayClientError> {
        Self::start_ticket_binding_only(
            endpoint,
            route,
            local_owner_socket,
            DEFAULT_OWNER_LANE_COUNT,
            credentials,
        )
        .await
    }

    /// Starts anonymous relay lanes for local development.
    ///
    /// This API is absent from release builds.
    #[cfg(any(test, debug_assertions))]
    pub async fn start_unauthenticated_for_development(
        endpoint: RelayEndpoint,
        route: RelayHandshake,
        local_owner_socket: SocketAddr,
        lane_count: usize,
    ) -> Result<Self, RelayClientError> {
        if !endpoint.is_numeric_loopback() {
            return Err(RelayClientError::DevelopmentEndpointNotLoopback);
        }
        Self::start_inner(
            endpoint,
            route,
            local_owner_socket,
            lane_count,
            AuthMode::DevelopmentAnonymous,
            RelayLimits::default(),
        )
        .await
    }

    pub fn lane_count(&self) -> usize {
        self.lane_count
    }

    pub fn status(&self) -> RelayBridgeStatus {
        self.handle.status()
    }

    pub fn subscribe(&self) -> watch::Receiver<RelayBridgeStatus> {
        self.handle.subscribe()
    }

    pub async fn stop(self) -> Result<(), RelayStopError> {
        self.handle.stop().await
    }

    /// Wait until at least one owner lane has been authenticated and accepted
    /// by the relay.
    pub async fn wait_until_ready(
        &self,
        timeout: std::time::Duration,
    ) -> Result<(), RelayClientError> {
        let mut status = self.subscribe();
        let ready = async {
            loop {
                match status.borrow().phase {
                    RelayBridgePhase::Waiting | RelayBridgePhase::Active => return Ok(()),
                    RelayBridgePhase::Stopped | RelayBridgePhase::Failed => {
                        return Err(RelayClientError::StoppedBeforeReady);
                    }
                    RelayBridgePhase::Starting | RelayBridgePhase::Degraded => {}
                }
                status
                    .changed()
                    .await
                    .map_err(|_| RelayClientError::StoppedBeforeReady)?;
            }
        };
        tokio::time::timeout(timeout, ready)
            .await
            .map_err(|_| RelayClientError::ReadyTimeout)?
    }

    async fn start_inner(
        endpoint: RelayEndpoint,
        route: RelayHandshake,
        local_owner_socket: SocketAddr,
        lane_count: usize,
        auth: AuthMode,
        limits: RelayLimits,
    ) -> Result<Self, RelayClientError> {
        if !(1..=MAX_OWNER_LANE_COUNT).contains(&lane_count) {
            return Err(RelayClientError::InvalidLaneCount {
                max: MAX_OWNER_LANE_COUNT,
            });
        }
        if !local_owner_socket.ip().is_loopback() || local_owner_socket.port() == 0 {
            return Err(RelayClientError::InvalidLocalSocket);
        }

        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (status_tx, status_rx) = watch::channel(RelayBridgeStatus::starting(lane_count));
        let task = tokio::spawn(run_owner(
            endpoint,
            route,
            local_owner_socket,
            lane_count,
            auth,
            cancel_rx,
            status_tx,
            limits,
        ));
        Ok(Self {
            lane_count,
            handle: BridgeHandle::new(cancel_tx, status_rx, task, limits),
        })
    }

    #[cfg(test)]
    pub(crate) async fn start_test(
        endpoint: RelayEndpoint,
        route: RelayHandshake,
        local_owner_socket: SocketAddr,
        lane_count: usize,
        limits: RelayLimits,
    ) -> Result<Self, RelayClientError> {
        Self::start_inner(
            endpoint,
            route,
            local_owner_socket,
            lane_count,
            AuthMode::DevelopmentAnonymous,
            limits,
        )
        .await
    }
}

impl fmt::Debug for RelayOwnerBridge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RelayOwnerBridge")
            .field("lane_count", &self.lane_count)
            .field("handle", &self.handle)
            .finish()
    }
}

enum LaneEvent {
    Active(oneshot::Sender<()>),
}

#[derive(Clone, Copy, Debug)]
struct LaneReport {
    became_ready: bool,
    became_active: bool,
    result: Result<(), TunnelError>,
}

#[allow(clippy::too_many_arguments)]
async fn run_owner(
    endpoint: RelayEndpoint,
    route: RelayHandshake,
    local_owner_socket: SocketAddr,
    lane_count: usize,
    auth: AuthMode,
    mut cancel: watch::Receiver<bool>,
    status_tx: watch::Sender<RelayBridgeStatus>,
    limits: RelayLimits,
) {
    let (event_tx, mut event_rx) = mpsc::channel(lane_count);
    let (ready_tx, mut ready_rx) = mpsc::channel(lane_count);
    let mut lanes = JoinSet::new();
    for _ in 0..lane_count {
        spawn_lane(
            &mut lanes,
            endpoint.clone(),
            route.clone(),
            local_owner_socket,
            auth.clone(),
            cancel.clone(),
            event_tx.clone(),
            ready_tx.clone(),
            limits,
            false,
        );
    }

    let mut waiting = 0_usize;
    let mut active = 0_usize;
    let mut last_error = None;
    publish_owner(&status_tx, waiting, active, last_error);

    loop {
        tokio::select! {
            biased;
            _ = cancelled(&mut cancel) => break,
            ready = ready_rx.recv() => {
                if ready.is_some() {
                    waiting = waiting.saturating_add(1).min(lane_count);
                    last_error = None;
                    publish_owner(&status_tx, waiting, active, last_error);
                }
            }
            event = event_rx.recv() => {
                if let Some(LaneEvent::Active(acknowledge)) = event {
                    waiting = waiting.saturating_sub(1);
                    active = active.saturating_add(1).min(lane_count);
                    last_error = None;
                    publish_owner(&status_tx, waiting, active, last_error);
                    let _ = acknowledge.send(());
                }
            }
            completed = lanes.join_next() => {
                let Some(completed) = completed else {
                    break;
                };
                let report = match completed {
                    Ok(report) => report,
                    Err(_) => LaneReport {
                        became_ready: false,
                        became_active: false,
                        result: Err(TunnelError::Failure(RelayFailureKind::RelayIo)),
                    },
                };
                if report.became_active {
                    active = active.saturating_sub(1);
                } else if report.became_ready {
                    waiting = waiting.saturating_sub(1);
                }
                if let Err(error) = report.result {
                    last_error = error.failure_kind();
                }
                publish_owner(&status_tx, waiting, active, last_error);
                spawn_lane(
                    &mut lanes,
                    endpoint.clone(),
                    route.clone(),
                    local_owner_socket,
                    auth.clone(),
                    cancel.clone(),
                    event_tx.clone(),
                    ready_tx.clone(),
                    limits,
                    true,
                );
            }
        }
    }
    lanes.abort_all();
    while lanes.join_next().await.is_some() {}
    status_tx.send_replace(RelayBridgeStatus::stopped());
}

#[allow(clippy::too_many_arguments)]
fn spawn_lane(
    lanes: &mut JoinSet<LaneReport>,
    endpoint: RelayEndpoint,
    route: RelayHandshake,
    local_owner_socket: SocketAddr,
    auth: AuthMode,
    cancel: watch::Receiver<bool>,
    event_tx: mpsc::Sender<LaneEvent>,
    ready_tx: mpsc::Sender<()>,
    limits: RelayLimits,
    delayed: bool,
) {
    lanes.spawn(async move {
        run_lane(
            endpoint,
            route,
            local_owner_socket,
            auth,
            cancel,
            event_tx,
            ready_tx,
            limits,
            delayed,
        )
        .await
    });
}

#[allow(clippy::too_many_arguments)]
async fn run_lane(
    endpoint: RelayEndpoint,
    route: RelayHandshake,
    local_owner_socket: SocketAddr,
    auth: AuthMode,
    mut cancel: watch::Receiver<bool>,
    event_tx: mpsc::Sender<LaneEvent>,
    ready_tx: mpsc::Sender<()>,
    limits: RelayLimits,
    delayed: bool,
) -> LaneReport {
    let mut became_ready = false;
    let mut became_active = false;
    let result = async {
        if delayed && !sleep_or_cancel(limits.retry, &mut cancel).await {
            return Err(TunnelError::Cancelled);
        }
        let started_at = Instant::now();
        let socket = establish_relay_with_ready_hook(
            &endpoint,
            &route,
            op_collab_relay_protocol::RelayRole::Owner,
            &auth,
            &mut cancel,
            started_at,
            limits,
            || {
                became_ready = true;
                let _ = ready_tx.try_send(());
            },
        )
        .await?;
        let local = connect_local(local_owner_socket, &mut cancel, limits).await?;
        let (acknowledge, acknowledged) = oneshot::channel();
        tokio::select! {
            _ = cancelled(&mut cancel) => return Err(TunnelError::Cancelled),
            result = event_tx.send(LaneEvent::Active(acknowledge)) => {
                if result.is_err() {
                    return Err(TunnelError::Cancelled);
                }
            }
        }
        tokio::select! {
            _ = cancelled(&mut cancel) => return Err(TunnelError::Cancelled),
            result = acknowledged => {
                if result.is_err() {
                    return Err(TunnelError::Cancelled);
                }
            }
        }
        became_active = true;
        pump(
            socket,
            local,
            ClientReauthContext {
                auth: &auth,
                role: op_collab_relay_protocol::RelayRole::Owner,
                route: route.route(),
            },
            &mut cancel,
            started_at,
            limits,
        )
        .await
    }
    .await;
    LaneReport {
        became_ready,
        became_active,
        result,
    }
}

async fn connect_local(
    local_owner_socket: SocketAddr,
    cancel: &mut watch::Receiver<bool>,
    limits: RelayLimits,
) -> Result<TcpStream, TunnelError> {
    let local = tokio::select! {
        _ = cancelled(cancel) => return Err(TunnelError::Cancelled),
        result = tokio::time::timeout(limits.connect, TcpStream::connect(local_owner_socket)) => {
            match result {
                Err(_) => return Err(TunnelError::Failure(RelayFailureKind::ConnectTimeout)),
                Ok(Err(_)) => return Err(TunnelError::Failure(RelayFailureKind::LocalIo)),
                Ok(Ok(local)) => local,
            }
        }
    };
    local
        .set_nodelay(true)
        .map_err(|_| TunnelError::Failure(RelayFailureKind::LocalIo))?;
    Ok(local)
}

fn publish_owner(
    status: &watch::Sender<RelayBridgeStatus>,
    waiting_lanes: usize,
    active_tunnels: usize,
    last_error: Option<RelayFailureKind>,
) {
    let phase = if active_tunnels > 0 {
        RelayBridgePhase::Active
    } else if waiting_lanes > 0 {
        RelayBridgePhase::Waiting
    } else if last_error.is_some() {
        RelayBridgePhase::Degraded
    } else {
        RelayBridgePhase::Starting
    };
    status.send_replace(RelayBridgeStatus {
        phase,
        waiting_lanes,
        active_tunnels,
        last_error,
    });
}
