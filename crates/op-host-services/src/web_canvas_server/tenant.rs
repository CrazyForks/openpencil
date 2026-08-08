//! One document authority per account.
//!
//! The local daemon has exactly one `Mutex<WebCanvasState>` and one
//! [`SseHub`]; the online daemon has one of each **per tenant**, and a
//! request reaches the pair belonging to whoever was verified for that
//! connection. Everything that used to be process-wide state and is still
//! process-wide is refused by [`ServeMode`](super::online_policy::ServeMode)
//! instead of being shared.
//!
//! ## Why a lease and not a timestamp
//!
//! Eviction has to reclaim idle tenants without ever pulling one out from
//! under a live connection: a request that is mid-flight holds a `&Mutex<..>`
//! into the tenant, and an SSE stream holds it for minutes. So a connection
//! takes a [`TenantLease`] for its whole lifetime, and eviction only removes
//! tenants whose lease count is zero. Both the lease increment and the
//! eviction check happen under the registry lock, so there is no window where
//! a lease is being taken while its tenant is being removed.
//!
//! ## Why removal is a compare-and-remove
//!
//! M4 will persist a tenant's document as it is evicted. If eviction removed
//! the map entry first and wrote afterwards, a returning request could create
//! a *second*, empty tenant for the same account while the first is still
//! being written — two live authorities for one document, and whichever
//! finished last wins. [`TenantRegistry::evict_idle`] therefore resolves the
//! victim, and removes it only after re-checking that the entry in the map is
//! still the exact `Arc` it decided to evict.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use op_editor_core::EditorState;

use super::tenant_auth::ResolvedIdentity;
use super::{SseHub, WebCanvasState};

/// Global connection ceiling for the online daemon.
pub const MAX_CONNS_ENV: &str = "OPENPENCIL_ONLINE_MAX_CONNS";
/// Per-tenant connection ceiling — one account cannot starve the rest.
pub const MAX_CONNS_PER_TENANT_ENV: &str = "OPENPENCIL_ONLINE_MAX_CONNS_PER_TENANT";
/// How many accounts may hold a live document at once.
pub const MAX_TENANTS_ENV: &str = "OPENPENCIL_ONLINE_MAX_TENANTS";
/// Idle seconds after which a tenant with no connections is reclaimed.
pub const IDLE_EVICT_SECS_ENV: &str = "OPENPENCIL_ONLINE_IDLE_EVICT_SECS";

pub const DEFAULT_MAX_CONNS: usize = 256;
pub const DEFAULT_MAX_CONNS_PER_TENANT: usize = 8;
pub const DEFAULT_MAX_TENANTS: usize = 100;
pub const DEFAULT_IDLE_EVICT_SECS: u64 = 1800;

/// Resource ceilings for one online daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TenantLimits {
    pub max_conns: usize,
    pub max_conns_per_tenant: usize,
    pub max_tenants: usize,
    pub idle_evict_secs: u64,
}

impl Default for TenantLimits {
    fn default() -> Self {
        Self {
            max_conns: DEFAULT_MAX_CONNS,
            max_conns_per_tenant: DEFAULT_MAX_CONNS_PER_TENANT,
            max_tenants: DEFAULT_MAX_TENANTS,
            idle_evict_secs: DEFAULT_IDLE_EVICT_SECS,
        }
    }
}

impl TenantLimits {
    /// Read the ceilings from the environment, falling back to the defaults.
    ///
    /// An unparseable or zero value keeps the default rather than disabling
    /// the ceiling: a typo in a deployment variable must not silently remove
    /// a resource bound.
    pub fn from_env() -> Self {
        let defaults = Self::default();
        Self {
            max_conns: env_usize(MAX_CONNS_ENV, defaults.max_conns),
            max_conns_per_tenant: env_usize(
                MAX_CONNS_PER_TENANT_ENV,
                defaults.max_conns_per_tenant,
            ),
            max_tenants: env_usize(MAX_TENANTS_ENV, defaults.max_tenants),
            idle_evict_secs: env_u64(IDLE_EVICT_SECS_ENV, defaults.idle_evict_secs),
        }
    }
}

fn env_usize(name: &str, fallback: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

fn env_u64(name: &str, fallback: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

/// Why the registry could not hand out a tenant.
///
/// Both variants are capacity verdicts, never a statement about the
/// requesting account — a caller learns that the service is full, not
/// anything about other tenants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenantError {
    /// The daemon already holds [`TenantLimits::max_tenants`] documents and
    /// none of them is idle enough to reclaim.
    TooManyTenants,
    /// This account already holds [`TenantLimits::max_conns_per_tenant`]
    /// connections.
    TooManyConnections,
}

impl TenantError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::TooManyTenants => "server-busy",
            Self::TooManyConnections => "too-many-connections",
        }
    }

    pub const fn http_status(self) -> &'static str {
        "503 Service Unavailable"
    }
}

impl std::fmt::Display for TenantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyTenants => f.write_str("server busy"),
            Self::TooManyConnections => {
                f.write_str("too many concurrent connections for this account")
            }
        }
    }
}

impl std::error::Error for TenantError {}

/// One account's live document authority.
pub struct Tenant {
    /// This account's editor, version counter and collaboration state — the
    /// exact structure the single-user daemon keeps, one per account.
    pub(crate) state: Mutex<WebCanvasState>,
    /// This account's SSE subscribers. Separate per tenant so a version bump
    /// is only ever broadcast to the account that caused it.
    pub(crate) hub: SseHub,
    /// Live leases. Non-zero means "do not evict".
    leases: AtomicUsize,
    /// Unix seconds of the last lease acquire or release.
    last_active_unix: AtomicU64,
}

impl Tenant {
    fn new(port: u16, allow_origins: &[String], now_unix: u64) -> Self {
        let mut state = WebCanvasState::new_for_tenant(EditorState::starter(), port);
        // Every tenant answers for the same public origin; the allowlist is a
        // deployment property, not an account one.
        state.allow_origins = allow_origins.to_vec();
        Self {
            state: Mutex::new(state),
            hub: SseHub::default(),
            leases: AtomicUsize::new(0),
            last_active_unix: AtomicU64::new(now_unix),
        }
    }

    /// Live lease count. Zero is the only evictable value.
    pub fn lease_count(&self) -> usize {
        self.leases.load(Ordering::Acquire)
    }

    fn touch(&self, now_unix: u64) {
        self.last_active_unix.store(now_unix, Ordering::Release);
    }

    fn idle_secs(&self, now_unix: u64) -> u64 {
        now_unix.saturating_sub(self.last_active_unix.load(Ordering::Acquire))
    }
}

/// A connection's claim on a tenant.
///
/// Held for the whole request (including a long-lived SSE stream). While one
/// exists the tenant cannot be evicted, so the borrows a connection takes out
/// of it stay valid.
pub struct TenantLease {
    tenant: Arc<Tenant>,
}

impl std::fmt::Debug for TenantLease {
    /// Deliberately opaque: a lease points at an account's whole editor, and
    /// a derived `Debug` would print its document and its credentials into
    /// whatever log or test failure rendered it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TenantLease")
            .field("leases", &self.tenant.lease_count())
            .finish_non_exhaustive()
    }
}

impl TenantLease {
    pub fn tenant(&self) -> &Tenant {
        &self.tenant
    }

    pub fn state(&self) -> &Mutex<WebCanvasState> {
        &self.tenant.state
    }

    pub fn hub(&self) -> &SseHub {
        &self.tenant.hub
    }
}

impl Drop for TenantLease {
    fn drop(&mut self) {
        self.tenant.leases.fetch_sub(1, Ordering::AcqRel);
        // Idleness is measured from the last release, not the last acquire:
        // a tenant that just finished a 20-minute SSE stream has been active
        // the whole time.
        self.tenant.touch(now_unix());
    }
}

/// Every live tenant, keyed by verified account id.
pub struct TenantRegistry {
    tenants: Mutex<HashMap<String, Arc<Tenant>>>,
    limits: TenantLimits,
    /// Reported by `GET /api/mcp/server`; the same bound port for everyone.
    port: u16,
    /// Public origins this deployment answers for, stamped onto every tenant.
    allow_origins: Vec<String>,
}

impl TenantRegistry {
    pub fn new(port: u16, limits: TenantLimits, allow_origins: Vec<String>) -> Self {
        Self {
            tenants: Mutex::new(HashMap::new()),
            limits,
            port,
            allow_origins,
        }
    }

    /// The deployment's public origin allowlist.
    pub fn allow_origins(&self) -> &[String] {
        &self.allow_origins
    }

    pub const fn limits(&self) -> TenantLimits {
        self.limits
    }

    pub fn tenant_count(&self) -> usize {
        self.lock().len()
    }

    /// Take a lease on `identity`'s tenant, creating it if this is the
    /// account's first live connection.
    ///
    /// The key is `identity.user_id` and nothing else — see the module docs
    /// on [`super::tenant_auth`] for why no request-supplied value may ever
    /// reach this argument.
    ///
    /// A brand new tenant starts from [`EditorState::starter`]. M1 does not
    /// persist, so an evicted tenant's document is gone and a returning
    /// account gets a fresh starter document; that is the documented M1
    /// semantic, and M4 replaces it with load-on-create.
    pub fn lease_for(&self, identity: &ResolvedIdentity) -> Result<TenantLease, TenantError> {
        let now = now_unix();
        let mut tenants = self.lock();
        let tenant = match tenants.get(&identity.user_id) {
            Some(existing) => Arc::clone(existing),
            None => {
                if tenants.len() >= self.limits.max_tenants {
                    return Err(TenantError::TooManyTenants);
                }
                let created = Arc::new(Tenant::new(self.port, &self.allow_origins, now));
                tenants.insert(identity.user_id.clone(), Arc::clone(&created));
                created
            }
        };
        if tenant.lease_count() >= self.limits.max_conns_per_tenant {
            return Err(TenantError::TooManyConnections);
        }
        // Both this increment and `evict_idle`'s zero-check run under the
        // registry lock, so a tenant can never be evicted between being
        // resolved above and being leased here.
        tenant.leases.fetch_add(1, Ordering::AcqRel);
        tenant.touch(now);
        Ok(TenantLease { tenant })
    }

    /// Reclaim tenants that hold no lease and have been idle past the limit.
    ///
    /// Returns how many were reclaimed. The removal is a compare-and-remove
    /// against the exact `Arc` that was chosen (see the module docs): M4
    /// writes the document to disk between the choice and the removal, and
    /// that write must not race a concurrent `lease_for` re-creating the
    /// same account.
    pub fn evict_idle(&self, now_unix: u64) -> usize {
        let mut tenants = self.lock();
        let victims: Vec<(String, Arc<Tenant>)> = tenants
            .iter()
            .filter(|(_, tenant)| {
                tenant.lease_count() == 0
                    && tenant.idle_secs(now_unix) >= self.limits.idle_evict_secs
            })
            .map(|(id, tenant)| (id.clone(), Arc::clone(tenant)))
            .collect();
        let mut evicted = 0;
        for (id, victim) in victims {
            // M4 persistence hook: write `victim.state` to
            // `$OPENPENCIL_ONLINE_DATA_DIR/<id>/current.op` HERE, before the
            // compare-and-remove below, so a returning account either finds
            // the old tenant still mapped or a file it can load.
            let still_the_same = tenants
                .get(&id)
                .is_some_and(|current| Arc::ptr_eq(current, &victim));
            if still_the_same && victim.lease_count() == 0 {
                tenants.remove(&id);
                evicted += 1;
            }
        }
        evicted
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<Tenant>>> {
        self.tenants.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// Seconds since the Unix epoch, saturating to 0 before it.
pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "tenant_tests.rs"]
mod tests;
