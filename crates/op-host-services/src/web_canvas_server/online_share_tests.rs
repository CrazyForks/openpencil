//! Sharing, the tenant query parameter, and eviction persistence, exercised
//! end to end through the online accept loop.
//!
//! Split out of `online_run_loop_tests.rs` at the 800-line cap; nested under
//! it so `use super::*` still reaches the request builder and helpers.

use super::*;
use crate::web_canvas_server::tenant_store::TenantStore;

/// A registry with a real on-disk store rooted in a temp directory.
struct PersistentRegistry {
    root: std::path::PathBuf,
    registry: TenantRegistry,
}

impl PersistentRegistry {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "op-online-share-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp root");
        Self {
            registry: TenantRegistry::with_store(
                3102,
                TenantLimits {
                    idle_evict_secs: 1,
                    ..TenantLimits::default()
                },
                vec![PUBLIC_ORIGIN.to_string()],
                TenantStore::new(Some(root.clone())),
            ),
            root,
        }
    }
}

impl Drop for PersistentRegistry {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Address a request at another account's tenant.
fn as_tenant(mut request: Request, owner: &'static str) -> Request {
    request.tenant = Some(owner);
    request
}

fn share(
    registry: &TenantRegistry,
    token: &'static str,
    route: &'static str,
    target: &str,
) -> String {
    serve(
        registry,
        &verifier(),
        Request::json(
            "POST",
            route,
            &serde_json::json!({ "userId": target }).to_string(),
        )
        .with_bearer(token),
    )
}

#[test]
fn a_visitor_reads_and_writes_the_owner_document_only_after_a_grant() {
    let registry = registry();
    let verifier = verifier();

    // Before the grant, addressing userA's tenant is refused.
    let refused = serve(
        &registry,
        &verifier,
        as_tenant(
            Request::new("GET", "/api/mcp/document").with_bearer("tokB"),
            "userA",
        ),
    );
    assert_eq!(status_line(&refused), "HTTP/1.1 403 Forbidden", "{refused}");
    assert_eq!(body(&refused)["error"], "tenant-not-shared");

    // userA shares.
    let granted = share(
        &registry,
        "tokA",
        op_editor_core::share_routes::GRANT,
        "userB",
    );
    assert_eq!(status_line(&granted), "HTTP/1.1 200 OK", "{granted}");

    // Now userB writes into userA's document…
    let pushed = serve(
        &registry,
        &verifier,
        as_tenant(
            Request::json("POST", "/api/mcp/document", SYNC_BODY).with_bearer("tokB"),
            "userA",
        ),
    );
    assert_eq!(status_line(&pushed), "HTTP/1.1 200 OK", "{pushed}");

    // …and userA sees it in their own tenant, with no parameter at all.
    let owner_view = serve(
        &registry,
        &verifier,
        Request::new("GET", "/api/mcp/document").with_bearer("tokA"),
    );
    assert!(owner_view.contains("Tenant Rect"), "{owner_view}");
    assert_eq!(body(&owner_view)["version"], 1);

    // userB's OWN document is untouched — the parameter addressed a tenant,
    // it did not move the visitor into it.
    let visitor_own = serve(
        &registry,
        &verifier,
        Request::new("GET", "/api/mcp/document").with_bearer("tokB"),
    );
    assert_eq!(body(&visitor_own)["version"], 0, "{visitor_own}");
}

#[test]
fn a_revoke_locks_the_visitor_out_again() {
    let registry = registry();
    share(
        &registry,
        "tokA",
        op_editor_core::share_routes::GRANT,
        "userB",
    );
    let allowed = serve(
        &registry,
        &verifier(),
        as_tenant(
            Request::new("GET", "/api/mcp/version").with_bearer("tokB"),
            "userA",
        ),
    );
    assert_eq!(status_line(&allowed), "HTTP/1.1 200 OK");

    share(
        &registry,
        "tokA",
        op_editor_core::share_routes::REVOKE,
        "userB",
    );
    let refused = serve(
        &registry,
        &verifier(),
        as_tenant(
            Request::new("GET", "/api/mcp/version").with_bearer("tokB"),
            "userA",
        ),
    );
    assert_eq!(status_line(&refused), "HTTP/1.1 403 Forbidden", "{refused}");
}

#[test]
fn a_tenant_parameter_naming_an_unshared_account_is_refused() {
    let registry = registry();
    for owner in ["userA", "nobody-at-all"] {
        let response = serve(
            &registry,
            &verifier(),
            Request {
                tenant: Some(Box::leak(owner.to_string().into_boxed_str())),
                ..Request::new("GET", "/api/mcp/document")
            }
            .with_bearer("tokB"),
        );
        assert_eq!(status_line(&response), "HTTP/1.1 403 Forbidden", "{owner}");
    }
}

#[test]
fn an_account_may_always_address_its_own_tenant_explicitly() {
    let registry = registry();
    let response = serve(
        &registry,
        &verifier(),
        as_tenant(
            Request::new("GET", "/api/mcp/version").with_bearer("tokA"),
            "userA",
        ),
    );
    assert_eq!(status_line(&response), "HTTP/1.1 200 OK", "{response}");
}

#[test]
fn the_share_list_reports_both_directions_over_the_wire() {
    let registry = registry();
    share(
        &registry,
        "tokA",
        op_editor_core::share_routes::GRANT,
        "userB",
    );

    let owner = serve(
        &registry,
        &verifier(),
        Request::new("GET", op_editor_core::share_routes::LIST).with_bearer("tokA"),
    );
    assert_eq!(body(&owner)["sharedWith"][0], "userB", "{owner}");

    let visitor = serve(
        &registry,
        &verifier(),
        Request::new("GET", op_editor_core::share_routes::LIST).with_bearer("tokB"),
    );
    assert_eq!(body(&visitor)["sharedWithMe"][0], "userA", "{visitor}");
}

#[test]
fn a_share_route_always_administers_the_callers_own_tenant() {
    // Even with a `?tenant=` parameter pointing at the owner, a visitor's
    // grant lands on the visitor's own access list.
    let registry = registry();
    share(
        &registry,
        "tokA",
        op_editor_core::share_routes::GRANT,
        "userB",
    );
    let response = serve(
        &registry,
        &verifier(),
        as_tenant(
            Request::json(
                "POST",
                op_editor_core::share_routes::GRANT,
                r#"{"userId":"userC"}"#,
            )
            .with_bearer("tokB"),
            "userA",
        ),
    );
    assert_eq!(status_line(&response), "HTTP/1.1 200 OK", "{response}");

    // userC still cannot reach userA.
    let stranger = serve(
        &registry,
        &verifier(),
        as_tenant(
            Request::new("GET", "/api/mcp/version").with_bearer("tokC"),
            "userA",
        ),
    );
    assert_eq!(
        status_line(&stranger),
        "HTTP/1.1 401 Unauthorized",
        "{stranger}"
    );
}

#[test]
fn an_evicted_tenant_is_written_and_read_back() {
    let temp = PersistentRegistry::new("roundtrip");
    let verifier = verifier();

    serve(
        &temp.registry,
        &verifier,
        Request::json("POST", "/api/mcp/document", SYNC_BODY).with_bearer("tokA"),
    );
    share(
        &temp.registry,
        "tokA",
        op_editor_core::share_routes::GRANT,
        "userB",
    );

    assert_eq!(temp.registry.evict_idle(now_unix() + 3600), 1);
    assert_eq!(temp.registry.tenant_count(), 0);
    assert!(temp.registry.store().has_document("userA"));

    // The document comes back…
    let restored = serve(
        &temp.registry,
        &verifier,
        Request::new("GET", "/api/mcp/document").with_bearer("tokA"),
    );
    assert!(restored.contains("Tenant Rect"), "{restored}");

    // …and so does the access list, so a share survives a reclaim.
    let visitor = serve(
        &temp.registry,
        &verifier,
        as_tenant(
            Request::new("GET", "/api/mcp/version").with_bearer("tokB"),
            "userA",
        ),
    );
    assert_eq!(status_line(&visitor), "HTTP/1.1 200 OK", "{visitor}");
}

#[test]
fn a_grant_is_persisted_immediately_rather_than_at_eviction() {
    // A share the user was told had succeeded must survive a restart, and the
    // document it applies to may not be written for another half hour.
    let temp = PersistentRegistry::new("acl-now");
    share(
        &temp.registry,
        "tokA",
        op_editor_core::share_routes::GRANT,
        "userB",
    );
    assert!(
        temp.registry.store().load_acl("userA").contains("userB"),
        "the grant must be on disk before any eviction"
    );
}

#[test]
fn a_corrupt_stored_document_yields_a_starter_and_keeps_the_bytes() {
    let temp = PersistentRegistry::new("corrupt");
    let verifier = verifier();
    serve(
        &temp.registry,
        &verifier,
        Request::json("POST", "/api/mcp/document", SYNC_BODY).with_bearer("tokA"),
    );
    assert_eq!(temp.registry.evict_idle(now_unix() + 3600), 1);

    let dir = temp.registry.store().tenant_dir("userA").expect("dir");
    std::fs::write(dir.join("current.op"), b"not a document").expect("corrupt");

    let served = serve(
        &temp.registry,
        &verifier,
        Request::new("GET", "/api/mcp/document").with_bearer("tokA"),
    );
    assert_eq!(status_line(&served), "HTTP/1.1 200 OK", "{served}");
    assert!(
        !served.contains("Tenant Rect"),
        "an unreadable document must yield a starter, not a failure: {served}"
    );
    let quarantined: Vec<String> = std::fs::read_dir(&dir)
        .expect("read dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("corrupt"))
        .collect();
    assert_eq!(
        quarantined.len(),
        1,
        "the bytes must be kept, not overwritten"
    );
}

#[test]
fn an_account_id_full_of_traversal_cannot_escape_the_data_directory() {
    let temp = PersistentRegistry::new("traversal");
    let hostile = "../../../../etc/op-escape";
    let verifier = StaticVerifier::parse(&format!("tokX={hostile}"));

    serve(
        &temp.registry,
        &verifier,
        Request::json("POST", "/api/mcp/document", SYNC_BODY).with_bearer("tokX"),
    );
    assert_eq!(temp.registry.evict_idle(now_unix() + 3600), 1);

    let dir = temp.registry.store().tenant_dir(hostile).expect("dir");
    assert!(
        dir.starts_with(&temp.root),
        "{dir:?} escaped {:?}",
        temp.root
    );
    assert!(
        dir.join("current.op").is_file(),
        "the document still round-trips"
    );
    // Nothing was created outside the store.
    assert!(!std::path::Path::new("/etc/op-escape").exists());
}

#[test]
fn a_tenant_that_cannot_be_written_stays_resident_rather_than_losing_its_document() {
    let temp = PersistentRegistry::new("unwritable");
    let verifier = verifier();
    serve(
        &temp.registry,
        &verifier,
        Request::json("POST", "/api/mcp/document", SYNC_BODY).with_bearer("tokA"),
    );
    // Make the store root a file so `create_dir_all` cannot succeed.
    std::fs::remove_dir_all(&temp.root).expect("clear root");
    std::fs::write(&temp.root, b"not a directory").expect("block the root");

    assert_eq!(
        temp.registry.evict_idle(now_unix() + 3600),
        0,
        "reclaiming memory must not be worth discarding a document"
    );
    assert_eq!(temp.registry.tenant_count(), 1);
    let still_there = serve(
        &temp.registry,
        &verifier,
        Request::new("GET", "/api/mcp/document").with_bearer("tokA"),
    );
    assert!(still_there.contains("Tenant Rect"), "{still_there}");

    let _ = std::fs::remove_file(&temp.root);
}
