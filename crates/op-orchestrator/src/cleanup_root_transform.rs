//! Root transform application and the decorative filled-stroke stripper.

use super::*;

/// Apply a whole-root transform (the serialize → mutate → deserialize round-trip
/// the structural passes use) to the page-root and commit it via `ReplaceSubtree`.
///
/// `ReplaceSubtree` allocates a FRESH id for the replaced node (see
/// `command_replace_tests`), so the root's id changes on every successful
/// transform. This returns the root's CURRENT id (re-resolved by its unchanged
/// position) so the caller threads it into the next pass — otherwise every
/// subsequent per-root cleanup pass would look up the stale id and no-op.
pub(super) fn apply_root_transform(
    sink: &mut dyn DocSink,
    root_id: &str,
    transform: fn(&mut PenNode) -> bool,
) -> String {
    let Some(idx) = sink
        .state()
        .active_children()
        .iter()
        .position(|n| n.id_str() == root_id)
    else {
        // A silent no-op here means EVERY cleanup pass silently skips this
        // root — surface it loudly so a stale-root bug can't hide again.
        tracing::warn!(root = %root_id, "cleanup: root id not found — pass skipped");
        return root_id.to_string();
    };
    let mut new_root = sink.state().active_children()[idx].clone();
    if !transform(&mut new_root) {
        return root_id.to_string();
    }
    sink.apply(EditorCommand::ReplaceSubtree {
        node_id: NodeId::new(root_id.to_string()),
        node: Box::new(new_root),
        drop_children: true,
        page_id: None,
    });
    sink.state()
        .active_children()
        .get(idx)
        .map(|n| n.id_str().to_string())
        .unwrap_or_else(|| root_id.to_string())
}

/// Strip the REDUNDANT border off a filled, shadowed container. When a
/// frame / group / rectangle has a fill AND a drop shadow AND a stroke,
/// the stroke is a "莫名其妙" hairline — the shadow already separates the
/// surface, so the border adds nothing on a light page. Clearing it
/// (`stroke_width = 0` → `stroke = None`) is conservative on purpose:
/// a filled container WITHOUT a shadow keeps its stroke (there the border
/// is the intentional boundary), and unfilled outlines (dividers) +
/// `text_input` borders are never touched.
pub(super) fn strip_decorative_filled_strokes(sink: &mut dyn DocSink, root_id: &str) {
    let targets: Vec<NodeId> = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return;
        };
        let mut ids = Vec::new();
        collect_redundant_borders(root, &mut ids);
        ids
    };
    for node_id in targets {
        sink.apply(EditorCommand::SetNodeStrokeWidth {
            node_id,
            width: 0.0,
        });
    }
}

pub(super) fn collect_redundant_borders(node: &PenNode, out: &mut Vec<NodeId>) {
    if has_redundant_shadowed_border(node) {
        out.push(NodeId::new(node.id_str().to_string()));
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_redundant_borders(child, out);
        }
    }
}

/// True when `node` is a Frame/Group/Rectangle carrying a non-empty fill,
/// a stroke, AND a drop shadow — the redundant-border case (the shadow,
/// not the stroke, separates the surface). A filled+stroked container
/// with NO shadow is left alone: there the border is intentional.
pub(super) fn has_redundant_shadowed_border(node: &PenNode) -> bool {
    let container = match node {
        PenNode::Frame(n) => &n.container,
        PenNode::Group(n) => &n.container,
        PenNode::Rectangle(n) => &n.container,
        _ => return false,
    };
    let has_fill = container.fill.as_ref().is_some_and(|f| !f.is_empty());
    let has_shadow = container
        .effects
        .as_ref()
        .is_some_and(|fx| fx.iter().any(|e| matches!(e, PenEffect::Shadow(_))));
    has_fill && container.stroke.is_some() && has_shadow
}
