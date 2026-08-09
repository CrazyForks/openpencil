//! Design-diagnostics detectors, ported from the TS `pen-ai-skills`
//! diagnostics layer. Each detector is a pure recursive walk over the jian
//! `PenNode` tree returning `Vec<Issue>`.
//!
//! `detect_all` runs all 14 detectors in the TS `detectAllIssues` order and
//! returns the deduplicated combined issue list.

use std::collections::HashSet;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;

use crate::design_form::{classify_root_form_node, DesignForm};
use crate::issue::Issue;

pub mod siblings;
pub mod spacing;
pub mod structure;
pub mod text;
pub mod typography;
pub mod widget_a11y;

#[cfg(test)]
mod siblings_tests;
#[cfg(test)]
mod spacing_edge_tests;

pub use siblings::*;
pub use spacing::*;
pub use structure::*;
pub use text::*;
pub use typography::*;
pub use widget_a11y::*;

/// Port of `detectAllIssues` (`detectors.ts:698-724`). Runs the 14 detectors
/// in the exact TS call order, concatenates their issue lists, and dedups on
/// `{node_id}:{property}` — the first occurrence (in detector order, then
/// tree-walk order) wins.
///
/// The dedup key reuses [`crate::issue::FixProperty::wire_str`] so the
/// `{node_id}:{property}` string is byte-identical to the TS
/// `` `${issue.nodeId}:${issue.property}` `` key.
pub fn detect_all(root: &PenNode, doc: &PenDocument) -> Vec<Issue> {
    detect_all_for_form(root, doc, classify_root_form_node(root))
}

/// [`detect_all`] told what kind of surface `root` is, for callers that
/// already know — the orchestrator holds the planned artboard, and on an
/// append path the node handed here is not always the artboard itself.
///
/// A deck board takes different geometry and typography floors from a
/// scrolling page (deck-system spec §4.1): a projector is read from the back
/// row, so its font floor and margins are absolute rather than relative to a
/// brand's rhythm. **No detector branches on the form yet** — this is the
/// wiring the deck detectors (spec §4.2) land on, and the point of putting it
/// in first is that each of them receives the form from the single classifier
/// instead of re-deriving it from a width comparison of its own.
pub fn detect_all_for_form(root: &PenNode, doc: &PenDocument, form: DesignForm) -> Vec<Issue> {
    let _ = form;
    let mut combined = Vec::new();
    combined.extend(detect_invisible_containers(root, doc));
    combined.extend(detect_empty_paths(root));
    combined.extend(detect_text_explicit_heights(root));
    combined.extend(detect_sibling_inconsistencies(root));
    combined.extend(detect_unexpected_rotation(root));
    combined.extend(detect_text_corner_radius(root));
    combined.extend(detect_mixed_sibling_corner_radius(root));
    combined.extend(detect_text_effect(root));
    combined.extend(detect_text_stroke(root));
    combined.extend(detect_mixed_sibling_padding(root));
    combined.extend(detect_excessive_frame_effects(root));
    combined.extend(detect_edge_section_padding(root));
    combined.extend(detect_stacked_horizontal_padding(root));
    combined.extend(detect_text_bg_contrast(root, doc));
    // Phase E5 — widget a11y. No TS counterpart; runs last so it never
    // shadows an earlier detector under the `{node_id}:{property}` dedup.
    combined.extend(detect_unlabeled_inputs(root));

    let mut seen = HashSet::new();
    combined
        .into_iter()
        .filter(|issue| seen.insert(format!("{}:{}", issue.node_id, issue.property.wire_str())))
        .collect()
}

#[cfg(test)]
mod detect_all_tests {
    use super::*;
    use crate::issue::{FixProperty, IssueCategory, IssueSeverity};
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    fn doc(value: serde_json::Value) -> PenDocument {
        serde_json::from_value(value).expect("fixture must deserialize as PenDocument")
    }

    /// A clean document with no detectable issues → empty list.
    #[test]
    fn detect_all_on_clean_doc_returns_empty() {
        let root = node(json!({
            "type": "frame", "id": "root", "layout": "vertical",
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "text", "id": "t1", "content": "Hello",
                    "fill": [{"type": "solid", "color": "#000000"}]
                }
            ]
        }));
        assert!(detect_all(&root, &doc(json!({"version": "1.0", "children": []}))).is_empty());
    }

    /// A `cornerRadius` outlier among same-role frames is caught by BOTH
    /// `detect_sibling_inconsistencies` (4th, `warning`) and
    /// `detect_mixed_sibling_corner_radius` (7th, `warning`) on the same
    /// `(node_id, "cornerRadius")` key. `detect_all` dedups to the
    /// sibling-inconsistency issue — it runs earlier in TS order.
    #[test]
    fn detect_all_dedups_collision_keeping_earlier_detector() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "a", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "b", "role": "card", "cornerRadius": 8},
                {"type": "frame", "id": "c", "role": "card", "cornerRadius": 16}
            ]
        }));
        let issues = detect_all(&root, &doc(json!({"version": "1.0", "children": []})));
        // Exactly one issue for node `c` — the earlier sibling-inconsistency
        // detector wins; the mixed-corner-radius duplicate is dropped.
        assert_eq!(
            issues
                .iter()
                .filter(|i| i.node_id == "c" && i.property == FixProperty::CornerRadius)
                .count(),
            1
        );
        let c_issue = issues
            .iter()
            .find(|i| i.node_id == "c")
            .expect("node c flagged");
        assert_eq!(c_issue.category, IssueCategory::SiblingInconsistency);
        assert_eq!(c_issue.severity, IssueSeverity::Warning);
    }

    /// `detect_all` classifies the root itself so every existing caller
    /// becomes form-aware the moment a detector starts branching — and, until
    /// one does, a deck board lints exactly like anything else.
    #[test]
    fn detect_all_classifies_the_root_and_reports_the_same_issues_for_now() {
        let board = node(json!({
            "type": "frame", "id": "board", "width": 1920, "height": 1080,
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {"type": "frame", "id": "tilted", "rotation": 12},
                {"type": "path", "id": "empty"}
            ]
        }));
        let document = doc(json!({"version": "1.0", "children": []}));
        assert_eq!(
            crate::design_form::classify_root_form_node(&board),
            crate::design_form::DesignForm::Deck
        );
        assert_eq!(
            detect_all(&board, &document),
            detect_all_for_form(&board, &document, DesignForm::Page),
            "no detector branches on the form yet — the deck floors land later"
        );
    }

    /// `detect_all` runs the 14 detectors in TS order — assert the combined
    /// list keeps that order across detectors. An empty path (detector 2)
    /// must precede an unexpected rotation (detector 5) on a doc carrying
    /// both, regardless of tree position.
    #[test]
    fn detect_all_preserves_ts_detector_order() {
        // The rotated frame is the FIRST child; the empty path is the SECOND.
        // Tree-walk order would emit rotation before empty-path, but
        // `detect_all`'s detector order (empty_paths is 2nd, rotation is 5th)
        // must put the empty-path issue first.
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "tilted", "rotation": 12},
                {"type": "path", "id": "empty"}
            ]
        }));
        let issues = detect_all(&root, &doc(json!({"version": "1.0", "children": []})));
        let categories: Vec<IssueCategory> = issues.iter().map(|i| i.category).collect();
        assert_eq!(
            categories,
            vec![IssueCategory::EmptyPath, IssueCategory::UnexpectedRotation]
        );
    }
}
