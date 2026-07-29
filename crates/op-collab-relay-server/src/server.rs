use std::{future::Future, sync::Arc, time::SystemTime};

use tokio::{
    net::TcpListener,
    sync::{watch, Semaphore},
    task::JoinSet,
};

use crate::{
    auth::{RelayAuthenticator, UnauthenticatedDevAuthenticator},
    config::RelayConfig,
    connection::{serve_connection, ConnectionServices},
    error::RelayServerError,
    registry::Registry,
};

pub async fn run(config: RelayConfig) -> Result<(), RelayServerError> {
    run_until(config, async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to install Ctrl-C handler");
        }
    })
    .await
}

pub async fn run_with_authenticator(
    config: RelayConfig,
    authenticator: Arc<dyn RelayAuthenticator>,
) -> Result<(), RelayServerError> {
    run_with_authenticator_until(config, authenticator, async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to install Ctrl-C handler");
        }
    })
    .await
}

pub async fn run_until<F>(config: RelayConfig, shutdown: F) -> Result<(), RelayServerError>
where
    F: Future<Output = ()>,
{
    if !config.unauthenticated_dev() {
        return Err(RelayServerError::AuthenticationRequired);
    }
    run_with_authenticator_until(
        config,
        Arc::new(UnauthenticatedDevAuthenticator::new(SystemTime::now)),
        shutdown,
    )
    .await
}

pub async fn run_with_authenticator_until<F>(
    config: RelayConfig,
    authenticator: Arc<dyn RelayAuthenticator>,
    shutdown: F,
) -> Result<(), RelayServerError>
where
    F: Future<Output = ()>,
{
    config.validate()?;
    let listener =
        TcpListener::bind(config.listen)
            .await
            .map_err(|source| RelayServerError::Bind {
                address: config.listen,
                source,
            })?;
    serve_listener(listener, config, authenticator, shutdown).await
}

pub(crate) async fn serve_listener<F>(
    listener: TcpListener,
    config: RelayConfig,
    authenticator: Arc<dyn RelayAuthenticator>,
    shutdown: F,
) -> Result<(), RelayServerError>
where
    F: Future<Output = ()>,
{
    let local_addr = listener.local_addr().map_err(RelayServerError::Accept)?;
    tracing::info!(address = %local_addr, "collaboration relay listening");

    let config = Arc::new(config);
    let registry = Registry::new(
        config.max_active_pairs,
        config.max_waiting_per_route,
        config.max_queued_bytes_per_route,
    );
    let pending = Arc::new(Semaphore::new(config.max_pending));
    let auth_in_flight = Arc::new(Semaphore::new(config.max_auth_in_flight));
    let queued_bytes = Arc::new(Semaphore::new(config.max_queued_bytes));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mut connections = JoinSet::new();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            biased;
            () = &mut shutdown => break,
            accepted = listener.accept() => {
                let (stream, _peer_addr) = accepted.map_err(RelayServerError::Accept)?;
                let Ok(pending_permit) = Arc::clone(&pending).try_acquire_owned() else {
                    tracing::warn!("relay pending connection capacity reached");
                    continue;
                };
                let config = Arc::clone(&config);
                let registry = registry.clone();
                let authenticator = Arc::clone(&authenticator);
                let auth_in_flight = Arc::clone(&auth_in_flight);
                let queued_bytes = Arc::clone(&queued_bytes);
                let shutdown = shutdown_rx.clone();
                connections.spawn(async move {
                    serve_connection(
                        stream,
                        ConnectionServices {
                            config,
                            registry,
                            authenticator,
                            auth_in_flight,
                            queued_bytes,
                        },
                        shutdown,
                        pending_permit,
                    )
                    .await;
                });
            }
            Some(result) = connections.join_next(), if !connections.is_empty() => {
                if let Err(error) = result {
                    tracing::warn!(%error, "relay connection task failed");
                }
            }
        }
    }

    tracing::info!("relay shutdown requested");
    let _ = shutdown_tx.send(true);
    while let Some(result) = connections.join_next().await {
        if let Err(error) = result {
            tracing::warn!(%error, "relay connection task failed during shutdown");
        }
    }
    Ok(())
}
