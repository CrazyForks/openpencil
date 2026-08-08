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

/// Longest gap between idle sweeps, however long the idle deadline is.
const MAX_SWEEP_INTERVAL_SECS: u64 = 300;

/// Opt-in to running with no persistence at all. Demo use only.
pub const EPHEMERAL_ENV: &str = "OPENPENCIL_ONLINE_EPHEMERAL";

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
    let store = super::tenant_store::TenantStore::from_env();
    // Eviction without persistence silently destroys documents: a tenant goes
    // idle, is reclaimed to free memory, and the account's work is simply
    // gone. That is defensible for a demo and indefensible for a deployment,
    // and the two are indistinguishable from inside the process — so the
    // operator has to say which one this is.
    check_persistence_configured(
        store.is_enabled(),
        ephemeral_opt_in(),
        limits.idle_evict_secs,
    )?;
    if !store.is_enabled() {
        eprintln!(
            "openpencil --serve-web --online: {EPHEMERAL_ENV} is set — evicted accounts lose \
             their documents. Never use this for a deployment."
        );
    }
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

    let registry = Arc::new(TenantRegistry::with_store(
        bound,
        limits,
        allow_origins,
        store,
    ));
    let conn_count = Arc::new(AtomicUsize::new(0));
    let shutdown = Arc::new(AtomicBool::new(false));
    spawn_sweeper(&registry, &shutdown);

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
    // The sweeper observes the same flag and retires on its next wake.
    shutdown.store(true, Ordering::Release);
    let flushed = registry.flush_all();
    eprintln!(
        "openpencil --serve-web --online: shutdown requested; flushed {flushed} account(s); \
         exiting"
    );
    Ok(())
}

/// Refuse to start a deployment that evicts accounts but keeps nothing.
///
/// Pure so the decision is testable without a socket or the environment. The
/// two states this separates — "a demo that intentionally forgets" and "a
/// deployment whose data directory was left unset" — look identical from
/// inside the process, and only one of them is acceptable. So the operator
/// has to say which, rather than the daemon guessing and silently destroying
/// documents on the first idle timer.
pub(super) fn check_persistence_configured(
    store_enabled: bool,
    ephemeral: bool,
    idle_evict_secs: u64,
) -> Result<()> {
    if store_enabled || ephemeral {
        return Ok(());
    }
    Err(WebCanvasError::Config(format!(
        "--online evicts idle accounts after {idle_evict_secs}s but no {} is configured, so \
         their documents would be discarded. Set {} to persist them, or set {EPHEMERAL_ENV}=1 \
         to accept that this deployment keeps nothing.",
        super::tenant_store::DATA_DIR_ENV,
        super::tenant_store::DATA_DIR_ENV,
    )))
}

/// Whether the operator accepted a deployment that persists nothing.
fn ephemeral_opt_in() -> bool {
    std::env::var(EPHEMERAL_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

/// Run the idle sweep on its own clock.
///
/// Eviction used to piggyback on connection arrivals, which meant the one
/// state it exists for — a daemon nobody is talking to — was exactly the state
/// it never ran in: idle accounts stayed resident indefinitely and were never
/// written to disk. This thread observes `shutdown` on every wake, so it
/// retires within one interval of the daemon being asked to stop.
fn spawn_sweeper(registry: &Arc<TenantRegistry>, shutdown: &Arc<AtomicBool>) {
    let registry = Arc::clone(registry);
    let shutdown = Arc::clone(shutdown);
    let interval = sweep_interval_secs(registry.limits().idle_evict_secs);
    let spawned = thread::Builder::new()
        .name("op-serve-web-online-sweeper".into())
        .spawn(move || {
            while !shutdown.load(Ordering::Acquire) {
                std::thread::sleep(std::time::Duration::from_secs(interval));
                if shutdown.load(Ordering::Acquire) {
                    return;
                }
                let evicted = registry.evict_idle(now_unix());
                if evicted > 0 {
                    eprintln!(
                        "openpencil --serve-web --online: reclaimed {evicted} idle account(s)"
                    );
                }
            }
        });
    if spawned.is_err() {
        eprintln!(
            "openpencil --serve-web --online: could not start the idle sweeper; accounts will \
             stay resident until a connection triggers a sweep"
        );
    }
}

/// How often to sweep, given the idle deadline.
///
/// A quarter of the deadline bounds how long past its timer a tenant can
/// linger, and the ceiling keeps a very long deadline from meaning the daemon
/// effectively never sweeps. The floor keeps a short test deadline from
/// spinning the thread.
pub(super) const fn sweep_interval_secs(idle_evict_secs: u64) -> u64 {
    let quarter = idle_evict_secs / 4;
    if quarter < 1 {
        1
    } else if quarter > MAX_SWEEP_INTERVAL_SECS {
        MAX_SWEEP_INTERVAL_SECS
    } else {
        quarter
    }
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
    // Which tenant is this request addressed to? The query names an OWNER;
    // whether this caller may reach it is still decided by the owner's access
    // list, never by the parameter. Absent, it is the caller's own tenant.
    let requested_owner = req
        .query
        .as_deref()
        .and_then(op_editor_core::share_routes::tenant_from_query)
        .map(str::to_string);
    let lease = match requested_owner.as_deref() {
        Some(owner) => registry.lease_for_shared(owner, &identity),
        None => registry.lease_for(&identity),
    };
    let lease = match lease {
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
    // Sharing is administered on the CALLER's own tenant, so it is answered
    // here rather than in the document tier — a visitor holding a `?tenant=`
    // lease on someone else's document must not be able to re-share it.
    if super::share_routes::is_share_route(&req.path) {
        let own = match registry.lease_for(&identity) {
            Ok(own) => own,
            Err(error) => {
                crate::mcp_serve::write_mcp_http_response_with_origin(
                    stream,
                    error.http_status(),
                    &serde_json::json!({
                        "ok": false,
                        "error": error.code(),
                        "message": error.to_string(),
                    })
                    .to_string(),
                    cors_origin.as_deref(),
                )?;
                return Ok(false);
            }
        };
        let reply = super::share_routes::handle(
            &req.method,
            &req.path,
            &req.body,
            &identity,
            &own,
            registry,
        );
        crate::mcp_serve::write_mcp_http_response_with_origin(
            stream,
            reply.status,
            &reply.body,
            cors_origin.as_deref(),
        )?;
        return Ok(false);
    }
    // The lease outlives the dispatch (including a minutes-long SSE stream),
    // so the tenant these borrows point at cannot be evicted underneath them.
    dispatch(
        stream,
        &req,
        &ConnCtx {
            state: lease.state(),
            hub: lease.hub(),
            mode: ServeMode::Online,
            // The public tool profile, narrowed further by whatever scopes
            // this particular credential carries.
            mcp_profile: crate::mcp_serve::tool_profile::McpAccessProfile::online(identity.scopes),
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
