//! Admission control for the live-GUI MCP endpoint (`127.0.0.1:<port>/mcp`).
//!
//! The live endpoint drives the on-screen document, and during a
//! collaboration session that document is the SHARED one. Before this
//! module existed the per-instance token authenticated exactly two
//! messages — the `ping` identity probe's reply and `openpencil/shutdown`
//! — so every read and every write tool call was reachable by any local
//! process, and (with no `Origin` / `Host` screening) by any web page on
//! the machine via DNS rebinding. That is a straight bypass of the
//! collaboration admission model: `CollabGatePolicy` can refuse an MCP
//! *mutation* mid-session, but it never saw reads, and it is not a
//! boundary — it is the last line behind one.
//!
//! Two independent gates, in this order:
//!
//! 1. [`check_boundary`] — browser screening. A `Host` that is not a
//!    numeric loopback literal on the bound port, or ANY `Origin` other
//!    than this instance's own loopback origin, is refused. Both headers
//!    are browser-controlled but not page-forgeable, which is what
//!    actually closes DNS rebinding: a rebound `evil.com` page still
//!    sends `Host: evil.com:<port>` and `Origin: http://evil.com`.
//!    Requests with no `Origin` at all are normal non-browser clients
//!    (the `op` CLI, the VS Code MCP proxy) and pass.
//! 2. [`check_token`] — the per-instance token from the
//!    `X-OpenPencil-Token` header, compared in constant time. This is the
//!    same credential the server already published to its clients (the
//!    discovery file `~/.openpencil/.op-mcp-port` and the `ping` reply)
//!    and the same header the managed web daemon uses
//!    (`web_canvas_server::RequestAuth`) — no new credential system.
//!
//! Deliberately still tokenless: `OPTIONS` preflight, and the stateless
//! `initialize` / `notifications/initialized` / `ping` probes, which carry
//! no document data and are how a client discovers this instance in the
//! first place. `openpencil/shutdown` keeps its own body-carried token
//! check (`mcp_serve::shutdown_request_id`), unchanged.
//!
//! KNOWN RESIDUAL (documented, not introduced here): the `ping` reply
//! hands the token to any local caller, so this gate raises the bar for a
//! local process rather than closing it outright. Closing that needs a
//! change to the `op` CLI's discovery handshake, which lives in another
//! crate.

use std::fmt;

/// JSON-RPC error code for a refused request. Server-defined range
/// (-32000..=-32099), one step away from the -32000 this endpoint already
/// uses for "server busy" / "Invalid or missing session ID".
const DENIED_CODE: i32 = -32001;

/// Per-instance admission material for the live endpoint: the token the
/// server published and the port it actually bound. Shared by every
/// connection thread (`Arc`), immutable for the life of the server.
pub(super) struct LiveAdmission {
    token: String,
    port: u16,
}

impl LiveAdmission {
    pub(super) fn new(token: String, port: u16) -> Self {
        Self { token, port }
    }

    /// The per-instance identity token — also what the `ping` reply and
    /// the `openpencil/shutdown` check use, so the wire contract the CLI
    /// already knows is preserved verbatim.
    pub(super) fn token(&self) -> &str {
        &self.token
    }

    pub(super) fn port(&self) -> u16 {
        self.port
    }
}

/// Why a request was refused. A typed enum rather than a `String` (the
/// workspace rule) — and deliberately NOT an `McpLiveError` variant: every
/// value here is a *client* fault answered on the wire with a 401/403 and
/// a JSON-RPC error body, never a server fault the accept loop logs and
/// turns into a 500.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AdmissionDenial {
    /// `Host` absent, not a numeric loopback literal, or carrying a port
    /// other than the one this server bound.
    ForeignHost,
    /// An `Origin` header that is not this instance's own loopback origin.
    ForeignOrigin,
    /// No `X-OpenPencil-Token` header (or an empty one).
    MissingToken,
    /// A token that is not this instance's token.
    BadToken,
}

impl AdmissionDenial {
    /// HTTP status line. Boundary failures are 403 (the caller is not
    /// allowed to talk to this endpoint at all); token failures are 401
    /// (the caller may retry with the credential it was given).
    pub(super) fn http_status(self) -> &'static str {
        match self {
            AdmissionDenial::ForeignHost | AdmissionDenial::ForeignOrigin => "403 Forbidden",
            AdmissionDenial::MissingToken | AdmissionDenial::BadToken => "401 Unauthorized",
        }
    }

    /// Client-facing reason. Intentionally coarse: it names the gate, not
    /// which byte of the token differed.
    pub(super) fn message(self) -> &'static str {
        match self {
            AdmissionDenial::ForeignHost => {
                "live MCP endpoint accepts loopback requests only (bad Host header)"
            }
            AdmissionDenial::ForeignOrigin => {
                "live MCP endpoint refuses cross-origin requests (bad Origin header)"
            }
            AdmissionDenial::MissingToken => {
                "live MCP endpoint requires the X-OpenPencil-Token header"
            }
            AdmissionDenial::BadToken => "live MCP endpoint token mismatch",
        }
    }
}

impl fmt::Display for AdmissionDenial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

/// Gate 1 — browser screening, applied to EVERY request (including the
/// stateless probes and the REST document-sync route) before any routing.
pub(super) fn check_boundary(
    req: &crate::mcp_serve::HttpRequest,
    admission: &LiveAdmission,
) -> Result<(), AdmissionDenial> {
    if !host_allowed(req.host.as_deref(), admission.port()) {
        return Err(AdmissionDenial::ForeignHost);
    }
    if !origin_allowed(req.origin.as_deref(), admission.port()) {
        return Err(AdmissionDenial::ForeignOrigin);
    }
    Ok(())
}

/// Gate 2 — per-instance token, required by every request that can
/// observe or mutate the live document.
pub(super) fn check_token(
    req: &crate::mcp_serve::HttpRequest,
    admission: &LiveAdmission,
) -> Result<(), AdmissionDenial> {
    if admission.token().is_empty() {
        // A server with no token can authenticate nobody. Refusing is the
        // only safe reading — the alternative ("empty means open") is the
        // bug this module exists to remove.
        return Err(AdmissionDenial::MissingToken);
    }
    match req.token.as_deref() {
        // No header at all, or a present-but-blank one: the same "brought
        // no credential" case, reported as such rather than as a mismatch.
        None | Some("") => Err(AdmissionDenial::MissingToken),
        Some(presented) if constant_time_eq(presented, admission.token()) => Ok(()),
        Some(_) => Err(AdmissionDenial::BadToken),
    }
}

/// Non-early-exit byte compare.
///
/// `subtle` is NOT a dependency of `op-host-services` (only
/// `op-host-desktop` and the two relay crates carry it), and adding one
/// would mean editing a manifest a concurrent session owns — so this is
/// the hand-rolled equivalent: fold every byte difference into a single
/// accumulator, no `return` inside the loop, no data-dependent branch, one
/// comparison at the end. Length is compared up front on purpose: the
/// token's shape is public (`{pid:x}-{nanos:x}`, see `make_live_token`),
/// so its length is not a secret; what must not leak is HOW MANY leading
/// bytes of a same-length guess were right.
pub(super) fn constant_time_eq(presented: &str, expected: &str) -> bool {
    let (presented, expected) = (presented.as_bytes(), expected.as_bytes());
    if presented.len() != expected.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in presented.iter().zip(expected.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// JSON-RPC error body for a refused `/mcp` request, echoing the caller's
/// request id so a client correlates the refusal with its call instead of
/// hanging (same discipline as `op_mcp::parser`'s parse-failure path).
pub(super) fn denial_json_rpc(request_body: &str, denial: AdmissionDenial) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","error":{{"code":{DENIED_CODE},"message":"{}"}},"id":{}}}"#,
        crate::mcp_serve::json_escape(denial.message()),
        request_id_raw(request_body)
    )
}

/// REST-shaped denial body for the `POST /api/mcp/document` route, which
/// speaks `{ok,error}` rather than JSON-RPC.
pub(super) fn denial_rest(denial: AdmissionDenial) -> String {
    crate::mcp_serve::rest_error_body(denial.message())
}

/// The caller's top-level JSON-RPC `id`, verbatim (so a string id stays a
/// string id), or `null` when the body is not a JSON object / has no id.
fn request_id_raw(request_body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(request_body)
        .ok()
        .and_then(|value| value.get("id").map(|id| id.to_string()))
        .unwrap_or_else(|| "null".to_string())
}

/// `Host` must name a numeric loopback address. A DNS name — including
/// `localhost` — is refused: rebinding attacks work precisely by pointing
/// a name at 127.0.0.1, and a browser writes the name it was given into
/// `Host`, so accepting names would leave the hole open.
///
/// The port is checked when present. It may be absent: `op`'s own
/// transport sends a bare `Host: 127.0.0.1`
/// (`op_rpc_transport::TcpJsonRpc::http_post_request`), and a browser can
/// never produce that against a non-80 port — it always writes the target
/// port it dialled. So "no port" identifies a non-browser client rather
/// than widening the browser surface.
fn host_allowed(host: Option<&str>, expected_port: u16) -> bool {
    // HTTP/1.1 requires `Host`; every browser and every client in this
    // repo sends it. Absent ⇒ refuse rather than guess.
    let Some(raw) = host else {
        return false;
    };
    let Some((host, port)) = split_authority(raw.trim()) else {
        return false;
    };
    is_numeric_loopback(host) && port.is_none_or(|port| port == expected_port)
}

/// Any `Origin` other than this instance's own loopback origin is refused.
/// `None` is the normal non-browser case (CLI / proxy) and passes — a page
/// cannot suppress the header on a cross-origin request, so "no Origin"
/// is not something an attacker page can claim.
fn origin_allowed(origin: Option<&str>, expected_port: u16) -> bool {
    let Some(origin) = origin else {
        return true;
    };
    let origin = origin.trim();
    // The live endpoint is plain HTTP on loopback, so only an `http://`
    // origin can possibly be it; `null` (sandboxed iframe / `file://`)
    // and any `https://` page fall through to a refusal.
    let Some(authority) = origin.strip_prefix("http://") else {
        return false;
    };
    // A real serialized origin is scheme + authority and nothing else.
    if authority.contains(['/', '@', '?', '#']) {
        return false;
    }
    let Some((host, port)) = split_authority(authority) else {
        return false;
    };
    is_numeric_loopback(host) && port.unwrap_or(80) == expected_port
}

/// Split an HTTP authority (`127.0.0.1:3100`, `127.0.0.1`, `[::1]:3100`)
/// into host and optional port. A malformed port refuses the whole value.
fn split_authority(value: &str) -> Option<(&str, Option<u16>)> {
    if let Some(rest) = value.strip_prefix('[') {
        let (inside, tail) = rest.split_once(']')?;
        return match tail {
            "" => Some((inside, None)),
            tail => {
                let port = tail.strip_prefix(':')?.parse::<u16>().ok()?;
                Some((inside, Some(port)))
            }
        };
    }
    match value.rsplit_once(':') {
        // An unbracketed IPv6 literal lands here with a nonsense split;
        // `is_numeric_loopback` then rejects the truncated host, which is
        // correct — unbracketed IPv6 in an authority is malformed anyway.
        Some((host, port)) => Some((host, Some(port.parse::<u16>().ok()?))),
        None => Some((value, None)),
    }
}

/// A numeric IP literal in a loopback range (127.0.0.0/8 or `::1`).
/// Names never qualify.
fn is_numeric_loopback(host: &str) -> bool {
    host.parse::<std::net::IpAddr>()
        .is_ok_and(|address| address.is_loopback())
}

#[cfg(test)]
#[path = "admission_tests.rs"]
mod tests;
