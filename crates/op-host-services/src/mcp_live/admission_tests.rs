//! Admission-gate tests for the live-MCP endpoint. Driven through the real
//! `serve_connection` router (generic over the stream, like the web-canvas
//! daemon's connection tests) so they cover the wiring, not just the
//! predicates: a refused request must never reach the UI-request channel.

use super::super::*;
use super::*;

const PORT: u16 = 51234;
const TOKEN: &str = "d34db33f-cafe";
/// A read-only tool call. Reads were entirely unauthenticated before this
/// gate existed, so the read path is exactly what these tests must pin.
const LIST_PAGES_CALL: &str = r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"list_pages","arguments":{}}}"#;

struct MockStream {
    input: std::io::Cursor<Vec<u8>>,
    output: Vec<u8>,
}

impl std::io::Read for MockStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.input, buf)
    }
}

impl std::io::Write for MockStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Run one raw HTTP request through the live router and return the raw
/// response.
fn drive(request: &str, req_tx: &Sender<UiRequest>) -> String {
    let admission = LiveAdmission::new(TOKEN.to_string(), PORT);
    let stateful_lock = Mutex::new(());
    let quit_flag = AtomicBool::new(false);
    let wake_ui: UiWake = Arc::new(|| {});
    let client_identity = Mutex::new(None);
    let mut stream = MockStream {
        input: std::io::Cursor::new(request.as_bytes().to_vec()),
        output: Vec::new(),
    };
    serve_connection(
        &mut stream,
        req_tx,
        &admission,
        &stateful_lock,
        &quit_flag,
        &wake_ui,
        &client_identity,
    )
    .expect("a refused request is answered on the wire, never a server error");
    String::from_utf8_lossy(&stream.output).into_owned()
}

fn request(path: &str, headers: &str, body: &str) -> String {
    request_with_method("POST", path, headers, body)
}

fn request_with_method(method: &str, path: &str, headers: &str, body: &str) -> String {
    format!(
        "{method} {path} HTTP/1.1\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// Headers a token-carrying non-browser client sends (VS Code MCP proxy
/// shape: explicit port, no `Origin`).
fn authed_headers() -> String {
    format!("Host: 127.0.0.1:{PORT}\r\nX-OpenPencil-Token: {TOKEN}\r\n")
}

#[test]
fn unauthenticated_tool_call_is_refused() {
    let (req_tx, req_rx) = mpsc::channel();
    let headers = format!("Host: 127.0.0.1:{PORT}\r\n");
    let response = drive(&request("/mcp", &headers, LIST_PAGES_CALL), &req_tx);

    assert!(
        response.starts_with("HTTP/1.1 401 Unauthorized"),
        "{response}"
    );
    assert!(response.contains(r#""code":-32001"#), "{response}");
    // The caller's id is echoed so a client fails fast instead of hanging.
    assert!(response.contains(r#""id":11"#), "{response}");
    assert!(
        req_rx.try_recv().is_err(),
        "a refused tool call must never reach the UI thread"
    );
}

#[test]
fn authenticated_tool_call_is_served() {
    let (req_tx, req_rx) = mpsc::channel();
    let responder = thread::spawn(move || match req_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(UiRequest::ListPages { ack }) => ack
            .send(op_mcp::ListPages {
                page_count: 3,
                active_page_index: 1,
                pages: vec![("p1".to_string(), "One".to_string())],
            })
            .is_ok(),
        _ => false,
    });
    let response = drive(
        &request("/mcp", &authed_headers(), LIST_PAGES_CALL),
        &req_tx,
    );

    assert!(
        responder.join().expect("responder thread"),
        "an authenticated tool call must reach the UI thread"
    );
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.contains("pageCount"), "{response}");
    assert!(!response.contains("-32001"), "{response}");
}

/// The `op` CLI's exact wire shape: no `Origin` (not a browser) and a bare
/// `Host: 127.0.0.1` with no port
/// (`op_rpc_transport::TcpJsonRpc::http_post_request`). It must keep
/// working with the token it is handed by the discovery file.
#[test]
fn cli_request_without_origin_or_host_port_is_served() {
    let (req_tx, req_rx) = mpsc::channel();
    let responder = thread::spawn(move || match req_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(UiRequest::ListPages { ack }) => ack
            .send(op_mcp::ListPages {
                page_count: 1,
                active_page_index: 0,
                pages: vec![("p1".to_string(), "One".to_string())],
            })
            .is_ok(),
        _ => false,
    });
    let headers = format!("Host: 127.0.0.1\r\nX-OpenPencil-Token: {TOKEN}\r\n");
    let response = drive(&request("/mcp", &headers, LIST_PAGES_CALL), &req_tx);

    assert!(
        responder.join().expect("responder thread"),
        "a portless-Host CLI request must still reach the UI thread"
    );
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
}

#[test]
fn foreign_origin_tool_call_is_refused() {
    let (req_tx, req_rx) = mpsc::channel();
    // Even WITH the right token: a browser page that somehow learned the
    // token is still not an allowed caller.
    let headers = format!(
        "Host: 127.0.0.1:{PORT}\r\nOrigin: http://evil.example\r\nX-OpenPencil-Token: {TOKEN}\r\n"
    );
    let response = drive(&request("/mcp", &headers, LIST_PAGES_CALL), &req_tx);

    assert!(response.starts_with("HTTP/1.1 403 Forbidden"), "{response}");
    assert!(req_rx.try_recv().is_err(), "refused before the UI thread");

    // Its own loopback origin on its own port is the one allowed value.
    assert!(origin_allowed(
        Some(&format!("http://127.0.0.1:{PORT}")),
        PORT
    ));
    // Right host, wrong port — a different local server's page.
    assert!(!origin_allowed(Some("http://127.0.0.1:1"), PORT));
    // `localhost` is a NAME, and names are the rebinding vector.
    assert!(!origin_allowed(
        Some(&format!("http://localhost:{PORT}")),
        PORT
    ));
    assert!(!origin_allowed(Some("null"), PORT));
    assert!(origin_allowed(None, PORT), "non-browser clients pass");
}

#[test]
fn non_loopback_or_wrong_port_host_is_refused() {
    let (req_tx, req_rx) = mpsc::channel();
    // The DNS-rebinding shape: the browser resolved `evil.example` to
    // 127.0.0.1 but still writes the NAME into `Host`.
    let headers =
        format!("Host: evil.example:{PORT}\r\nX-OpenPencil-Token: {TOKEN}\r\nOrigin: http://evil.example\r\n");
    let response = drive(&request("/mcp", &headers, LIST_PAGES_CALL), &req_tx);
    assert!(response.starts_with("HTTP/1.1 403 Forbidden"), "{response}");
    assert!(req_rx.try_recv().is_err(), "refused before the UI thread");

    // A loopback literal, but a port this server never bound.
    let headers = format!(
        "Host: 127.0.0.1:{}\r\nX-OpenPencil-Token: {TOKEN}\r\n",
        PORT + 1
    );
    let response = drive(&request("/mcp", &headers, LIST_PAGES_CALL), &req_tx);
    assert!(response.starts_with("HTTP/1.1 403 Forbidden"), "{response}");
    assert!(req_rx.try_recv().is_err(), "refused before the UI thread");

    assert!(host_allowed(Some(&format!("127.0.0.1:{PORT}")), PORT));
    assert!(host_allowed(Some(&format!("[::1]:{PORT}")), PORT));
    assert!(!host_allowed(Some(&format!("localhost:{PORT}")), PORT));
    assert!(!host_allowed(Some(&format!("10.0.0.4:{PORT}")), PORT));
    assert!(!host_allowed(None, PORT), "a missing Host is refused");
}

#[test]
fn constant_time_compare_rejects_same_length_wrong_token() {
    // Same length, differing only in the last byte — the case an
    // early-exit `==` would answer faster than a first-byte mismatch.
    assert_eq!(TOKEN.len(), "d34db33f-caff".len());
    assert!(!constant_time_eq("d34db33f-caff", TOKEN));
    assert!(!constant_time_eq("e34db33f-cafe", TOKEN));
    assert!(constant_time_eq(TOKEN, TOKEN));
    assert!(!constant_time_eq("", TOKEN));
    assert!(!constant_time_eq(&format!("{TOKEN}x"), TOKEN));

    let admission = LiveAdmission::new(TOKEN.to_string(), PORT);
    let same_length_guess = format!(
        "Host: 127.0.0.1:{PORT}\r\nX-OpenPencil-Token: d34db33f-caff\r\nContent-Length: 0\r\n\r\n"
    );
    let mut cursor = std::io::Cursor::new(format!("POST /mcp HTTP/1.1\r\n{same_length_guess}"));
    let req = crate::mcp_serve::read_http_request(&mut cursor).expect("request parses");
    assert_eq!(
        check_token(&req, &admission),
        Err(AdmissionDenial::BadToken)
    );

    // An instance with no token authenticates nobody.
    let tokenless = LiveAdmission::new(String::new(), PORT);
    assert_eq!(
        check_token(&req, &tokenless),
        Err(AdmissionDenial::MissingToken)
    );
}

/// The identity probes stay tokenless on purpose: `op` discovers this
/// instance by pinging it and matching the reply's token against the
/// discovery file. Gating `ping` would break discovery for every CLI.
#[test]
fn ping_probe_stays_tokenless_for_cli_discovery() {
    let (req_tx, req_rx) = mpsc::channel();
    let headers = "Host: 127.0.0.1\r\n".to_string();
    let ping = r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#;
    let response = drive(&request("/mcp", &headers, ping), &req_tx);

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.contains(r#""mode":"live""#), "{response}");
    assert!(response.contains(TOKEN), "{response}");
    assert!(
        req_rx.try_recv().is_err(),
        "ping never touches the UI thread"
    );

    // …but a ping from a foreign origin is still refused, so a web page
    // cannot use the probe to harvest the token.
    let headers = format!("Host: 127.0.0.1:{PORT}\r\nOrigin: http://evil.example\r\n");
    let response = drive(&request("/mcp", &headers, ping), &req_tx);
    assert!(response.starts_with("HTTP/1.1 403 Forbidden"), "{response}");
    assert!(!response.contains(TOKEN), "{response}");
}

/// Whole-document sync replaces the LIVE (possibly shared) document — the
/// REST twin of the JSON-RPC write path, and equally unauthenticated
/// before this gate.
#[test]
fn document_sync_route_requires_the_instance_token() {
    let (req_tx, req_rx) = mpsc::channel();
    let headers = format!("Host: 127.0.0.1:{PORT}\r\n");
    let body = r#"{"document":{"version":"1.0","children":[],"pages":[]}}"#;
    let response = drive(&request("/api/mcp/document", &headers, body), &req_tx);

    assert!(
        response.starts_with("HTTP/1.1 401 Unauthorized"),
        "{response}"
    );
    assert!(response.contains(r#""ok":false"#), "{response}");
    assert!(
        req_rx.try_recv().is_err(),
        "an unauthenticated whole-document sync must never reach the UI thread"
    );
}

// --- browser-extension snapshot ingress (`snapshot_ingest.rs`) ---

/// A well-formed Chrome extension id: 32 characters from `a`–`p`.
const EXTENSION_ORIGIN: &str = "chrome-extension://abcdefghijklmnopabcdefghijklmnop";
const INGEST_PATH: &str = "/api/import/web-snapshot";
/// The smallest payload the v1 extractor can emit that still maps to a node.
const SNAPSHOT: &str = r#"{"version":1,"source":"https://example.com/","title":"Example","viewport":{"width":800,"height":600},"root":{"kind":"element","tag":"body","rect":{"x":0,"y":0,"w":800,"h":600},"styles":{"background-color":"rgb(255, 255, 255)"},"children":[{"kind":"element","tag":"div","rect":{"x":10,"y":10,"w":100,"h":40},"styles":{"background-color":"rgb(0, 0, 0)"},"children":[]}]}}"#;

fn extension_headers() -> String {
    format!(
        "Host: 127.0.0.1:{PORT}\r\nOrigin: {EXTENSION_ORIGIN}\r\nContent-Type: application/json\r\n"
    )
}

/// The whole point of the scoped route: an extension origin buys exactly
/// one insert-only tool, never the general tool surface.
#[test]
fn extension_origin_is_refused_outside_the_snapshot_ingress() {
    let (req_tx, req_rx) = mpsc::channel();
    let headers = format!(
        "Host: 127.0.0.1:{PORT}\r\nOrigin: {EXTENSION_ORIGIN}\r\nX-OpenPencil-Token: {TOKEN}\r\n"
    );
    let response = drive(&request("/mcp", &headers, LIST_PAGES_CALL), &req_tx);

    assert!(response.starts_with("HTTP/1.1 403 Forbidden"), "{response}");
    assert!(
        req_rx.try_recv().is_err(),
        "an extension must not reach the general tool surface"
    );
}

#[test]
fn foreign_page_origin_is_refused_on_the_snapshot_ingress() {
    let (req_tx, req_rx) = mpsc::channel();
    for origin in [
        "http://evil.example",
        "https://evil.example",
        "null",
        // Prefix-only lookalikes: wrong length, wrong alphabet, and a
        // path-carrying value that a strict origin never has.
        "chrome-extension://abc",
        "chrome-extension://ABCDEFGHIJKLMNOPABCDEFGHIJKLMNOP",
        "chrome-extension://abcdefghijklmnopabcdefghijklmnop/x",
        "chrome-extension://zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
    ] {
        let headers = format!(
            "Host: 127.0.0.1:{PORT}\r\nOrigin: {origin}\r\nContent-Type: application/json\r\n"
        );
        let response = drive(&request(INGEST_PATH, &headers, SNAPSHOT), &req_tx);
        assert!(
            response.starts_with("HTTP/1.1 403 Forbidden"),
            "{origin}: {response}"
        );
    }
    assert!(
        req_rx.try_recv().is_err(),
        "a refused ingest must never reach the UI thread"
    );
}

#[test]
fn snapshot_ingress_requires_a_json_content_type() {
    let (req_tx, req_rx) = mpsc::channel();
    let headers = format!(
        "Host: 127.0.0.1:{PORT}\r\nOrigin: {EXTENSION_ORIGIN}\r\nContent-Type: text/plain\r\n"
    );
    let response = drive(&request(INGEST_PATH, &headers, SNAPSHOT), &req_tx);

    assert!(
        response.starts_with("HTTP/1.1 415 Unsupported Media Type"),
        "{response}"
    );
    assert!(req_rx.try_recv().is_err(), "refused before the UI thread");
}

#[test]
fn snapshot_ingress_rejects_an_empty_body() {
    let (req_tx, req_rx) = mpsc::channel();
    let response = drive(&request(INGEST_PATH, &extension_headers(), ""), &req_tx);

    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request"),
        "{response}"
    );
    assert!(response.contains(r#""ok":false"#), "{response}");
    assert!(req_rx.try_recv().is_err(), "refused before the UI thread");
}

/// The end-to-end shape the Chrome extension depends on: no token, an
/// extension origin, and the snapshot lands as ONE apply on the UI thread.
#[test]
fn extension_snapshot_ingress_inserts_without_a_token() {
    let (req_tx, req_rx) = mpsc::channel();
    let responder = thread::spawn(move || {
        let mut applied = 0usize;
        while let Ok(request) = req_rx.recv_timeout(Duration::from_secs(5)) {
            match request {
                UiRequest::Snapshot { ack } => {
                    let _ = ack.send(EditorState::starter());
                }
                UiRequest::Apply { ack, .. } => {
                    applied += 1;
                    let _ = ack.send(ApplyAck { applied: true });
                    break;
                }
                _ => break,
            }
        }
        applied
    });
    let response = drive(
        &request(INGEST_PATH, &extension_headers(), SNAPSHOT),
        &req_tx,
    );

    assert_eq!(
        responder.join().expect("responder thread"),
        1,
        "the snapshot must reach the UI thread as one apply"
    );
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.contains(r#""ok":true"#), "{response}");
    assert!(response.contains("nodeCount"), "{response}");
    // The reply is readable by the ONE origin that was accepted, not by
    // every extension the browser happens to have installed.
    assert!(
        response.contains(&format!(
            "Access-Control-Allow-Origin: {EXTENSION_ORIGIN}\r\n"
        )),
        "{response}"
    );
    assert!(
        !response.contains("Access-Control-Allow-Origin: *"),
        "{response}"
    );
}

/// The preflight is what a browser consults before it will let the extension
/// POST at all, so it has to be scoped exactly like the request it precedes:
/// 204, and an `Access-Control-Allow-Origin` naming this extension only.
#[test]
fn extension_preflight_is_answered_scoped_to_that_origin() {
    let (req_tx, req_rx) = mpsc::channel();
    let headers = format!(
        "Host: 127.0.0.1:{PORT}\r\nOrigin: {EXTENSION_ORIGIN}\r\n\
         Access-Control-Request-Method: POST\r\n"
    );
    let response = drive(
        &request_with_method("OPTIONS", INGEST_PATH, &headers, ""),
        &req_tx,
    );

    assert!(
        response.starts_with("HTTP/1.1 204 No Content"),
        "{response}"
    );
    assert!(
        response.contains(&format!(
            "Access-Control-Allow-Origin: {EXTENSION_ORIGIN}\r\n"
        )),
        "{response}"
    );
    assert!(
        !response.contains("Access-Control-Allow-Origin: *"),
        "{response}"
    );
    assert!(req_rx.try_recv().is_err(), "a preflight touches no state");

    // A preflight from an origin the boundary does not accept is refused
    // outright, and carries no CORS header the caller could act on.
    let headers = format!("Host: 127.0.0.1:{PORT}\r\nOrigin: http://evil.example\r\n");
    let response = drive(
        &request_with_method("OPTIONS", INGEST_PATH, &headers, ""),
        &req_tx,
    );
    assert!(response.starts_with("HTTP/1.1 403 Forbidden"), "{response}");
    assert!(
        !response.contains("Access-Control-Allow-Origin"),
        "{response}"
    );
}

/// Fail closed on every method but `POST`: the ingest path is not a place to
/// read anything back from, and the boundary widening for extension origins
/// is method-agnostic on purpose (the preflight needs it).
#[test]
fn non_post_methods_on_the_ingest_path_are_not_served() {
    let (req_tx, req_rx) = mpsc::channel();
    for method in ["GET", "HEAD", "PUT", "DELETE"] {
        let response = drive(
            &request_with_method(method, INGEST_PATH, &extension_headers(), ""),
            &req_tx,
        );
        assert!(
            response.starts_with("HTTP/1.1 404 Not Found"),
            "{method}: {response}"
        );
    }
    assert!(
        req_rx.try_recv().is_err(),
        "no method but POST may reach the UI thread"
    );
}

/// The untokened route's own body cap, enforced from the declared
/// `Content-Length` BEFORE the body is read. The request below declares
/// 33 MiB and supplies zero bytes: if the server tried to read the body it
/// would hit EOF and `drive` would panic on the resulting transport error.
#[test]
fn oversized_declared_body_is_refused_without_reading_it() {
    let (req_tx, req_rx) = mpsc::channel();
    let oversized = snapshot_ingest::MAX_SNAPSHOT_BODY + 1;
    let raw = format!(
        "POST {INGEST_PATH} HTTP/1.1\r\n{}Content-Length: {oversized}\r\nConnection: close\r\n\r\n",
        extension_headers()
    );
    let response = drive(&raw, &req_tx);

    assert!(
        response.starts_with("HTTP/1.1 413 Payload Too Large"),
        "{response}"
    );
    assert!(response.contains(r#""ok":false"#), "{response}");
    assert!(req_rx.try_recv().is_err(), "refused before the UI thread");

    // …and a POST with no `Content-Length` at all is refused too, rather
    // than read until the peer decides to stop.
    let raw = format!(
        "POST {INGEST_PATH} HTTP/1.1\r\n{}Connection: close\r\n\r\n",
        extension_headers()
    );
    let response = drive(&raw, &req_tx);
    assert!(
        response.starts_with("HTTP/1.1 411 Length Required"),
        "{response}"
    );
    assert!(req_rx.try_recv().is_err(), "refused before the UI thread");
}

/// Host screening runs FIRST, so the extension-origin widening never
/// rescues a rebinding attempt: the browser still writes the name it dialled
/// into `Host`.
#[test]
fn foreign_host_is_refused_even_with_an_accepted_extension_origin() {
    let (req_tx, req_rx) = mpsc::channel();
    let headers = format!(
        "Host: evil.com:{PORT}\r\nOrigin: {EXTENSION_ORIGIN}\r\nContent-Type: application/json\r\n"
    );
    let response = drive(&request(INGEST_PATH, &headers, SNAPSHOT), &req_tx);

    assert!(response.starts_with("HTTP/1.1 403 Forbidden"), "{response}");
    assert!(
        response.contains("bad Host header"),
        "the Host gate, not the Origin gate, must be the one that refused: {response}"
    );
    assert!(req_rx.try_recv().is_err(), "refused before the UI thread");
}

/// Extension-id pinning (`OPENPENCIL_EXTENSION_ALLOWED_IDS`). Driven against
/// the predicate rather than the process environment so both modes are
/// covered without a global-state race between tests.
#[test]
fn extension_id_allowlist_pins_which_extensions_pass() {
    const ID: &str = "abcdefghijklmnopabcdefghijklmnop";
    const OTHER: &str = "ponmlkjihgfedcbaponmlkjihgfedcba";
    let pinned = [ID.to_string()];

    // Open mode (the shipped default while the extension is unpublished):
    // any well-formed extension origin passes…
    assert!(extension_origin_allowed(Some(EXTENSION_ORIGIN), None));
    assert!(extension_origin_allowed(
        Some(&format!("chrome-extension://{OTHER}")),
        None
    ));
    // …but the shape is still checked, in either mode.
    assert!(!extension_origin_allowed(
        Some("chrome-extension://abc"),
        None
    ));
    assert!(!extension_origin_allowed(Some("http://evil.example"), None));
    assert!(!extension_origin_allowed(None, None));

    // Pinned mode: only the listed ids.
    assert!(extension_origin_allowed(
        Some(EXTENSION_ORIGIN),
        Some(&pinned)
    ));
    assert!(!extension_origin_allowed(
        Some(&format!("chrome-extension://{OTHER}")),
        Some(&pinned)
    ));
    // An explicitly empty allowlist denies everything. `extension_id_allowlist`
    // never builds one — a blank / separator-only env var collapses to open
    // mode — so this pins the predicate, not a reachable configuration.
    assert!(!extension_origin_allowed(Some(EXTENSION_ORIGIN), Some(&[])));
}
