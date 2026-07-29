#![cfg(test)]

use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ed25519_dalek::{Signer as _, SigningKey, Verifier as _};
use op_collab_relay_control_plane::{
    OwnerPublishDraft, OwnerPublishRequest, RelayLocatorPublishServiceError, RelayLocatorSigner,
    RelayPublishLifetime, SignedLocatorResponse, MAX_PUBLISH_AUTHORIZATION_BYTES,
};
use op_collab_relay_protocol::{
    ExpectedDiscoveryId, LocatorKeyId, LocatorSignature, OwnerNoiseStatic, RelayRegion,
    UnsignedRelayLocatorV1,
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};

use crate::{
    serve_listener_until, ExpectedUnixPeer, LocatorHttpLimits, LocatorPublisher,
    LocatorServerConfig, UnixHsmRelayLocatorSigner, HSM_SIGN_REQUEST_BYTES,
    HSM_SIGN_RESPONSE_BYTES,
};

const OWNER_KEY: [u8; 32] = [0x42; 32];
const TEST_BEARER: &[u8] = b"header.payload.signature";
const TEST_KEY_ID: &str = "locator-test-key";

struct SigningPublisher {
    signing_key: SigningKey,
    delay: Duration,
}

impl SigningPublisher {
    fn immediate() -> Self {
        Self {
            signing_key: SigningKey::from_bytes(&[0x71; 32]),
            delay: Duration::ZERO,
        }
    }
}

impl LocatorPublisher for SigningPublisher {
    fn publish(
        &self,
        request_body: &[u8],
        opaque_ticket: &[u8],
    ) -> Result<SignedLocatorResponse, RelayLocatorPublishServiceError> {
        if opaque_ticket != TEST_BEARER {
            return Err(RelayLocatorPublishServiceError::AuthenticationFailed);
        }
        if !self.delay.is_zero() {
            std::thread::sleep(self.delay);
        }
        let request = OwnerPublishRequest::decode_binary(request_body)
            .map_err(|_| RelayLocatorPublishServiceError::RequestRejected)?;
        let now = unix_now();
        let key_id = LocatorKeyId::new(TEST_KEY_ID).expect("test locator key id");
        let claims = UnsignedRelayLocatorV1::new(
            request.home_region(),
            *request.route_id(),
            request.generation(),
            *request.owner_noise_static(),
            request.expected_discovery_id().clone(),
            now,
            now + request.desired_lifetime().seconds(),
            key_id,
        )
        .map_err(|_| RelayLocatorPublishServiceError::Unavailable)?;
        let signature = LocatorSignature::new(
            self.signing_key
                .sign(&claims.canonical_signing_bytes())
                .to_bytes(),
        )
        .map_err(|_| RelayLocatorPublishServiceError::Unavailable)?;
        SignedLocatorResponse::decode(&claims.attach_signature(signature).encode())
            .map_err(|_| RelayLocatorPublishServiceError::Unavailable)
    }
}

#[tokio::test]
async fn real_http_route_accepts_exact_publish_and_health_is_non_sensitive() {
    let server = TestServer::start(
        Arc::new(SigningPublisher::immediate()),
        LocatorHttpLimits::default(),
    )
    .await;
    let body = publish_request();
    let response = server.request(exact_publish_request(&body)).await;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("\r\ncontent-type: application/vnd.openpencil.relay-locator-v1\r\n"));
    let response_body = http_body(&response);
    let locator = SignedLocatorResponse::decode(response_body).expect("signed locator");
    assert_eq!(locator.locator().claims().home_region(), RelayRegion::Cn);

    let health = server
        .request(b"GET /healthz HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n".to_vec())
        .await;
    assert!(health.starts_with("HTTP/1.1 204 No Content\r\n"));
    assert!(http_body(&health).is_empty());
    server.stop().await;
}

#[tokio::test]
async fn real_http_route_rejects_method_query_headers_bearer_and_body() {
    let server = TestServer::start(
        Arc::new(SigningPublisher::immediate()),
        LocatorHttpLimits::default(),
    )
    .await;
    let body = publish_request();
    let cases = [
        (
            b"GET /v1/locator HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n".to_vec(),
            "405 Method Not Allowed",
        ),
        (
            publish_wire(
                "/v1/locator?leak=1",
                &body,
                "Bearer header.payload.signature",
                true,
            ),
            "404 Not Found",
        ),
        (
            publish_wire("/v1/locator", &body, "Basic abc", true),
            "401 Unauthorized",
        ),
        (
            publish_wire("/v1/locator", &body, "Bearer bad,token", true),
            "401 Unauthorized",
        ),
        (
            publish_wire(
                "/v1/locator",
                &body,
                "Bearer header.payload.signature",
                false,
            ),
            "400 Bad Request",
        ),
    ];
    for (request, expected) in cases {
        let response = server.request(request).await;
        assert!(
            response.starts_with(&format!("HTTP/1.1 {expected}\r\n")),
            "{response}"
        );
        assert!(!response.contains("header.payload.signature"));
    }

    let maximum_bearer = format!(
        "Bearer {}",
        "a".repeat(MAX_PUBLISH_AUTHORIZATION_BYTES - "Bearer ".len())
    );
    let response = server
        .request(publish_wire("/v1/locator", &body, &maximum_bearer, true))
        .await;
    assert!(
        response.starts_with("HTTP/1.1 401 Unauthorized\r\n"),
        "the bounded 48 KiB ticket envelope must reach authentication: {response:?}"
    );

    let mut duplicate = exact_publish_request(&body);
    let marker = b"Authorization: Bearer header.payload.signature\r\n";
    let insertion = duplicate
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("authorization");
    duplicate.splice(insertion..insertion, marker.iter().copied());
    let response = server.request(duplicate).await;
    assert!(response.starts_with("HTTP/1.1 401 Unauthorized\r\n"));

    let mut oversized = vec![0_u8; body.len() + 1];
    oversized[..body.len()].copy_from_slice(&body);
    let response = server.request(exact_publish_request(&oversized)).await;
    assert!(
        response.starts_with("HTTP/1.1 413 "),
        "unexpected oversized-body response: {response:?}"
    );
    server.stop().await;
}

#[tokio::test]
async fn rate_auth_concurrency_and_auth_timeout_fail_closed() {
    let rate_limits = LocatorHttpLimits {
        max_requests_per_second: std::num::NonZeroU32::MIN,
        ..LocatorHttpLimits::default()
    };
    let server = TestServer::start(Arc::new(SigningPublisher::immediate()), rate_limits).await;
    let request = exact_publish_request(&publish_request());
    let malformed = publish_wire(
        "/v1/locator",
        &publish_request(),
        "Bearer header.payload.signature",
        false,
    );
    assert!(server
        .request(malformed)
        .await
        .starts_with("HTTP/1.1 400 Bad Request\r\n"));
    assert!(server
        .request(request.clone())
        .await
        .starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(server
        .request(request)
        .await
        .starts_with("HTTP/1.1 429 Too Many Requests\r\n"));
    server.stop().await;

    let timeout_limits = LocatorHttpLimits {
        auth_timeout: Duration::from_millis(10),
        ..LocatorHttpLimits::default()
    };
    let server = TestServer::start(
        Arc::new(SigningPublisher {
            signing_key: SigningKey::from_bytes(&[0x71; 32]),
            delay: Duration::from_millis(100),
        }),
        timeout_limits,
    )
    .await;
    let response = server
        .request(exact_publish_request(&publish_request()))
        .await;
    assert!(response.starts_with("HTTP/1.1 503 Service Unavailable\r\n"));
    server.stop().await;
}

#[cfg(unix)]
#[test]
fn unix_hsm_protocol_authenticates_peer_and_returns_signature() {
    use std::{
        io::{Read as _, Write as _},
        os::unix::net::UnixListener,
        thread,
    };

    let directory = workspace_tempdir();
    let socket_path = directory.path().join("hsm.sock");
    let listener = UnixListener::bind(&socket_path).expect("HSM listener");
    let signing_key = SigningKey::from_bytes(&[0x33; 32]);
    let verifying_key = signing_key.verifying_key();
    let hsm = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request = Vec::new();
        stream.read_to_end(&mut request).expect("request");
        assert_eq!(request.len(), HSM_SIGN_REQUEST_BYTES);
        assert_eq!(&request[..4], b"OPLS");
        assert_eq!(request[4], 1);
        assert_eq!(request[5], 1);
        let key_length = usize::from(request[6]);
        assert_eq!(&request[7..7 + key_length], TEST_KEY_ID.as_bytes());
        assert!(request[7 + key_length..71].iter().all(|byte| *byte == 0));
        let signature = signing_key.sign(&request[71..]).to_bytes();
        let mut response = [0_u8; HSM_SIGN_RESPONSE_BYTES];
        response[..4].copy_from_slice(b"OPLR");
        response[4] = 1;
        response[5] = 0;
        response[6..].copy_from_slice(&signature);
        stream.write_all(&response).expect("response");
    });
    let signer = UnixHsmRelayLocatorSigner::new(
        &socket_path,
        LocatorKeyId::new(TEST_KEY_ID).expect("key id"),
        current_peer(),
        Duration::from_secs(1),
    )
    .expect("signer");
    signer.validate_socket().expect("safe socket");
    let canonical = [0xA5; 268];
    let signature = signer
        .sign(&LocatorKeyId::new(TEST_KEY_ID).expect("key id"), &canonical)
        .expect("signature");
    verifying_key
        .verify(
            &canonical,
            &ed25519_dalek::Signature::from_bytes(signature.as_bytes()),
        )
        .expect("valid signature");
    hsm.join().expect("HSM");
    let debug = format!("{signer:?}");
    assert!(!debug.contains(socket_path.to_string_lossy().as_ref()));
}

#[cfg(unix)]
#[test]
fn unix_hsm_rejects_symlink_and_wrong_peer_identity() {
    use std::os::unix::{fs::symlink, net::UnixListener};

    let directory = workspace_tempdir();
    let socket_path = directory.path().join("hsm.sock");
    let link_path = directory.path().join("hsm-link.sock");
    let _listener = UnixListener::bind(&socket_path).expect("HSM listener");
    symlink(&socket_path, &link_path).expect("symlink");
    let key_id = LocatorKeyId::new(TEST_KEY_ID).expect("key id");
    let linked = UnixHsmRelayLocatorSigner::new(
        link_path,
        key_id.clone(),
        current_peer(),
        Duration::from_secs(1),
    )
    .expect("shape");
    assert!(linked.validate_socket().is_err());

    let current = current_peer();
    let wrong = UnixHsmRelayLocatorSigner::new(
        socket_path,
        key_id,
        ExpectedUnixPeer {
            uid: current.uid.wrapping_add(1),
            gid: current.gid,
        },
        Duration::from_secs(1),
    )
    .expect("shape");
    assert!(wrong.validate_socket().is_err());
}

struct TestServer {
    address: std::net::SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

impl TestServer {
    async fn start(publisher: Arc<dyn LocatorPublisher>, limits: LocatorHttpLimits) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("address");
        let config = LocatorServerConfig::new(address, limits).expect("config");
        let (shutdown, receiver) = oneshot::channel();
        let task = tokio::spawn(async move {
            serve_listener_until(listener, config, publisher, async {
                let _ = receiver.await;
            })
            .await
            .expect("server");
        });
        Self {
            address,
            shutdown,
            task,
        }
    }

    async fn request(&self, request: Vec<u8>) -> String {
        let mut stream = TcpStream::connect(self.address).await.expect("connect");
        stream.write_all(&request).await.expect("write");
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.expect("read");
        String::from_utf8(response).expect("HTTP response")
    }

    async fn stop(self) {
        let _ = self.shutdown.send(());
        self.task.await.expect("join");
    }
}

fn publish_request() -> [u8; 191] {
    OwnerPublishDraft::generate(
        RelayRegion::Cn,
        OwnerNoiseStatic::new(OWNER_KEY).expect("owner key"),
        ExpectedDiscoveryId::new("stable-relay-prelude").expect("discovery"),
        RelayPublishLifetime::new(600).expect("lifetime"),
    )
    .expect("draft")
    .request()
    .encode_binary()
}

fn exact_publish_request(body: &[u8]) -> Vec<u8> {
    publish_wire("/v1/locator", body, "Bearer header.payload.signature", true)
}

fn publish_wire(path: &str, body: &[u8], authorization: &str, include_accept: bool) -> Vec<u8> {
    let accept = if include_accept {
        "Accept: application/vnd.openpencil.relay-locator-v1\r\n"
    } else {
        ""
    };
    let headers = format!(
        "POST {path} HTTP/1.1\r\nHost: test\r\n\
         Content-Type: application/vnd.openpencil.relay-owner-publish-v1\r\n\
         {accept}Authorization: {authorization}\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut request = headers.into_bytes();
    request.extend_from_slice(body);
    request
}

fn http_body(response: &str) -> &str {
    response.split_once("\r\n\r\n").map_or("", |(_, body)| body)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs()
}

#[cfg(unix)]
fn current_peer() -> ExpectedUnixPeer {
    ExpectedUnixPeer {
        uid: unsafe { libc::geteuid() },
        gid: unsafe { libc::getegid() },
    }
}

#[cfg(unix)]
fn workspace_tempdir() -> tempfile::TempDir {
    let system_temp = std::env::temp_dir()
        .canonicalize()
        .expect("canonical system temp directory");
    tempfile::tempdir_in(system_temp).expect("system temp directory")
}
