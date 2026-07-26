//! multiscreen-fanout-break fix — end-to-end `Orchestrator::run()` tests for
//! the per-screen-group N-root scaffold (item A) and the inter-group
//! concurrent executor (item D-lite).
//!
//! Wired as `#[path = "run_tests_screen_groups.rs"] mod tests_screen_groups;`
//! inside `run.rs`; stays a child module of `run`, so `use super::*`
//! resolves to `run`.

use super::*;
use crate::test_support::{
    CountingLlm, DelayedLlm, ScriptResponse, ScriptedLlm, SkippedPreValidator,
    SkippedScreenshotProvider, SkippedVisionLlmClient, VecDocSink,
};
use crate::types::LlmError;
use jian_ops_schema::node::{PenNode, TextContent};

fn stub_providers() -> ValidationProviders<'static> {
    ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    }
}

fn req() -> DesignRequest {
    DesignRequest {
        prompt: "continue generating the remaining screens".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: false,

        visual_ref_enabled: false,
    }
}

fn req_with_concurrency(concurrency: u32) -> DesignRequest {
    DesignRequest {
        concurrency,
        ..req()
    }
}

fn progress_event(progress: &Progress) -> &Progress {
    match progress {
        Progress::WorkerScoped(worker) => progress_event(worker.event.as_ref()),
        event => event,
    }
}

const MULTI_SCREEN_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "App", "width": 390, "height": 844,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "home-hero", "label": "Home Hero", "screen": "Home",
      "region": { "width": 390, "height": 300 } },
    { "id": "profile-hero", "label": "Profile Hero", "screen": "Profile",
      "region": { "width": 390, "height": 300 } }
  ]
}"##;

// Script-gen (the default protocol): `I(parent, obj)` calls, not raw
// `_parent` JSONL. Ids are dropped — batch_design reassigns fresh ones
// anyway; the "content" string is what survives verbatim and identifies
// which screen's subtask landed where.
fn node_json(marker: &str) -> String {
    format!(
        r#"I(null, {{"type":"frame","name":"Sec","x":0,"y":0,"width":390,"height":300,"children":[{{"type":"text","content":"{marker}","fontSize":18}}]}});"#
    )
}

fn contains_text(node: &PenNode, marker: &str) -> bool {
    if let PenNode::Text(t) = node {
        if let TextContent::Plain(s) = &t.content {
            if s == marker {
                return true;
            }
        }
    }
    node.children()
        .into_iter()
        .flatten()
        .any(|c| contains_text(c, marker))
}

fn screen_marker(node: &PenNode) -> Option<&str> {
    match node {
        PenNode::Frame(f) => f.screen.as_deref(),
        _ => None,
    }
}

/// Core multi-root regression: a plan whose subtasks span 2 distinct
/// `screen` labels must produce 2 SEPARATE top-level roots (not one flat
/// frame), correctly named, positioned side-by-side, and with each
/// subtask's content routed into its OWN screen's root — the exact
/// "continue generating the remaining screens" bug this fix addresses.
#[test]
fn multi_screen_plan_gets_one_root_per_screen_group() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(MULTI_SCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("home")),
        ScriptResponse::Text(node_json("profile")),
    ]);
    let mut sink = VecDocSink::new();

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("multi-screen run ok");

    let roots = sink.state.active_children();
    assert_eq!(roots.len(), 2, "one root per screen group, got {roots:?}");

    assert_eq!(roots[0].base().name.as_deref(), Some("Home"));
    assert_eq!(roots[1].base().name.as_deref(), Some("Profile"));

    let x0 = roots[0].base().x.unwrap_or(0.0);
    let x1 = roots[1].base().x.unwrap_or(0.0);
    assert!(
        x1 >= x0 + 390.0 + 80.0 - 0.01,
        "second root must be laid out to the right of the first: x0={x0} x1={x1}"
    );
    assert_eq!(
        roots[0].base().y,
        roots[1].base().y,
        "sibling screens share the same top edge"
    );

    assert!(
        contains_text(&roots[0], "home"),
        "home content lands in the Home root"
    );
    assert!(
        !contains_text(&roots[0], "profile"),
        "profile content must not leak into the Home root"
    );
    assert!(
        contains_text(&roots[1], "profile"),
        "profile content lands in the Profile root"
    );
    assert!(
        !contains_text(&roots[1], "home"),
        "home content must not leak into the Profile root"
    );

    // First surviving root is the "primary" summary root, mirroring the
    // deleted concurrent path's own convention.
    assert_eq!(summary.root_frame_id, roots[0].id_str());
    assert_eq!(summary.subtasks.len(), 2);
}

/// Co-op point with Track A (`wire_screen_navigation`, run as part of
/// `finalize_design`'s cleanup tail): once N screen-shaped roots exist,
/// EVERY one of them — not just the first — gets a `screen` route marker,
/// proving the whole set of real (post-remap) root ids reached cleanup, not
/// just the first / primary one.
#[test]
fn multi_screen_plan_roots_all_get_wired_for_app_mode_preview() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(MULTI_SCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("home")),
        ScriptResponse::Text(node_json("profile")),
    ]);
    let mut sink = VecDocSink::new();

    futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("multi-screen run ok");

    let roots = sink.state.active_children();
    assert_eq!(roots.len(), 2);
    for root in roots {
        assert!(
            screen_marker(root).is_some(),
            "every screen-group root must come out of cleanup App-Mode-ready: {root:?}"
        );
    }
}

// ── Item D-lite: inter-group concurrency ────────────────────────────────

/// Three screens, matching the user-reported repro shape ("continue
/// generating Search/Library/Premium") — one subtask each, so each group's
/// worker runs exactly one LLM call.
const THREE_SCREEN_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "App", "width": 390, "height": 844,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "search-body", "label": "Search Body", "screen": "Search",
      "region": { "width": 390, "height": 300 } },
    { "id": "library-body", "label": "Library Body", "screen": "Library",
      "region": { "width": 390, "height": 300 } },
    { "id": "premium-body", "label": "Premium Body", "screen": "Premium",
      "region": { "width": 390, "height": 300 } }
  ]
}"##;

/// Matches on the base name OR that name with the promise-delivery
/// invariant's " (unfilled)" suffix (`unfilled_screens::mark_unfilled_screens`)
/// — a subtask whose group never delivered content is exactly the case that
/// suffix exists to flag, so a root under test here may legitimately carry
/// it by the time `run` returns.
fn find_root_by_name<'a>(roots: &'a [PenNode], name: &str) -> &'a PenNode {
    roots
        .iter()
        .find(|r| {
            r.base()
                .name
                .as_deref()
                .is_some_and(|n| n == name || n == format!("{name} (unfilled)"))
        })
        .unwrap_or_else(|| panic!("no root named {name} among {roots:?}"))
}

// Cluster test modules — this file keeps the shared fixtures.
#[path = "run_tests_screen_concurrency.rs"]
mod concurrency_tests;
#[path = "run_tests_screen_replay.rs"]
mod replay_tests;
