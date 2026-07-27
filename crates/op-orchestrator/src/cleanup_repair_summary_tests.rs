//! End-to-end coverage for the quality tally the cleanup driver produces —
//! the counting layer added on top of `run_cleanup_passes`, NOT the passes
//! themselves. Two invariants matter here and nowhere else:
//!
//! 1. Reaching a checkpoint records the category as CHECKED, so a document
//!    with nothing wrong still yields a positive credential.
//! 2. A repair the passes actually make shows up as a non-zero count under
//!    the category that made it — the number is measured, not asserted.

use crate::cleanup::run_cleanup_passes_with_summary;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::repair_summary::{CheckCategory, RepairSummary};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::json;

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
            width: 1200.0,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn run_with_summary(tree: serde_json::Value) -> (VecDocSink, RepairSummary) {
    let mut sink = VecDocSink::new();
    let node: PenNode = serde_json::from_value(tree).expect("fixture json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary(&mut sink, &plan(), &[&root_id], &mut summary);
    (sink, summary)
}

#[test]
fn a_clean_document_still_reports_every_category_as_checked() {
    let (_sink, summary) = run_with_summary(json!({
        "type": "frame",
        "id": "root",
        "name": "Simple Page",
        "width": 1200,
        "height": 400,
        "layout": "vertical",
        "children": [{
            "type": "text",
            "id": "title",
            "role": "heading",
            "content": "Hello",
            "fontSize": 32,
            "fontWeight": 700
        }]
    }));

    assert!(
        !summary.is_empty(),
        "the passes ran, so the credential must not be suppressed"
    );
    assert_eq!(
        summary.checked(),
        CheckCategory::ALL.to_vec(),
        "every checkpoint in the driver is reached on any non-empty root"
    );
}

#[test]
fn no_root_ids_leaves_nothing_vouched_for_per_root() {
    // Whole-document passes still run (and still checkpoint), but the
    // per-root loop is skipped entirely. What matters is that the summary is
    // never fabricated: it reports only categories whose checkpoints ran.
    let mut sink = VecDocSink::new();
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary(&mut sink, &plan(), &[], &mut summary);

    for category in summary.checked() {
        assert_eq!(
            summary.repairs_for(category),
            0,
            "an empty document cannot have repaired anything"
        );
    }
}

#[test]
fn an_over_bold_screen_counts_its_repairs_under_hierarchy() {
    // Same fixture as `cleanup_tests::run_cleanup_passes_repairs_overbold_
    // text_hierarchy`, which proves the pass fires; here we assert the tally
    // sees it, and attributes it to the right category.
    let (sink, summary) = run_with_summary(json!({
        "type": "frame",
        "id": "root",
        "name": "Mobile Root",
        "width": 390,
        "height": 844,
        "children": [
            { "type": "text", "id": "title", "role": "heading",
              "content": "Popular Restaurants", "width": 320, "height": 40,
              "fontSize": 30, "fontWeight": 800 },
            { "type": "text", "id": "subtitle", "role": "body-text",
              "content": "Fresh Brooklyn favorites, delivered fast.",
              "width": 320, "height": 22, "fontSize": 16, "fontWeight": 800 },
            { "type": "text", "id": "placeholder", "name": "Placeholder",
              "content": "Search restaurants or dishes", "width": 280,
              "height": 24, "fontSize": 17, "fontWeight": 800 },
            { "type": "text", "id": "metadata", "role": "caption",
              "content": "20-30 min", "width": 100, "height": 18,
              "fontSize": 14, "fontWeight": 800 }
        ]
    }));

    assert!(
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::SetNodeFontWeight {
                font_weight: 400,
                ..
            }
        )),
        "precondition: the hierarchy pass must actually fire on this fixture"
    );
    assert!(
        summary.repairs_for(CheckCategory::Hierarchy) > 0,
        "the demotions the pass applied must be counted under hierarchy: {summary:?}"
    );
    assert!(
        summary.total_repairs() >= summary.repairs_for(CheckCategory::Hierarchy),
        "the headline total can never be smaller than one of its parts"
    );
}

#[test]
fn the_tally_never_exceeds_the_edits_the_sink_actually_took() {
    // The credential's number must be defensible against the document: it
    // counts accepted applies, so it can never claim more repairs than the
    // sink recorded.
    let (sink, summary) = run_with_summary(json!({
        "type": "frame",
        "id": "root",
        "name": "Mobile Root",
        "width": 390,
        "height": 844,
        "children": [
            { "type": "text", "id": "a", "role": "body-text", "content": "One",
              "width": 320, "height": 22, "fontSize": 16, "fontWeight": 800 },
            { "type": "text", "id": "b", "role": "caption", "content": "Two",
              "width": 100, "height": 18, "fontSize": 14, "fontWeight": 800 }
        ]
    }));

    assert!(
        summary.total_repairs() <= sink.applied.len(),
        "counted {} repairs but the sink only took {} edits",
        summary.total_repairs(),
        sink.applied.len()
    );
}
