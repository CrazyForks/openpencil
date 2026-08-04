//! Endpoint parsing, normalization and validation.
//!
//! This is the extension's outbound-destination guard. It is deliberately a
//! whitelist of loopback literals rather than a URL parser, for two
//! independent reasons:
//!
//! * The extension must never be talked into POSTing a capture of the page
//!   you are on (cookies, form values, and rasterized images included) to an
//!   arbitrary host. Restricting the parse is the guard; the manifest's
//!   loopback-only `host_permissions` is the backstop.
//! * The live editor's admission gate refuses any `Host` header that is not a
//!   numeric loopback literal (a DNS name is exactly what a rebinding attack
//!   supplies), so `localhost:3100` would be answered with 403 even though it
//!   resolves to the same socket — hence the rewrite to `127.0.0.1`.

use crate::js_text::js_trim;

/// Endpoint the popup offers before the user has stored one.
pub const DEFAULT_ENDPOINT: &str = "127.0.0.1:3100";

/// The insert-only snapshot route on the desktop app's live MCP endpoint.
const INGEST_PATH: &str = "/api/import/web-snapshot";
/// The general MCP surface, used only as the unmanaged-daemon fallback.
const MCP_PATH: &str = "/mcp";

/// Hosts an endpoint may name. `localhost` is accepted and rewritten; the
/// two numeric literals pass through unchanged.
const LOOPBACK_HOSTS: [&str; 3] = ["127.0.0.1", "[::1]", "localhost"];

/// Normalize a user-typed endpoint to `host:port`.
///
/// Accepts an optional `http://` / `https://` prefix and trailing slashes,
/// exactly as the JS predecessor's
/// `/^(127\.0\.0\.1|\[::1\]|localhost):([0-9]{1,5})$/i` did after stripping
/// them.
///
/// Returns `None` when the input is unparseable or names anything but
/// loopback — the caller must refuse to send.
pub fn normalize_endpoint(raw: &str) -> Option<String> {
    let trimmed = js_trim(raw);
    if trimmed.is_empty() {
        return None;
    }
    let without_scheme = strip_trailing_slashes(strip_scheme(trimmed));

    // Split on the LAST colon: `[::1]:3100` carries colons inside the host.
    // Any leftover colon in the host part fails the whitelist below, which is
    // what the anchored JS regex did too.
    let (host, port_text) = without_scheme.rsplit_once(':')?;

    // `[0-9]{1,5}` — no sign, no whitespace, no leading `+`, 1..=5 digits.
    if port_text.is_empty() || port_text.len() > 5 || !port_text.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let port: u32 = port_text.parse().ok()?;
    if !(1..=65535).contains(&port) {
        return None;
    }

    let matched = LOOPBACK_HOSTS
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(host))?;
    // The JS regex carried the `i` flag, so `LOCALHOST` was accepted; only
    // `localhost` has letters, so case folding matters for it alone.
    let host = if matched.eq_ignore_ascii_case("localhost") {
        "127.0.0.1"
    } else {
        *matched
    };
    Some(format!("{host}:{port}"))
}

/// Absolute URL of the desktop app's insert-only snapshot ingest route.
///
/// `endpoint` must already have passed [`normalize_endpoint`]; the caller
/// never has a reason to hold an un-normalized one.
pub fn ingest_url(endpoint: &str) -> String {
    format!("http://{endpoint}{INGEST_PATH}")
}

/// Absolute URL of the general MCP surface (the unmanaged-daemon fallback).
pub fn mcp_url(endpoint: &str) -> String {
    format!("http://{endpoint}{MCP_PATH}")
}

/// Drop a single leading `http://` or `https://`, case-insensitively.
fn strip_scheme(s: &str) -> &str {
    for scheme in ["http://", "https://"] {
        // `get` rather than a slice index: the prefix length can land inside
        // a multi-byte character, which would panic on `&s[..n]`.
        if s.get(..scheme.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(scheme))
        {
            return &s[scheme.len()..];
        }
    }
    s
}

fn strip_trailing_slashes(s: &str) -> &str {
    s.trim_end_matches('/')
}
