//! The public multi-account accept loop (`--serve-web --online`).
//!
//! Structurally the same daemon as [`run_web_canvas`](super::run_web_canvas):
//! bind, thread per connection, bounded concurrency. The difference is where
//! the document comes from. The single-user loop builds one
//! `Mutex<WebCanvasState>` up front and hands every connection the same one;
//! this loop builds none, and each connection resolves its own from the
//! identity its credentials verify to.
//!
//! ## Order of operations, and why
//!
//! 1. Read the request.
//! 2. Answer `OPTIONS` and the static bundle **anonymously** — the page has
//!    to load before the browser can present a credential, and neither
//!    branch touches a document.
//! 3. Verify the credential.
//! 4. Take a tenant lease.
//! 5. Dispatch.
//!
//! Nothing between steps 1 and 4 reads or writes any account's state, so an
//! unauthenticated caller can reach exactly the bytes the CDN would serve.
//!
//! ## What this loop does not run
//!
//! No collaboration driver. Relay collaboration needs a per-account device
//! ticket and the bridge holds one per process, so
//! `ServeMode::allows_relay_collaboration` is false online and every tenant's
//! availability stays at its default `Unavailable` — which is exactly what a
//! never-ticked runtime reports. M4 brings up in-service (Tier 1) sessions,
//! at which point the driver becomes per-tenant: it will walk the registry's
//! leased tenants (or run one lazy thread per live session) rather than
//! pumping a single global state. That is the extension point.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use super::online_policy::ServeMode;
use super::tenant::{now_unix, TenantLimits, TenantRegistry};
use super::tenant_auth::{IdentityVerifier, PresentedCredentials, StaticVerifier};
use super::*;

/// How often the accept loop sweeps for idle tenants.
///
/// Eviction is cheap (a map scan under one lock) and the deadline it enforces
/// is measured in minutes, so a sweep tied to connection arrivals would be
/// both too eager under load and never under idle. This is the floor between
/// sweeps, checked as connections arrive.
const EVICT_SWEEP_INTERVAL_SECS: u64 = 60;

/// Run the multi-account web-canvas daemon.
///
/// `options.path` is ignored: a tenant is never backed by a local file, and
/// booting every account from one operator-chosen document would put one
/// account's content in front of all of them.
pub fn run_online_web_canvas(options: ServeWebOptions) -> Result<()> {
    let ServeWebOptions {
        port, host, path, ..
    } = options;
    if path.is_some() {
        eprintln!(
            "openpencil --serve-web --online: ignoring the start-up document — every account \
             starts from the starter document"
        );
    }
    let limits = TenantLimits::from_env();
    let allow_origins = online_policy::allowed_origins_from_env();
    if allow_origins.is_empty() {
        // Not fatal: a same-origin deployment behind a reverse proxy needs no
        // CORS header at all. It IS fatal for cookie-authenticated writes,
        // which have no other CSRF boundary — so say so rather than letting
        // the first failed save be the discovery.
        eprintln!(
            "openpencil --serve-web --online: no public origin configured (set {}); \
             cookie-authenticated writes will be refused",
            super::origin_guard::WEB_ALLOWED_ORIGINS_ENV
        );
    }
    let verifier = resolve_verifier();

    let listener = TcpListener::bind((host.as_str(), port))
        .map_err(|e| WebCanvasError::Config(format!("bind {host}:{port}: {e}")))?;
    let local_addr = listener
        .local_addr()
        .map_err(|e| WebCanvasError::Config(e.to_string()))?;
    let bound = local_addr.port();
    eprintln!("openpencil --serve-web --online: listening on {host}:{bound}");
    eprintln!(
        "openpencil --serve-web --online: limits — {} conns, {} per account, {} accounts, \
         {}s idle eviction",
        limits.max_conns, limits.max_conns_per_tenant, limits.max_tenants, limits.idle_evict_secs
    );
    match crate::web_static::resolve_bundle_dir() {
        Some(dir) => eprintln!(
            "openpencil --serve-web --online: serving web bundle from {}",
            dir.display()
        ),
        None => eprintln!(
            "openpencil --serve-web --online: no web bundle found — `/` serves build \
             instructions (tools/check-wasm-bundle.sh, or set OPENPENCIL_WEB_BUNDLE_DIR)"
        ),
    }

    let registry = Arc::new(TenantRegistry::new(bound, limits, allow_origins));
    let conn_count = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false));
    let mut last_sweep = now_unix();

    for stream in listener.incoming() {
        if shutdown.load(Ordering::Acquire) {
            break;
        }
        let mut s = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("openpencil --serve-web --online: accept: {e}");
                continue;
            }
        };
        let now = now_unix();
        if now.saturating_sub(last_sweep) >= EVICT_SWEEP_INTERVAL_SECS {
            last_sweep = now;
            registry.evict_idle(now);
        }
        if conn_count.load(Ordering::Acquire) >= limits.max_conns {
            let _ = s.set_write_timeout(Some(IO_TIMEOUT));
            let _ = crate::mcp_serve::write_mcp_http_response(
                &mut s,
                "503 Service Unavailable",
                r#"{"ok":false,"error":"server busy"}"#,
            );
            continue;
        }
        conn_count.fetch_add(1, Ordering::AcqRel);
        let registry = Arc::clone(&registry);
        let verifier = Arc::clone(&verifier);
        let conns = Arc::clone(&conn_count);
        let shutdown_flag = Arc::clone(&shutdown);
        let spawned = thread::Builder::new()
            .name("op-serve-web-online-conn".into())
            .spawn(move || {
                let _conn_guard = ConnGuard(conns);
                let _ = s.set_read_timeout(Some(IO_TIMEOUT));
                let _ = s.set_write_timeout(Some(IO_TIMEOUT));
                match serve_one_online(&mut s, registry.as_ref(), verifier.as_ref()) {
                    Ok(true) => {
                        shutdown_flag.store(true, Ordering::Release);
                        let _ = std::net::TcpStream::connect(("127.0.0.1", bound));
                    }
                    Ok(false) => {}
                    Err(e) => eprintln!("openpencil --serve-web --online: {e}"),
                }
            });
        if spawned.is_err() {
            conn_count.fetch_sub(1, Ordering::AcqRel);
        }
    }
    eprintln!("openpencil --serve-web --online: shutdown requested; exiting");
    Ok(())
}

/// Pick the identity verifier this deployment runs.
///
/// The hub is the production answer. `StaticVerifier` stays reachable so the
/// M1 development smoke still works with no hub in sight, and so a
/// misconfigured hub URL does not silently downgrade to it — a hub that is
/// configured but unbuildable is a hard failure, not a fallback.
fn resolve_verifier() -> Arc<dyn IdentityVerifier> {
    match super::hub_verifier::HubVerifier::from_env() {
        Ok(Some(verifier)) => {
            eprintln!(
                "openpencil --serve-web --online: verifying identities against the hub at {}",
                std::env::var(crate::hub_auth_client::HUB_BASE_URL_ENV).unwrap_or_default()
            );
            if crate::hub_auth_client::internal_auth_from_env().is_none() {
                eprintln!(
                    "openpencil --serve-web --online: no {} configured; API-token \
                     introspection will be refused and only browser sessions will work",
                    crate::hub_auth_client::HUB_INTERNAL_AUTH_FILE_ENV
                );
            }
            return Arc::new(verifier);
        }
        Ok(None) => {}
        Err(error) => {
            // Configured but unusable. Serving with the development verifier
            // here would mean a production deployment quietly accepting an
            // env token table instead of real accounts.
            eprintln!(
                "openpencil --serve-web --online: {} is set but unusable ({error}); every \
                 authenticated route will answer 503",
                crate::hub_auth_client::HUB_BASE_URL_ENV
            );
            return Arc::new(StaticVerifier::parse(""));
        }
    }
    let static_verifier = StaticVerifier::from_env();
    if static_verifier.is_empty() {
        // Fail loud but keep serving: every request answers 503
        // `verifier-unavailable`, which is a diagnosable state. Serving
        // requests with NO verifier would be the unsafe alternative.
        eprintln!(
            "openpencil --serve-web --online: no identity verifier configured (set {} for a \
             hub deployment, or {} for development); every authenticated route will answer 503",
            crate::hub_auth_client::HUB_BASE_URL_ENV,
            super::tenant_auth::STATIC_IDENTITIES_ENV
        );
    } else {
        eprintln!(
            "openpencil --serve-web --online: using the DEVELOPMENT static identity table; \
             set {} for a real deployment",
            crate::hub_auth_client::HUB_BASE_URL_ENV
        );
    }
    Arc::new(static_verifier)
}

/// Serve one online connection: anonymous prefix, then verify, then dispatch
/// against the caller's own tenant.
///
/// Returns `Ok(true)` on an accepted operator shutdown, exactly as
/// [`serve_one`](super::serve_one) does.
pub(super) fn serve_one_online<S: Read + Write>(
    stream: &mut S,
    registry: &TenantRegistry,
    verifier: &dyn IdentityVerifier,
) -> Result<bool> {
    let req = crate::mcp_serve::read_http_request(stream)?;
    let allow_origins = registry.allow_origins();
    let cors_origin = online_policy::online_cors_origin(allow_origins, req.origin.as_deref());
    if let Some(done) = serve_anonymous_prefix(stream, &req, cors_origin.as_deref())? {
        return Ok(done);
    }
    // The tenant key comes from here and nowhere else. Note that the request
    // body has not been looked at yet, and never contributes.
    let identity = match verifier.resolve(&PresentedCredentials::from_request(&req)) {
        Ok(identity) => identity,
        Err(error) => {
            crate::mcp_serve::write_mcp_http_response(
                stream,
                error.http_status(),
                &serde_json::json!({
                    "ok": false,
                    "error": error.code(),
                    "message": error.to_string(),
                })
                .to_string(),
            )?;
            return Ok(false);
        }
    };
    // CSRF boundary. A session cookie rides along on any cross-site request
    // the browser makes, so on a public origin the cookie alone does not
    // establish that this deployment's own page initiated the write. A bearer
    // token is exempt: it is only ever attached by code that already holds it.
    if identity.via == super::tenant_auth::IdentityVia::SessionCookie
        && !matches!(req.method.as_str(), "GET" | "HEAD" | "OPTIONS")
        && !online_policy::cookie_write_origin_allowed(allow_origins, req.origin.as_deref())
    {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "403 Forbidden",
            &serde_json::json!({
                "ok": false,
                "error": "cross-origin-write-forbidden",
                "message": "a cookie-authenticated write must come from this deployment's origin",
            })
            .to_string(),
            cors_origin.as_deref(),
        )?;
        return Ok(false);
    }
    let lease = match registry.lease_for(&identity) {
        Ok(lease) => lease,
        Err(error) => {
            crate::mcp_serve::write_mcp_http_response(
                stream,
                error.http_status(),
                &serde_json::json!({
                    "ok": false,
                    "error": error.code(),
                    "message": error.to_string(),
                })
                .to_string(),
            )?;
            return Ok(false);
        }
    };
    // The lease outlives the dispatch (including a minutes-long SSE stream),
    // so the tenant these borrows point at cannot be evicted underneath them.
    dispatch(
        stream,
        &req,
        &ConnCtx {
            state: lease.state(),
            hub: lease.hub(),
            mode: ServeMode::Online,
        },
    )
}

/// The part of the route table that is served before anyone is identified.
///
/// `Ok(None)` means the request still needs an identity. Every branch here is
/// either a CORS preflight or a static asset — nothing that reads or writes an
/// account's document — because the browser cannot present a session until the
/// page that holds it has loaded.
fn serve_anonymous_prefix<S: Read + Write>(
    stream: &mut S,
    req: &crate::mcp_serve::HttpRequest,
    cors_origin: Option<&str>,
) -> Result<Option<bool>> {
    if req.method == "OPTIONS" {
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            "204 No Content",
            "",
            cors_origin,
        )?;
        return Ok(Some(false));
    }
    if req.method == "GET" {
        let bundle_dir = crate::web_static::resolve_bundle_dir();
        if let Some(reply) =
            crate::web_static::handle_static_request(&req.path, bundle_dir.as_deref())
        {
            return crate::web_static::write_static_response(stream, &reply, cors_origin)
                .map(|()| Some(false));
        }
    }
    Ok(None)
}

#[cfg(test)]
#[path = "online_run_loop_tests.rs"]
mod tests;
