//! Canvas press/drag state transitions shared by the native and web
//! widget hosts (their `canvas_select_drag.rs` / `node_drag.rs` twins
//! used to carry these as copy-pasted inline blocks).
//!
//! Everything here touches only `EditorState`; the hosts keep the
//! platform glue (`mark_dirty`, scene caches, layer-panel scrolling,
//! hover-probe caches, drag-state structs) in their thin wrappers.

use crate::selection_resolve::{resolve_canvas_depth_targets, CanvasDepthTargets};
use crate::state::EditorState;
use crate::NodeId;

/// Canvas double-click window, in milliseconds.
const CANVAS_DOUBLE_CLICK_MS: u64 = 400;

/// Everything a canvas press needs to route itself, resolved once from
/// the rendered root-to-deepest hit path.
pub struct CanvasPressResolve {
    /// Depth-resolved targets for the current entered-container scope.
    pub targets: CanvasDepthTargets,
    /// Node the press actually selects — normally `targets.primary`,
    /// but the deepest node when it is already the crop selection.
    pub primary: NodeId,
    /// Second press inside the double-click window over the same
    /// deepest geometry.
    pub is_double: bool,
    /// The deepest hit is the current single crop-editable selection, so
    /// this press must preserve it (a second press then activates crop
    /// editing) instead of drilling one level.
    pub selected_crop_is_deepest: bool,
}

/// Resolve a canvas press over `hit_path` and refresh the double-click
/// tracker. `None` when the path carries no usable target.
///
/// Shift and an existing multi-selection deliberately disable
/// drill-down so a set-edit gesture cannot unexpectedly enter a
/// container.
pub fn resolve_canvas_press(
    state: &mut EditorState,
    hit_path: &[NodeId],
    now_ms: u64,
    shift_held: bool,
) -> Option<CanvasPressResolve> {
    let deepest = hit_path.last()?.clone();
    let targets =
        resolve_canvas_depth_targets(hit_path, state.editor_ui.entered_container.as_ref())?;
    let is_double = matches!(
        &state.editor_ui.last_canvas_click,
        Some((prev, t)) if *prev == deepest
            && now_ms.saturating_sub(*t) < CANVAS_DOUBLE_CLICK_MS
    ) && !shift_held
        && state.selection_count() <= 1;
    state.editor_ui.last_canvas_click = if shift_held || is_double {
        None
    } else {
        Some((deepest.clone(), now_ms))
    };
    // A leaf selected directly from the Layer panel can sit below the
    // canvas depth resolver's primary target. Preserve that exact crop
    // selection on the first press so the second press can activate crop
    // editing. A child hit does not qualify: it must retain ordinary
    // one-level drill semantics.
    let selected_crop_is_deepest = deepest == state.selection.anchor
        && state.selection_count() == 1
        && state.can_edit_selected_image_crop();
    let primary = if selected_crop_is_deepest {
        deepest
    } else {
        targets.primary.clone()
    };
    Some(CanvasPressResolve {
        targets,
        primary,
        is_double,
        selected_crop_is_deepest,
    })
}

/// Double-click drill: select the direct child under the pointer and
/// enter the primary as the sibling scope. The stationary-pointer hover
/// is rebased immediately so the outline follows without a mouse move.
pub fn enter_child_scope(state: &mut EditorState, primary: NodeId, secondary: NodeId) {
    state.set_single_selection(secondary.clone());
    state.editor_ui.entered_container = Some(primary);
    state.editor_ui.canvas_hover_node = Some(secondary);
}

/// Apply the press selection at the resolved level and exit the entered
/// container when the press landed outside it.
///
/// A plain click selects the solid-outline primary, so clicking a
/// sibling inside an entered scope moves selection at that level
/// instead of dragging an arbitrary deepest leaf. Shift toggles set
/// membership.
///
/// Returns whether this press should begin a node drag — a shift press
/// that removed the node from the set must not drag it.
pub fn apply_canvas_press_selection(
    state: &mut EditorState,
    target: NodeId,
    shift_held: bool,
    hit_path: &[NodeId],
) -> bool {
    let should_start_drag = if shift_held {
        let was_in_set = state.is_selected(&target);
        state.toggle_selection(target);
        !was_in_set
    } else {
        let already_in_set = state.is_selected(&target);
        if !already_in_set || state.selection_count() == 1 {
            state.set_single_selection(target);
        }
        true
    };
    if state
        .editor_ui
        .entered_container
        .as_ref()
        .is_some_and(|entered| !hit_path.contains(entered))
    {
        state.editor_ui.entered_container = None;
    }
    should_start_drag
}

/// Outcome of the first cursor move that promotes a press into a drag.
pub struct NodeDragActivation {
    /// Selection ids the Option/Alt gesture cloned away from — the drop
    /// policy must not treat them as containers.
    pub option_drag_source_ids: Vec<NodeId>,
    /// The clone (and any flex reorder) mutated the tree, so the caller
    /// invalidates its scene cache and repaints.
    pub duplicated: bool,
}

/// Promote a press into a drag: the gesture can no longer be the first
/// half of a double-click drill, and an Option/Alt drag clones the
/// selection in place before travelling.
pub fn activate_node_drag(
    state: &mut EditorState,
    next_node_id: &mut u64,
    alt_held: bool,
    total_dx: f64,
    total_dy: f64,
) -> NodeDragActivation {
    // Once the gesture becomes a drag it cannot be the first half of a
    // later double-click drill.
    state.editor_ui.last_canvas_click = None;
    let option_source_ids: Vec<NodeId> = state.selection.set.to_vec();
    if alt_held
        && !option_source_ids.is_empty()
        && state.duplicate_selected(next_node_id, 0.0).is_some()
    {
        let _ = state.move_selected_in_layout_direction(total_dx, total_dy);
        NodeDragActivation {
            option_drag_source_ids: option_source_ids,
            duplicated: true,
        }
    } else {
        NodeDragActivation {
            option_drag_source_ids: Vec::new(),
            duplicated: false,
        }
    }
}
