//! Deterministic mobile content-rail ownership.
//!
//! Generated mobile screens commonly mix full-width chrome with individually
//! padded content sections. A root-level gutter cannot represent that shape:
//! it would inset status/navigation chrome and destroy intentional horizontal
//! scrollers. This pass repairs transparent root-direct content sections,
//! wraps unambiguously edge-spanning rounded/stroked cards in a transparent
//! rail owner, and gives clipped horizontal scrollers a leading rail while
//! keeping their trailing edge flush.

use crate::types::DocSink;
use jian_ops_schema::node::{
    container::{ContainerProps, LayoutMode},
    Padding, PenNode,
};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::{EditorCommand, LayoutPropValue, NodeId, PenNodeExt};
use std::collections::{BTreeMap, HashSet};

const DEFAULT_MOBILE_RAIL: f64 = 24.0;
const MIN_MOBILE_WIDTH: f64 = 320.0;
const MAX_MOBILE_WIDTH: f64 = 480.0;
const MIN_CONTENT_RAIL: f64 = 16.0;
const MAX_CONTENT_RAIL: f64 = 28.0;

pub(crate) fn repair_mobile_content_rails_for_all_roots(sink: &mut dyn DocSink) {
    let root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect();
    for root_id in root_ids {
        repair_mobile_content_rails(sink, &root_id);
    }
}

pub(crate) fn repair_mobile_content_rails(sink: &mut dyn DocSink, root_id: &str) {
    // `root_id` is a top-level active root in the common case (fresh-document
    // generation), but the classic selected-frame append path nests the
    // inserted subtree root under an existing top-level mobile screen instead
    // of placing it at the top level (the same "Component 11c" shape
    // `cleanup.rs::find_root` already works around). The appended fragment
    // itself is usually just a section — it will not pass
    // `looks_like_mobile_screen` on its own — so on a miss we walk up to the
    // enclosing top-level screen to use as detection CONTEXT, but keep
    // mutations scoped to `root_id`'s own subtree so pre-existing content
    // elsewhere in that screen is never touched.
    let (repairs, apply_root_id) = {
        let roots = sink.state().active_children();
        if let Some(root) = roots.iter().find(|node| node.id_str() == root_id) {
            (collect_repairs(root), root_id.to_string())
        } else if let Some((screen, scope)) = find_enclosing_top_level(roots, root_id) {
            let repairs = collect_repairs(screen)
                .into_iter()
                .filter(|repair| repair_touches_scope(repair, &scope))
                .collect();
            (repairs, screen.id_str().to_string())
        } else {
            return;
        }
    };
    // Scope filtering above is per-repair, but some repairs are only correct
    // as a PAIR (a scroller's own leading-rail add + its inner lane's
    // matching leading-rail clear): if the append's scope contains only the
    // inner lane, filtering can keep the "clear" half and drop the "add"
    // half, net-erasing the rail entirely. Drop any repair whose declared
    // partner did not survive scoping.
    let repairs = drop_unpaired_repairs(repairs);

    for repair in repairs {
        match repair {
            RailRepair::SetPadding {
                node_id, padding, ..
            } => {
                sink.apply(EditorCommand::SetNodeLayoutProp {
                    node_id: NodeId::new(node_id),
                    property: "padding".to_string(),
                    value: LayoutPropValue::NumberArray(padding),
                });
            }
            RailRepair::WrapSurface {
                surface_id,
                wrapper,
                original_index,
            } => {
                let wrapper_id = NodeId::new(wrapper.id_str().to_string());
                if !sink.apply(EditorCommand::InsertAuthoredSubtree {
                    nodes: vec![*wrapper],
                    parent_id: NodeId::new(apply_root_id.clone()),
                    page_id: None,
                }) {
                    continue;
                }
                if !sink.apply(EditorCommand::MoveNode {
                    node_id: NodeId::new(surface_id),
                    target_parent: wrapper_id.clone(),
                    page_id: None,
                    index: None,
                }) {
                    sink.apply(EditorCommand::DeleteNode {
                        node_id: wrapper_id,
                        page_id: None,
                    });
                    continue;
                }
                sink.apply(EditorCommand::MoveNode {
                    node_id: wrapper_id,
                    target_parent: NodeId::new(apply_root_id.clone()),
                    page_id: None,
                    index: Some(original_index),
                });
            }
        }
    }
}

/// Finds the top-level active root whose subtree contains `target_id`,
/// returning that root together with the full id set of `target_id`'s own
/// subtree (the mutation-scope allowlist).
fn find_enclosing_top_level<'a>(
    roots: &'a [PenNode],
    target_id: &str,
) -> Option<(&'a PenNode, HashSet<String>)> {
    roots
        .iter()
        .find_map(|root| subtree_ids_rooted_at(root, target_id).map(|scope| (root, scope)))
}

/// Returns the id set of `target_id`'s own subtree (including itself) if
/// `target_id` appears anywhere under `node`, else `None`.
fn subtree_ids_rooted_at(node: &PenNode, target_id: &str) -> Option<HashSet<String>> {
    if node.id_str() == target_id {
        let mut ids = HashSet::new();
        collect_node_ids(node, &mut ids);
        return Some(ids);
    }
    node.children()
        .into_iter()
        .flatten()
        .find_map(|child| subtree_ids_rooted_at(child, target_id))
}

/// True when a repair's mutation target lies inside the scoped subtree.
fn repair_touches_scope(repair: &RailRepair, scope: &HashSet<String>) -> bool {
    match repair {
        RailRepair::SetPadding { node_id, .. } => scope.contains(node_id),
        RailRepair::WrapSurface { surface_id, .. } => scope.contains(surface_id),
    }
}

/// Drops any `SetPadding` repair whose `requires` partner (the other half of
/// an atomic outer-add/inner-clear pair, see `collect_scroller_padding_repairs`)
/// is not itself present in `repairs`. A no-op when `repairs` is unfiltered
/// (both halves of every pair are always present together), so this is safe
/// to run unconditionally.
fn drop_unpaired_repairs(repairs: Vec<RailRepair>) -> Vec<RailRepair> {
    let present: HashSet<String> = repairs
        .iter()
        .filter_map(|repair| match repair {
            RailRepair::SetPadding { node_id, .. } => Some(node_id.clone()),
            RailRepair::WrapSurface { .. } => None,
        })
        .collect();
    repairs
        .into_iter()
        .filter(|repair| match repair {
            RailRepair::SetPadding {
                requires: Some(req),
                ..
            } => present.contains(req),
            _ => true,
        })
        .collect()
}

#[derive(Debug)]
enum RailRepair {
    SetPadding {
        node_id: String,
        padding: Vec<f64>,
        /// When `Some(id)`, this repair is only meaningful if the repair
        /// targeting `id` is ALSO present in the final (post-scope-filter)
        /// list — see `collect_scroller_padding_repairs`'s outer-add /
        /// inner-clear pair.
        requires: Option<String>,
    },
    WrapSurface {
        surface_id: String,
        wrapper: Box<PenNode>,
        original_index: usize,
    },
}

fn collect_repairs(root: &PenNode) -> Vec<RailRepair> {
    if !looks_like_mobile_screen(root)
        || has_expression_padding(root)
        || horizontal_padding(root).is_some_and(nonzero_pair)
    {
        return Vec::new();
    }
    let Some(sections) = root.children() else {
        return Vec::new();
    };
    let root_width = root.width_px().unwrap_or_default();
    let rail = infer_content_rail(sections);
    let mut repairs = Vec::new();
    let mut known_ids = HashSet::new();
    collect_node_ids(root, &mut known_ids);

    for (index, section) in sections.iter().enumerate() {
        if !section.is_container()
            || is_mobile_chrome(section)
            || is_intentional_full_bleed_role(section)
            || has_expression_padding(section)
        {
            continue;
        }

        // A root-direct horizontal viewport owns its own asymmetric rail,
        // including image-only carousels that have no text/icon descendants.
        if is_clipped_horizontal_scroller(section) {
            collect_scroller_padding_repairs(section, rail, &mut repairs);
            continue;
        }

        // A root-direct surfaced card needs an OUTER rail owner; padding the
        // card itself would inset only its contents while leaving the border
        // glued to the viewport. Prefer this structural repair even when the
        // card contains a full-width cover image. Authored absolute-positioned
        // surfaces are deliberately excluded by the predicate below.
        if !is_transparent_surface(section) {
            if has_surface_content_descendant(section)
                && is_edge_spanning_insettable_surface(section, root_width)
            {
                let wrapper_id = unique_wrapper_id(section.id_str(), &mut known_ids);
                repairs.push(RailRepair::WrapSurface {
                    surface_id: section.id_str().to_string(),
                    wrapper: Box::new(content_rail_wrapper(&wrapper_id, section, rail)),
                    original_index: index,
                });
            }
            continue;
        }

        let scrollers: Vec<&PenNode> = section
            .children()
            .into_iter()
            .flatten()
            .filter(|child| is_clipped_horizontal_scroller(child))
            .collect();
        if !scrollers.is_empty() {
            // A horizontal rail needs a flush trailing edge so the last card
            // can scroll offscreen. Keep the section full-width, inset its
            // short header siblings, and add only a leading inset to each
            // viewport.
            for child in section.children().into_iter().flatten() {
                if is_clipped_horizontal_scroller(child) {
                    collect_scroller_padding_repairs(child, rail, &mut repairs);
                } else if is_scroller_header(child)
                    && !has_expression_padding(child)
                    && horizontal_padding(child).is_none_or(|pair| !nonzero_pair(pair))
                {
                    repairs.push(RailRepair::SetPadding {
                        node_id: child.id_str().to_string(),
                        padding: padding_with_horizontal_rail(child, rail),
                        requires: None,
                    });
                }
            }
            continue;
        }

        // A deeper scroller has no unambiguous direct header/viewport owner.
        // Leave it untouched rather than putting a symmetric rail on an
        // ancestor and silently closing its intentional trailing edge.
        if contains_clipped_horizontal_scroller(section) {
            continue;
        }

        // Transparent media-overlay owners are intentional full bleed. This
        // must stay aligned with the design-lint detector so pre-validation
        // cannot undo the cleanup decision on the next pipeline stage.
        if has_full_bleed_media_child(section, root_width) || !has_text_or_icon_descendant(section)
        {
            continue;
        }

        if horizontal_padding(section).is_none_or(|pair| !nonzero_pair(pair)) {
            repairs.push(RailRepair::SetPadding {
                node_id: section.id_str().to_string(),
                padding: padding_with_horizontal_rail(section, rail),
                requires: None,
            });
        }
    }

    repairs
}

fn looks_like_mobile_screen(root: &PenNode) -> bool {
    let Some(props) = container_props(root) else {
        return false;
    };
    let Some(SizingBehavior::Number(width)) = props.width else {
        return false;
    };
    if !(MIN_MOBILE_WIDTH..=MAX_MOBILE_WIDTH).contains(&width)
        || props.layout != Some(LayoutMode::Vertical)
    {
        return false;
    }
    let Some(children) = root.children() else {
        return false;
    };
    let tall_or_screen_structured = match props.height {
        Some(SizingBehavior::Number(height)) => height >= 568.0,
        _ => children.len() >= 4 || children.iter().any(is_mobile_chrome),
    };
    tall_or_screen_structured && children.len() >= 2
}

fn infer_content_rail(sections: &[PenNode]) -> f64 {
    let mut counts: BTreeMap<i64, usize> = BTreeMap::new();
    for section in sections {
        if is_mobile_chrome(section)
            || is_intentional_full_bleed_role(section)
            || !is_transparent_surface(section)
        {
            continue;
        }
        let Some((left, right)) = horizontal_padding(section) else {
            continue;
        };
        if (left - right).abs() > 0.5 || !(MIN_CONTENT_RAIL..=MAX_CONTENT_RAIL).contains(&left) {
            continue;
        }
        *counts.entry(left.round() as i64).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(rail, count)| (*count, *rail))
        .map(|(rail, _)| rail as f64)
        .unwrap_or(DEFAULT_MOBILE_RAIL)
}

fn is_mobile_chrome(node: &PenNode) -> bool {
    let role = node
        .base()
        .role
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if matches!(
        role.as_str(),
        "status-bar"
            | "bottom-tab-bar"
            | "bottom-nav"
            | "bottom-navigation-bar"
            | "tab-bar"
            | "tabbar"
    ) {
        return true;
    }
    let name = node
        .base()
        .name
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    name.contains("status bar")
        || name.contains("bottom navigation")
        || name.contains("bottom nav")
        || name.contains("bottom tab")
}

fn is_intentional_full_bleed_role(node: &PenNode) -> bool {
    matches!(
        node.base()
            .role
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "hero" | "banner" | "cover" | "header" | "top-nav" | "navbar"
    )
}

fn is_transparent_surface(node: &PenNode) -> bool {
    let Ok(value) = serde_json::to_value(node) else {
        return false;
    };
    let has_fill = value
        .get("fill")
        .and_then(|fill| fill.as_array())
        .is_some_and(|fill| !fill.is_empty());
    let has_stroke = value.get("stroke").is_some_and(|stroke| !stroke.is_null());
    let has_effects = value
        .get("effects")
        .and_then(|effects| effects.as_array())
        .is_some_and(|effects| !effects.is_empty());
    let has_radius = value
        .get("cornerRadius")
        .and_then(|radius| radius.as_f64())
        .is_some_and(|radius| radius > 0.0);
    !has_fill && !has_stroke && !has_effects && !has_radius
}

fn is_edge_spanning_insettable_surface(node: &PenNode, root_width: f64) -> bool {
    let Some(props) = container_props(node) else {
        return false;
    };
    if has_authored_position(node) {
        return false;
    }
    let spans_width = matches!(
        props.width.as_ref(),
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    ) || matches!(
        props.width.as_ref(),
        Some(SizingBehavior::Number(width)) if *width >= root_width - 1.0
    );
    if !spans_width {
        return false;
    }
    let Ok(value) = serde_json::to_value(node) else {
        return false;
    };
    let has_stroke = value.get("stroke").is_some_and(|stroke| !stroke.is_null());
    let has_effects = value
        .get("effects")
        .and_then(|effects| effects.as_array())
        .is_some_and(|effects| !effects.is_empty());
    let has_radius = value
        .get("cornerRadius")
        .and_then(|radius| radius.as_f64())
        .is_some_and(|radius| radius > 0.0);
    has_stroke || has_effects || has_radius
}

fn has_authored_position(node: &PenNode) -> bool {
    serde_json::to_value(node).ok().is_some_and(|value| {
        value.get("x").is_some_and(|x| !x.is_null()) || value.get("y").is_some_and(|y| !y.is_null())
    })
}

fn has_full_bleed_media_child(node: &PenNode, root_width: f64) -> bool {
    node.children().is_some_and(|children| {
        children.iter().any(|child| {
            let role_is_media = matches!(
                child
                    .base()
                    .role
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .to_ascii_lowercase()
                    .as_str(),
                "hero" | "banner" | "cover" | "media" | "image-placeholder"
            );
            let image_like = matches!(child, PenNode::Image(_))
                || serde_json::to_value(child)
                    .ok()
                    .and_then(|value| value.get("fill").cloned())
                    .and_then(|fill| fill.as_array().cloned())
                    .is_some_and(|fills| {
                        fills.iter().any(|fill| {
                            fill.get("type").and_then(|kind| kind.as_str()) == Some("image")
                        })
                    });
            (role_is_media || image_like) && node_spans_width(child, root_width)
        })
    })
}

fn node_spans_width(node: &PenNode, root_width: f64) -> bool {
    container_props(node)
        .and_then(|props| props.width.as_ref())
        .is_some_and(|width| {
            matches!(width, SizingBehavior::Keyword(SizingKeyword::FillContainer))
                || matches!(width, SizingBehavior::Number(width) if *width >= root_width - 1.0)
        })
        || matches!(node, PenNode::Image(image) if {
            matches!(
                image.width.as_ref(),
                Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
            ) || matches!(
                image.width.as_ref(),
                Some(SizingBehavior::Number(width)) if *width >= root_width - 1.0
            )
        })
}

fn has_text_or_icon_descendant(node: &PenNode) -> bool {
    matches!(node, PenNode::Text(_) | PenNode::IconFont(_))
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_text_or_icon_descendant))
}

fn has_surface_content_descendant(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Text(_) | PenNode::IconFont(_) | PenNode::Image(_)
    ) || node
        .children()
        .is_some_and(|children| children.iter().any(has_surface_content_descendant))
}

fn is_clipped_horizontal_scroller(node: &PenNode) -> bool {
    container_props(node).is_some_and(|props| {
        props.layout == Some(LayoutMode::Horizontal) && props.clip_content == Some(true)
    })
}

fn collect_scroller_padding_repairs(scroller: &PenNode, rail: f64, repairs: &mut Vec<RailRepair>) {
    if has_expression_padding(scroller) {
        return;
    }

    let current_padding = horizontal_padding(scroller);
    let needs_leading_rail = current_padding.is_none_or(|(left, _)| left <= 0.0);

    if needs_leading_rail {
        repairs.push(RailRepair::SetPadding {
            node_id: scroller.id_str().to_string(),
            padding: padding_with_leading_rail(scroller, rail),
            requires: None,
        });
    }

    if let Some((node_id, padding, inner_left)) = redundant_inner_scroller_rail_repair(scroller) {
        let duplicates_existing_rail =
            current_padding.is_some_and(|(outer_left, _)| (outer_left - inner_left).abs() <= 0.5);
        if needs_leading_rail || duplicates_existing_rail {
            // The inner lane's rail is only redundant once the OUTER
            // scroller actually owns a leading rail. When this push is
            // triggered by `needs_leading_rail`, the outer's own add above
            // is what establishes that ownership — if scope filtering drops
            // the outer add (e.g. an append whose inserted root is this
            // inner lane itself, not the outer viewport), clearing the inner
            // lane alone would erase the rail entirely. `duplicates_existing_rail`
            // needs no such pairing: the outer already owns a rail we are not
            // touching, so the inner clear stands on its own.
            let requires = needs_leading_rail.then(|| scroller.id_str().to_string());
            repairs.push(RailRepair::SetPadding {
                node_id,
                padding,
                requires,
            });
        }
    }
}

fn redundant_inner_scroller_rail_repair(scroller: &PenNode) -> Option<(String, Vec<f64>, f64)> {
    let children = scroller.children()?;
    let [inner] = children.as_slice() else {
        return None;
    };
    let props = container_props(inner)?;
    if props.layout != Some(LayoutMode::Horizontal)
        || props.clip_content == Some(true)
        || !matches!(
            props.width.as_ref(),
            Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
        )
        || !is_transparent_surface(inner)
        || has_expression_padding(inner)
        || inner.children().map_or(0, |children| children.len()) < 2
    {
        return None;
    }

    let (left, right) = horizontal_padding(inner)?;
    if !(MIN_CONTENT_RAIL..=MAX_CONTENT_RAIL).contains(&left) {
        return None;
    }
    let (top, bottom) = vertical_padding(inner);
    Some((
        inner.id_str().to_string(),
        vec![top, right, bottom, 0.0],
        left,
    ))
}

fn contains_clipped_horizontal_scroller(node: &PenNode) -> bool {
    if is_clipped_horizontal_scroller(node) {
        return true;
    }

    // Only transparent structural wrappers can pass scroller ownership up to
    // an ancestor section. Surfaced cards often contain clipped horizontal
    // progress meters; treating those meters as page rails suppresses the
    // card group's own mobile content inset.
    is_transparent_surface(node)
        && node
            .children()
            .is_some_and(|children| children.iter().any(contains_clipped_horizontal_scroller))
}

fn is_scroller_header(node: &PenNode) -> bool {
    if !node.is_container() || !is_transparent_surface(node) || !has_text_or_icon_descendant(node) {
        return false;
    }
    let name = node
        .base()
        .name
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.contains("header") || name.contains("title") {
        return true;
    }
    let child_count = node.children().map_or(0, |children| children.len());
    child_count <= 3
        && container_props(node).is_some_and(|props| {
            props.layout == Some(LayoutMode::Horizontal)
                && props.height.as_ref().is_none_or(|height| match height {
                    SizingBehavior::Number(height) => *height <= 80.0,
                    _ => true,
                })
        })
}

fn container_props(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(node) => Some(&node.container),
        PenNode::Group(node) => Some(&node.container),
        PenNode::Rectangle(node) => Some(&node.container),
        _ => None,
    }
}

fn horizontal_padding(node: &PenNode) -> Option<(f64, f64)> {
    match container_props(node)?.padding.as_ref()? {
        Padding::Uniform(value) => Some((*value, *value)),
        Padding::XY([_, horizontal]) => Some((*horizontal, *horizontal)),
        Padding::LtrB([_, right, _, left]) => Some((*left, *right)),
        Padding::Expression(_) => None,
    }
}

fn has_expression_padding(node: &PenNode) -> bool {
    matches!(
        container_props(node).and_then(|props| props.padding.as_ref()),
        Some(Padding::Expression(_))
    )
}

fn vertical_padding(node: &PenNode) -> (f64, f64) {
    match container_props(node).and_then(|props| props.padding.as_ref()) {
        Some(Padding::Uniform(value)) => (*value, *value),
        Some(Padding::XY([vertical, _])) => (*vertical, *vertical),
        Some(Padding::LtrB([top, _, bottom, _])) => (*top, *bottom),
        Some(Padding::Expression(_)) | None => (0.0, 0.0),
    }
}

fn nonzero_pair((left, right): (f64, f64)) -> bool {
    left > 0.0 || right > 0.0
}

fn padding_with_horizontal_rail(node: &PenNode, rail: f64) -> Vec<f64> {
    let (top, bottom) = vertical_padding(node);
    vec![top, rail, bottom, rail]
}

fn padding_with_leading_rail(node: &PenNode, rail: f64) -> Vec<f64> {
    let (top, bottom) = vertical_padding(node);
    let right = horizontal_padding(node).map_or(0.0, |(_, right)| right);
    vec![top, right, bottom, rail]
}

// Single-sourced in op-editor-core (same walk as the reveal bookkeeping).
use op_editor_core::agent_reveals::collect_node_ids;

fn unique_wrapper_id(surface_id: &str, known_ids: &mut HashSet<String>) -> String {
    let base = format!("{surface_id}__content_rail");
    let mut candidate = base.clone();
    let mut suffix = 2usize;
    while known_ids.contains(&candidate) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    known_ids.insert(candidate.clone());
    candidate
}

fn content_rail_wrapper(id: &str, surface: &PenNode, rail: f64) -> PenNode {
    let name = surface.base().name.as_deref().unwrap_or("Content");
    serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": id,
        "name": format!("{name} Content Rail"),
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "padding": [0, rail, 0, rail],
        "children": []
    }))
    .expect("content rail wrapper is valid PenNode")
}
