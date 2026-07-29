use std::{
    borrow::Cow,
    num::NonZeroU64,
    sync::{Arc, Mutex},
};

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use op_collab_relay_protocol::{
    is_valid_relay_bearer_token, RelayClientHello, RelayProtocolError, RelayRejectCode,
    RelayServerStatus, MAX_RELAY_BEARER_BYTES, MAX_RELAY_REAUTH_RESPONSE_TEXT_BYTES,
    RELAY_CHALLENGE_HEADER_NAME,
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, watch, OwnedSemaphorePermit, Semaphore},
    time::{self, Instant, Sleep},
};
use tokio_tungstenite::{
    accept_hdr_async_with_config,
    tungstenite::{
        handshake::server::{ErrorResponse, Request, Response},
        http::HeaderValue,
        protocol::{frame::coding::CloseCode, CloseFrame, WebSocketConfig},
        Message,
    },
    WebSocketStream,
};
use zeroize::Zeroizing;

use crate::{
    auth::{AuthenticatedRoute, RelayAuthenticator, RelayBearerCredential, RelayUpgradeChallenge},
    config::RelayConfig,
    connection_reauth::{
        perform_reauthentication, ReauthOutcome, ReauthRelayTraffic, RelayAuthState,
        RelaySessionIdentity,
    },
    registry::{PairNotice, QueuedPayload, Registration, Registry, WaitingRegistration},
};

type WebSocket = WebSocketStream<TcpStream>;
pub(crate) type WebSocketSink = SplitSink<WebSocket, Message>;
pub(crate) type WebSocketSource = SplitStream<WebSocket>;

const _: () = assert!(MAX_RELAY_BEARER_BYTES == op_auth_bridge::MAX_COLLAB_TICKET_BYTES);

pub(crate) struct ConnectionServices {
    pub(crate) config: Arc<RelayConfig>,
    pub(crate) registry: Registry,
    pub(crate) authenticator: Arc<dyn RelayAuthenticator>,
    pub(crate) auth_in_flight: Arc<Semaphore>,
    pub(crate) queued_bytes: Arc<Semaphore>,
}

pub(crate) async fn serve_connection(
    stream: TcpStream,
    services: ConnectionServices,
    mut shutdown: watch::Receiver<bool>,
    pending_permit: OwnedSemaphorePermit,
) {
    let ConnectionServices {
        config,
        registry,
        authenticator,
        auth_in_flight,
        queued_bytes,
    } = services;
    let started_at = Instant::now();
    let handshake_deadline = started_at + config.handshake_timeout;
    let Ok(auth_permit) = Arc::clone(&auth_in_flight).try_acquire_owned() else {
        return;
    };
    let challenge_authenticator = Arc::clone(&authenticator);
    let challenge_key = tokio::task::spawn_blocking(move || {
        (challenge_authenticator.challenge_key_id(), auth_permit)
    });
    let (challenge_key, auth_permit) =
        match time::timeout_at(handshake_deadline, challenge_key).await {
            Ok(Ok((Ok(key_id), permit))) => (key_id, permit),
            Ok(Ok((Err(_), _))) | Ok(Err(_)) | Err(_) => return,
        };
    let challenge = match challenge_key {
        Some(key_id) => {
            match RelayUpgradeChallenge::generate(key_id, handshake_deadline.into_std()) {
                Ok(challenge) => Some(challenge),
                Err(_) => return,
            }
        }
        None => None,
    };
    let strict_reauth = challenge.is_some();
    let challenge_header = challenge
        .as_ref()
        .map(|challenge| challenge.challenge().encode_header());
    let (websocket, credential) = match time::timeout_at(
        handshake_deadline,
        accept_tunnel(
            stream,
            config.max_message_bytes,
            config.unauthenticated_dev(),
            challenge_header,
        ),
    )
    .await
    {
        Ok(Ok(websocket)) => websocket,
        Ok(Err(_)) => return,
        Err(_) => {
            tracing::debug!("relay WebSocket handshake timed out");
            return;
        }
    };

    let (mut sink, mut source) = websocket.split();
    let first = match time::timeout_at(handshake_deadline, source.next()).await {
        Ok(Some(Ok(Message::Binary(payload)))) => Zeroizing::new(payload),
        Ok(Some(Ok(_))) => {
            reject_and_close(&mut sink, RelayRejectCode::MalformedHello).await;
            return;
        }
        Ok(Some(Err(_))) | Ok(None) | Err(_) => return,
    };
    let hello = match RelayClientHello::decode(&first) {
        Ok(hello) => hello,
        Err(error) => {
            reject_and_close(&mut sink, hello_decode_reject_code(&error)).await;
            return;
        }
    };
    drop(first);
    let session_hello = hello.clone();
    let initial_authenticator = Arc::clone(&authenticator);
    let authentication = tokio::task::spawn_blocking(move || {
        let _auth_permit = auth_permit;
        initial_authenticator.authenticate(&hello, credential.as_ref(), challenge)
    });
    let authentication = match time::timeout_at(handshake_deadline, authentication).await {
        Ok(Ok(authentication)) => authentication,
        Ok(Err(_)) | Err(_) => {
            reject_and_close(&mut sink, RelayRejectCode::AuthenticationFailed).await;
            return;
        }
    };
    let AuthenticatedRoute {
        route,
        role,
        expires_at_unix,
    } = match authentication {
        Ok(route) => route,
        Err(code) => {
            reject_and_close(&mut sink, code).await;
            return;
        }
    };
    let Some(expires_at_unix) =
        NonZeroU64::new(expires_at_unix.get().min(session_hello.expires_at_unix()))
    else {
        reject_and_close(&mut sink, RelayRejectCode::AuthenticationFailed).await;
        return;
    };
    let Some(mut auth_state) = RelayAuthState::new(expires_at_unix) else {
        reject_and_close(&mut sink, RelayRejectCode::AuthenticationFailed).await;
        return;
    };
    let Some(identity) = RelaySessionIdentity::new(route, role, &session_hello) else {
        reject_and_close(&mut sink, RelayRejectCode::AuthenticationFailed).await;
        return;
    };
    let configured_deadline = started_at
        .checked_add(config.tunnel_lifetime)
        .unwrap_or(auth_state.deadline());

    let (relay_tx, relay_rx) = mpsc::channel(config.relay_queue_capacity);
    let registration = match registry.register(route, role, relay_tx).await {
        Ok(registration) => registration,
        Err(_) => {
            reject_and_close(&mut sink, RelayRejectCode::Capacity).await;
            return;
        }
    };
    if !send_status(&mut sink, RelayServerStatus::Ready).await {
        if let Registration::Waiting(waiting) = &registration {
            registry.unregister_waiter(waiting).await;
        }
        return;
    }

    let mut pair = match registration {
        Registration::Paired(pair) => {
            drop(pending_permit);
            pair
        }
        Registration::Waiting(mut waiting) => {
            let pair = wait_for_pair(
                &mut sink,
                &mut source,
                &mut waiting,
                &config,
                &mut auth_state,
                configured_deadline,
                strict_reauth,
                Arc::clone(&authenticator),
                Arc::clone(&auth_in_flight),
                &identity,
                &mut shutdown,
            )
            .await;
            registry.unregister_waiter(&waiting).await;
            drop(pending_permit);
            let Some(pair) = pair else {
                return;
            };
            pair
        }
    };

    if !send_status(&mut sink, RelayServerStatus::Paired).await {
        registry.close_pair(pair.pair_id).await;
        return;
    }
    let synchronize_timeout = auth_state
        .effective_deadline(configured_deadline)
        .saturating_duration_since(Instant::now())
        .min(config.handshake_timeout);
    if synchronize_timeout.is_zero() || !pair.synchronize(synchronize_timeout).await {
        close(&mut sink, CloseCode::Away, "peer disconnected").await;
        registry.close_pair(pair.pair_id).await;
        return;
    }
    relay_paired(
        sink,
        source,
        relay_rx,
        pair,
        queued_bytes,
        &registry,
        &config,
        auth_state,
        configured_deadline,
        strict_reauth,
        authenticator,
        auth_in_flight,
        identity,
        &mut shutdown,
    )
    .await;
}

// Tungstenite fixes the handshake callback's error to a full HTTP response.
#[allow(clippy::result_large_err)]
async fn accept_tunnel(
    stream: TcpStream,
    max_message_bytes: usize,
    allow_anonymous_dev: bool,
    challenge_header: Option<String>,
) -> Result<(WebSocket, Option<RelayBearerCredential>), tokio_tungstenite::tungstenite::Error> {
    let credential = Arc::new(Mutex::new(None));
    let callback_credential = Arc::clone(&credential);
    let callback = move |request: &Request, mut response: Response| {
        if request.uri().path_and_query().map(|uri| uri.as_str()) != Some("/v1/tunnel") {
            return Err(http_error_response(404, "not found"));
        }
        match parse_bearer(request) {
            Ok(Some(value)) => {
                *callback_credential
                    .lock()
                    .expect("relay credential mutex is not poisoned") = Some(value);
                if let Some(value) = challenge_header.as_deref() {
                    response.headers_mut().insert(
                        RELAY_CHALLENGE_HEADER_NAME,
                        HeaderValue::from_str(value)
                            .expect("protocol challenge encoding is a valid HTTP header"),
                    );
                }
                Ok(response)
            }
            Ok(None) if allow_anonymous_dev => {
                debug_assert!(challenge_header.is_none());
                Ok(response)
            }
            Ok(None) | Err(()) => Err(http_error_response(401, "unauthorized")),
        }
    };
    let max_wire_message_bytes = max_message_bytes.max(MAX_RELAY_REAUTH_RESPONSE_TEXT_BYTES);
    let config = WebSocketConfig {
        write_buffer_size: 8 * 1024,
        max_write_buffer_size: max_wire_message_bytes.saturating_mul(2) + 8 * 1024,
        max_message_size: Some(max_wire_message_bytes),
        max_frame_size: Some(max_wire_message_bytes),
        ..WebSocketConfig::default()
    };
    let websocket = accept_hdr_async_with_config(stream, callback, Some(config)).await?;
    let credential = credential
        .lock()
        .expect("relay credential mutex is not poisoned")
        .take();
    Ok((websocket, credential))
}

fn parse_bearer(request: &Request) -> Result<Option<RelayBearerCredential>, ()> {
    const PREFIX: &[u8] = b"Bearer ";

    let mut values = request.headers().get_all("authorization").iter();
    let Some(value) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() {
        return Err(());
    }
    let value = value.as_bytes();
    let token = value.strip_prefix(PREFIX).ok_or(())?;
    if token.len() > MAX_RELAY_BEARER_BYTES || !is_rfc6750_b64token(token) {
        return Err(());
    }
    Ok(Some(RelayBearerCredential::new(token.to_vec())))
}

pub(crate) fn is_rfc6750_b64token(token: &[u8]) -> bool {
    is_valid_relay_bearer_token(token)
}

fn http_error_response(status: u16, message: &'static str) -> ErrorResponse {
    Response::builder()
        .status(status)
        .body(Some(message.to_string()))
        .expect("static HTTP rejection response is valid")
}

#[allow(clippy::too_many_arguments)]
async fn wait_for_pair(
    sink: &mut WebSocketSink,
    source: &mut WebSocketSource,
    waiting: &mut WaitingRegistration,
    config: &RelayConfig,
    auth_state: &mut RelayAuthState,
    configured_deadline: Instant,
    strict_reauth: bool,
    authenticator: Arc<dyn RelayAuthenticator>,
    auth_in_flight: Arc<Semaphore>,
    identity: &RelaySessionIdentity,
    shutdown: &mut watch::Receiver<bool>,
) -> Option<PairNotice> {
    let waiting_deadline = Instant::now() + config.waiting_timeout;
    let mut idle = sleep_until(Instant::now() + config.idle_timeout);
    let mut waiting_timeout = sleep_until(waiting_deadline);
    let mut lifetime = sleep_until(auth_state.effective_deadline(configured_deadline));
    let mut reauth_at = strict_reauth
        .then(|| {
            auth_state.reauth_at(
                Instant::now(),
                configured_deadline,
                identity.locator_expiry_unix(),
            )
        })
        .flatten();
    let mut reauth = sleep_until(
        reauth_at.unwrap_or_else(|| auth_state.effective_deadline(configured_deadline)),
    );

    loop {
        tokio::select! {
            biased;
            changed = shutdown.changed() => {
                if changed.is_ok() && *shutdown.borrow() {
                    close(sink, CloseCode::Restart, "relay shutdown").await;
                    return None;
                }
            }
            () = &mut lifetime => {
                reject_and_close(sink, RelayRejectCode::AuthenticationFailed).await;
                return None;
            }
            () = &mut reauth, if reauth_at.is_some() => {
                match perform_reauthentication(
                    sink,
                    source,
                    Arc::clone(&authenticator),
                    Arc::clone(&auth_in_flight),
                    identity,
                    *auth_state,
                    configured_deadline,
                    config.handshake_timeout,
                    None,
                ).await {
                    Ok(ReauthOutcome::Extended(extended)) => {
                        *auth_state = extended;
                        lifetime.as_mut().reset(
                            auth_state.effective_deadline(configured_deadline),
                        );
                        reauth_at = auth_state.reauth_at(
                            Instant::now(),
                            configured_deadline,
                            identity.locator_expiry_unix(),
                        );
                        reauth.as_mut().reset(
                            reauth_at.unwrap_or_else(|| {
                                auth_state.effective_deadline(configured_deadline)
                            }),
                        );
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Ok(ReauthOutcome::StillCurrent) => {
                        reauth_at = auth_state.reauth_at(
                            Instant::now(),
                            configured_deadline,
                            identity.locator_expiry_unix(),
                        );
                        reauth.as_mut().reset(
                            reauth_at.unwrap_or_else(|| {
                                auth_state.effective_deadline(configured_deadline)
                            }),
                        );
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Err(()) => {
                        close(
                            sink,
                            CloseCode::Policy,
                            "relay reauthentication failed",
                        ).await;
                        return None;
                    }
                }
            }
            () = &mut waiting_timeout => {
                reject_and_close(sink, RelayRejectCode::PairingTimeout).await;
                return None;
            }
            () = &mut idle => {
                close(sink, CloseCode::Away, "idle timeout").await;
                return None;
            }
            result = &mut waiting.pair_rx => {
                return result.ok();
            }
            message = source.next() => {
                match message {
                    Some(Ok(Message::Ping(payload))) => {
                        if sink.send(Message::Pong(payload)).await.is_err() {
                            return None;
                        }
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Some(Ok(Message::Pong(_))) => {
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Some(Ok(Message::Close(frame))) => {
                        let _ = sink.send(Message::Close(frame)).await;
                        return None;
                    }
                    Some(Ok(Message::Binary(_) | Message::Text(_) | Message::Frame(_))) => {
                        // No application payload is accepted before both peers
                        // have received Paired.
                        reject_and_close(sink, RelayRejectCode::MalformedHello).await;
                        return None;
                    }
                    Some(Err(_)) | None => return None,
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn relay_paired(
    mut sink: WebSocketSink,
    mut source: WebSocketSource,
    mut relay_rx: mpsc::Receiver<QueuedPayload>,
    pair: PairNotice,
    queued_bytes: Arc<Semaphore>,
    registry: &Registry,
    config: &RelayConfig,
    mut auth_state: RelayAuthState,
    configured_deadline: Instant,
    strict_reauth: bool,
    authenticator: Arc<dyn RelayAuthenticator>,
    auth_in_flight: Arc<Semaphore>,
    identity: RelaySessionIdentity,
    shutdown: &mut watch::Receiver<bool>,
) {
    let mut idle = sleep_until(Instant::now() + config.idle_timeout);
    let mut lifetime = sleep_until(auth_state.effective_deadline(configured_deadline));
    let mut reauth_at = strict_reauth
        .then(|| {
            auth_state.reauth_at(
                Instant::now(),
                configured_deadline,
                identity.locator_expiry_unix(),
            )
        })
        .flatten();
    let mut reauth = sleep_until(
        reauth_at.unwrap_or_else(|| auth_state.effective_deadline(configured_deadline)),
    );

    loop {
        tokio::select! {
            biased;
            changed = shutdown.changed() => {
                if changed.is_ok() && *shutdown.borrow() {
                    close(&mut sink, CloseCode::Restart, "relay shutdown").await;
                    break;
                }
            }
            () = &mut lifetime => {
                close(&mut sink, CloseCode::Away, "tunnel lifetime reached").await;
                break;
            }
            () = &mut reauth, if reauth_at.is_some() => {
                match perform_reauthentication(
                    &mut sink,
                    &mut source,
                    Arc::clone(&authenticator),
                    Arc::clone(&auth_in_flight),
                    &identity,
                    auth_state,
                    configured_deadline,
                    config.handshake_timeout,
                    Some(&mut ReauthRelayTraffic {
                        relay_rx: &mut relay_rx,
                        peer_tx: &pair.peer_tx,
                        route_budget: &pair.route_budget,
                        queued_bytes: &queued_bytes,
                        max_message_bytes: config.max_message_bytes,
                    }),
                ).await {
                    Ok(ReauthOutcome::Extended(extended)) => {
                        auth_state = extended;
                        lifetime.as_mut().reset(
                            auth_state.effective_deadline(configured_deadline),
                        );
                        reauth_at = auth_state.reauth_at(
                            Instant::now(),
                            configured_deadline,
                            identity.locator_expiry_unix(),
                        );
                        reauth.as_mut().reset(
                            reauth_at.unwrap_or_else(|| {
                                auth_state.effective_deadline(configured_deadline)
                            }),
                        );
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Ok(ReauthOutcome::StillCurrent) => {
                        reauth_at = auth_state.reauth_at(
                            Instant::now(),
                            configured_deadline,
                            identity.locator_expiry_unix(),
                        );
                        reauth.as_mut().reset(
                            reauth_at.unwrap_or_else(|| {
                                auth_state.effective_deadline(configured_deadline)
                            }),
                        );
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Err(()) => {
                        close(
                            &mut sink,
                            CloseCode::Policy,
                            "relay reauthentication failed",
                        ).await;
                        break;
                    }
                }
            }
            () = &mut idle => {
                close(&mut sink, CloseCode::Away, "idle timeout").await;
                break;
            }
            message = source.next() => {
                match message {
                    Some(Ok(Message::Binary(payload))) => {
                        if payload.len() > config.max_message_bytes {
                            close(&mut sink, CloseCode::Size, "message too large").await;
                            break;
                        }
                        let permits = u32::try_from(payload.len())
                            .expect("payload length is bounded to 64 KiB");
                        let Ok(global_budget) = Arc::clone(&queued_bytes)
                            .try_acquire_many_owned(permits)
                        else {
                            close(&mut sink, CloseCode::Again, "relay backpressure").await;
                            break;
                        };
                        let Ok(route_budget) = Arc::clone(&pair.route_budget)
                            .try_acquire_many_owned(permits)
                        else {
                            close(&mut sink, CloseCode::Again, "relay backpressure").await;
                            break;
                        };
                        let queued =
                            QueuedPayload::new(payload, global_budget, route_budget);
                        if pair.peer_tx.try_send(queued).is_err() {
                            close(&mut sink, CloseCode::Again, "relay backpressure").await;
                            break;
                        }
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Some(Ok(Message::Text(_))) => {
                        close(&mut sink, CloseCode::Unsupported, "binary messages only").await;
                        break;
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if sink.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Some(Ok(Message::Pong(_))) => {
                        reset_idle(&mut idle, config.idle_timeout);
                    }
                    Some(Ok(Message::Close(frame))) => {
                        let _ = sink.send(Message::Close(frame)).await;
                        break;
                    }
                    Some(Ok(Message::Frame(_))) => {
                        close(&mut sink, CloseCode::Protocol, "unexpected frame").await;
                        break;
                    }
                    Some(Err(_)) | None => break,
                }
            }
            queued = relay_rx.recv() => {
                let Some(queued) = queued else {
                    close(&mut sink, CloseCode::Away, "peer disconnected").await;
                    break;
                };
                let (payload, _global_budget, _route_budget) =
                    queued.into_parts();
                if sink.send(Message::Binary(payload)).await.is_err() {
                    break;
                }
                reset_idle(&mut idle, config.idle_timeout);
            }
        }
    }

    registry.close_pair(pair.pair_id).await;
}

fn sleep_until(deadline: Instant) -> std::pin::Pin<Box<Sleep>> {
    Box::pin(time::sleep_until(deadline))
}

fn reset_idle(idle: &mut std::pin::Pin<Box<Sleep>>, timeout: std::time::Duration) {
    idle.as_mut().reset(Instant::now() + timeout);
}

async fn send_status(sink: &mut WebSocketSink, status: RelayServerStatus) -> bool {
    sink.send(Message::Binary(status.encode().to_vec()))
        .await
        .is_ok()
}

async fn reject_and_close(sink: &mut WebSocketSink, code: RelayRejectCode) {
    let _ = send_status(sink, RelayServerStatus::Rejected(code)).await;
    close(sink, CloseCode::Policy, "relay request rejected").await;
}

fn hello_decode_reject_code(error: &RelayProtocolError) -> RelayRejectCode {
    match error {
        RelayProtocolError::UnsupportedVersion { .. } => RelayRejectCode::UnsupportedVersion,
        _ => RelayRejectCode::MalformedHello,
    }
}

async fn close(sink: &mut WebSocketSink, code: CloseCode, reason: &'static str) {
    let _ = sink
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: Cow::Borrowed(reason),
        })))
        .await;
}
