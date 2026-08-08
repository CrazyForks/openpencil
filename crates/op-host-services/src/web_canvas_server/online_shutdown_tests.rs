//! The shutdown write barrier and its start-up probes.
//!
//! Split out of `online_share_tests.rs` at the 800-line cap; nested under it
//! so `use super::*` still reaches the request builder and helpers.

use super::*;

// ---------------------------------------------------------------------------
// B3: the shutdown write barrier.
// ---------------------------------------------------------------------------

#[test]
fn the_write_barrier_admits_writes_until_shutdown_closes_it() {
    use crate::web_canvas_server::tenant::WriteBarrier;
    let barrier = WriteBarrier::default();
    assert!(!barrier.is_closed());

    let pass = barrier.enter().expect("open");
    assert_eq!(barrier.active(), 1);
    drop(pass);
    assert_eq!(barrier.active(), 0);

    barrier.close();
    assert!(barrier.is_closed());
    assert!(
        barrier.enter().is_none(),
        "a stopping daemon must refuse writes rather than ack one it will not persist"
    );
}

#[test]
fn a_held_pass_keeps_the_barrier_busy_so_the_flush_waits() {
    // The window this closes: a worker past the connection drain, about to
    // take the state lock, commits after the flush snapshotted the document —
    // having already answered 200.
    use crate::web_canvas_server::tenant::WriteBarrier;
    let barrier = WriteBarrier::default();
    let held = barrier.enter().expect("open");
    barrier.close();
    assert_eq!(
        barrier.active(),
        1,
        "closing must not abandon a write already inside"
    );
    drop(held);
    assert_eq!(barrier.active(), 0, "the drain can now proceed");
}

#[test]
fn closing_between_the_check_and_the_increment_does_not_admit_a_writer() {
    // `enter` re-checks after incrementing precisely so a close landing in
    // that window cannot admit a writer the drain has stopped waiting for.
    use crate::web_canvas_server::tenant::WriteBarrier;
    let barrier = std::sync::Arc::new(WriteBarrier::default());
    barrier.close();
    for _ in 0..100 {
        assert!(barrier.enter().is_none());
    }
    assert_eq!(barrier.active(), 0, "a refused entry must not leak a count");
}

#[test]
fn a_write_during_shutdown_is_refused_rather_than_acked() {
    use crate::web_canvas_server::tenant::WriteBarrier;
    let registry = registry();
    let verifier = verifier();
    let barrier = WriteBarrier::default();
    barrier.close();

    let request = Request::json("POST", "/api/mcp/document", SYNC_BODY).with_bearer("tokA");
    let mut stream = MockStream {
        input: std::io::Cursor::new(request.wire().into_bytes()),
        output: Vec::new(),
    };
    serve_one_online(&mut stream, &registry, &verifier, &barrier).expect("serve");
    let response = String::from_utf8_lossy(&stream.output).into_owned();
    assert_eq!(
        status_line(&response),
        "HTTP/1.1 503 Service Unavailable",
        "{response}"
    );
    assert_eq!(body(&response)["error"], "shutting-down");
}
