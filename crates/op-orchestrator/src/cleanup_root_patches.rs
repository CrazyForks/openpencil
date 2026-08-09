//! Root-level property patches the cleanup driver applies in place.
//!
//! Carved off `cleanup.rs` (spine + siblings, 800-line cap) as pure code
//! motion. What holds these three together is the SHAPE of the edit, not the
//! defect they fix: each reads one root, decides one property, and writes it
//! with a `PatchNodeData` — deliberately NOT an `apply_root_transform`, which
//! rebuilds the subtree and hands the root a fresh id. Anything holding the
//! root id (the caller's `root_ids`, the loop's screen bookkeeping) would be
//! left pointing at a node that no longer exists, so a pass that only sets a
//! number or a keyword must not use one.

use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

use crate::types::DocSink;

/// The geometric half of the deck judgement, per root.
///
/// [`crate::cleanup::CleanupPolicy::is_deck`] answers "the REQUEST asked for a
/// deck", which is prompt keywords and therefore blind three ways: the agentic
/// loop has no prompt at all (its policy is `Default`, so `is_deck` was
/// permanently false), a scene template that reaches cleanup carries no
/// request, and a user who says "1920×1080 主视觉" without ever typing PPT
/// still drew a board. Unioned in at the point of use rather than baked into
/// the policy so the answer is taken PER ROOT — one document can hold a board
/// and a page, and a single run-wide flag cannot describe that.
pub(super) fn root_is_deck_board(sink: &dyn DocSink, root_id: &str) -> bool {
    crate::geometry_validation::root_design_form(sink.state(), root_id).is_deck_board()
}

/// Centre a deck board's content on its fixed 16:9 surface.
///
/// A slide root is 1080 tall no matter how much content it holds, and its
/// sections hug their own height, so without this they stack from the top and
/// leave the lower half blank — measured on every generated deck.
///
/// Only `justifyContent` is written. Stretching the sections to fill instead
/// would distort whatever the model composed; centring changes where the block
/// sits, not what it is.
pub(super) fn centre_deck_board_content(sink: &mut dyn DocSink, root_id: &str) {
    let Some(root) = sink
        .state()
        .active_children()
        .iter()
        .find(|node| node.id_str() == root_id)
    else {
        return;
    };
    let value = serde_json::to_value(root).unwrap_or(serde_json::Value::Null);
    // Respect an explicit distribution: a board that deliberately pushes
    // content apart (space_between) or pins it low is a composition, not the
    // default top-stack this repairs.
    if value
        .get("justifyContent")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|mode| !mode.is_empty())
    {
        return;
    }
    let node_id = NodeId::new(root.id_str());
    sink.apply(EditorCommand::PatchNodeData {
        node_id,
        patch_json: r#"{"justifyContent":"center"}"#.to_string(),
        page_id: None,
    });
}

/// Write the repaired root gap as a property patch.
///
/// Deliberately NOT an `apply_root_transform`: that rebuilds the subtree and
/// hands the root a fresh id, which is the right shape for passes that
/// restructure but wrong for setting one number — anything holding the root id
/// (the caller's `root_ids`, the loop's screen bookkeeping) would be left
/// pointing at a node that no longer exists.
pub(super) fn patch_root_section_gap(sink: &mut dyn DocSink, root_id: &str) {
    let Some(root) = sink
        .state()
        .active_children()
        .iter()
        .find(|node| node.id_str() == root_id)
    else {
        return;
    };
    let node_id = NodeId::new(root.id_str());
    let Ok(mut value) = serde_json::to_value(root) else {
        return;
    };
    if !crate::root_section_gap::fix_root_section_gap(&mut value) {
        return;
    }
    let Some(gap) = value.get("gap") else {
        return;
    };
    sink.apply(EditorCommand::PatchNodeData {
        node_id,
        patch_json: format!(r#"{{"gap":{gap}}}"#),
        page_id: None,
    });
}
