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
const MAX_HEADER_NEIGHBOR_SIBLINGS: usize = 4;

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
    // A two-script fresh generation can append a second, byte-equivalent
    // page-content shell under the same mobile page. Merge that narrow shape
    // before rail/header collection so every later repair sees one coherent
    // ownership tree. The merge itself is one atomic editor command.
    merge_duplicate_page_shells(sink, root_id);

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
            RailRepair::WrapLooseNode {
                node_id,
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
                    node_id: NodeId::new(node_id),
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
            RailRepair::MoveIntoHeader {
                header_id,
                children,
            } => {
                let mut moved = Vec::new();
                for (node_id, original_index) in children {
                    if sink.apply(EditorCommand::MoveNode {
                        node_id: NodeId::new(node_id.clone()),
                        target_parent: NodeId::new(header_id.clone()),
                        page_id: None,
                        index: None,
                    }) {
                        moved.push((node_id, original_index));
                        continue;
                    }

                    // The repair is an atomic ownership decision: if one of
                    // the validated siblings cannot move, put the earlier
                    // siblings back instead of leaving a half-filled header.
                    for (moved_id, moved_index) in moved.into_iter().rev() {
                        sink.apply(EditorCommand::MoveNode {
                            node_id: NodeId::new(moved_id),
                            target_parent: NodeId::new(apply_root_id.clone()),
                            page_id: None,
                            index: Some(moved_index),
                        });
                    }
                    break;
                }
            }
        }
    }
}

/// Merge repeated root-direct page-content shells created by independent
/// fresh-generation scripts. This is intentionally a whole-document repair:
/// selected-frame append and multi-screen documents stay out of scope.
fn merge_duplicate_page_shells(sink: &mut dyn DocSink, root_id: &str) {
    let commands = {
        let roots = sink.state().active_children();
        let [root] = roots else {
            return;
        };
        if root.id_str() != root_id || !looks_like_mobile_screen(root) {
            return;
        }
        collect_page_shell_merge_commands(root)
    };

    if let Some(commands) = commands {
        // `EditorCommand::Batch` snapshots and rolls back the whole document
        // when any move/delete is rejected. No partially merged shell can
        // escape on a concurrent-edit or stale-state failure.
        sink.apply(EditorCommand::Batch { commands });
    }
}

fn collect_page_shell_merge_commands(root: &PenNode) -> Option<Vec<EditorCommand>> {
    let mut groups: BTreeMap<String, Vec<&PenNode>> = BTreeMap::new();
    for child in root.children()? {
        if let Some(name) = page_shell_name(child) {
            groups.entry(name).or_default().push(child);
        }
    }

    let mut duplicate_groups = groups.into_values().filter(|shells| shells.len() >= 2);
    let shells = duplicate_groups.next()?;
    // Two independently duplicated shell families under one page are not a
    // high-confidence two-script shape. Decline instead of merging either.
    if duplicate_groups.next().is_some() {
        return None;
    }

    let first_fingerprint = page_shell_fingerprint(shells[0])?;
    if shells.iter().skip(1).any(|shell| {
        page_shell_fingerprint(shell).as_ref() != Some(&first_fingerprint)
            || !page_shell_heights_are_mergeable(shells[0], shell)
    }) {
        return None;
    }

    let target_id = NodeId::new(shells[0].id_str().to_string());
    let mut commands = Vec::new();
    for shell in shells.iter().skip(1) {
        for child in shell.children()? {
            commands.push(EditorCommand::MoveNode {
                node_id: NodeId::new(child.id_str().to_string()),
                target_parent: target_id.clone(),
                page_id: None,
                index: None,
            });
        }
    }
    for shell in shells.iter().skip(1) {
        commands.push(EditorCommand::DeleteNode {
            node_id: NodeId::new(shell.id_str().to_string()),
            page_id: None,
        });
    }
    Some(commands)
}

fn page_shell_name(node: &PenNode) -> Option<String> {
    let PenNode::Frame(frame) = node else {
        return None;
    };
    if frame.container.layout != Some(LayoutMode::Vertical)
        || !matches!(
            frame.container.width.as_ref(),
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        )
        || frame.screen.is_some()
        || frame.route.is_some()
        || frame.breakpoint.is_some()
        || !frame.children.as_ref().is_some_and(|children| {
            children
                .iter()
                .any(op_design_lint::node_util::is_node_visible)
        })
    {
        return None;
    }

    let normalized = frame
        .base
        .name
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    matches!(
        normalized.as_str(),
        "app content" | "page content" | "main content" | "content root"
    )
    .then_some(normalized)
}

/// Canonical shell attributes. Height is deliberately excluded: the measured
/// two-script lesion is `fit_content` followed by a stale numeric flow height.
/// Padding encodings are normalized so `[v,h]` and `[v,h,v,h]` compare by
/// meaning; every other authored property remains an exact equality gate.
fn page_shell_fingerprint(node: &PenNode) -> Option<serde_json::Value> {
    let mut value = serde_json::to_value(node).ok()?;
    let object = value.as_object_mut()?;
    object.remove("id");
    object.remove("name");
    object.remove("height");
    object.remove("children");
    if let Some(padding) = container_props(node)?.padding.as_ref() {
        let canonical = match padding {
            Padding::Uniform(value) => serde_json::json!([value, value, value, value]),
            Padding::XY([vertical, horizontal]) => {
                serde_json::json!([vertical, horizontal, vertical, horizontal])
            }
            Padding::LtrB(values) => serde_json::json!(values),
            Padding::Expression(expression) => serde_json::json!(expression),
        };
        object.insert("padding".to_string(), canonical);
    }
    Some(value)
}

fn page_shell_heights_are_mergeable(first: &PenNode, later: &PenNode) -> bool {
    let first_height = container_props(first).and_then(|props| props.height.as_ref());
    let later_height = container_props(later).and_then(|props| props.height.as_ref());
    first_height == later_height
        || page_shell_height_is_hug(first_height)
        || page_shell_height_is_hug(later_height)
}

fn page_shell_height_is_hug(height: Option<&SizingBehavior>) -> bool {
    height.is_none()
        || matches!(
            height,
            Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
        )
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
        RailRepair::WrapLooseNode { node_id, .. } => scope.contains(node_id),
        RailRepair::MoveIntoHeader {
            header_id,
            children,
        } => {
            scope.contains(header_id) && children.iter().all(|(node_id, _)| scope.contains(node_id))
        }
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
            RailRepair::WrapSurface { .. }
            | RailRepair::WrapLooseNode { .. }
            | RailRepair::MoveIntoHeader { .. } => None,
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
    WrapLooseNode {
        node_id: String,
        wrapper: Box<PenNode>,
        original_index: usize,
    },
    MoveIntoHeader {
        header_id: String,
        children: Vec<(String, usize)>,
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
    let header_adoption = collect_header_adoption(sections);
    let adopted_ids: HashSet<&str> = header_adoption
        .as_ref()
        .into_iter()
        .flat_map(|adoption| {
            adoption
                .children
                .iter()
                .map(|(node_id, _)| node_id.as_str())
        })
        .collect();

    for (index, section) in sections.iter().enumerate() {
        if adopted_ids.contains(section.id_str()) {
            continue;
        }

        if is_ordinary_root_leaf(section) {
            let wrapper_id = unique_wrapper_id(section.id_str(), &mut known_ids);
            repairs.push(RailRepair::WrapLooseNode {
                node_id: section.id_str().to_string(),
                wrapper: Box::new(content_rail_wrapper(
                    &wrapper_id,
                    section,
                    DEFAULT_MOBILE_RAIL,
                )),
                original_index: index,
            });
            continue;
        }

        if !section.is_container()
            || is_mobile_chrome(section)
            || is_intentional_full_bleed_role(section)
            || is_empty_header_shell(section)
            || is_compact_header_action(section)
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

    // Structural wrapping above replaces nodes one-for-one at their original
    // indices. Reparent header children last so those precomputed indices stay
    // stable while wrappers are inserted and moved into place.
    if let Some(adoption) = header_adoption {
        repairs.push(RailRepair::MoveIntoHeader {
            header_id: adoption.header_id,
            children: adoption.children,
        });
    }

    repairs
}

#[derive(Debug)]
struct HeaderAdoption {
    header_id: String,
    children: Vec<(String, usize)>,
}

/// Finds the narrow, generated failure shape where an empty authored header
/// shell is immediately followed by the content that clearly belongs to it.
/// Both a brand title and a compact semantic action are required, and only a
/// search rail may sit between them. This deliberately declines plausible but
/// ambiguous reconstruction instead of guessing at document ownership.
fn collect_header_adoption(sections: &[PenNode]) -> Option<HeaderAdoption> {
    let candidates: Vec<(usize, &PenNode)> = sections
        .iter()
        .enumerate()
        .filter(|(_, node)| is_empty_header_shell(node))
        .collect();
    let [(header_index, header)] = candidates.as_slice() else {
        return None;
    };

    let mut brands = Vec::new();
    let mut actions = Vec::new();
    for (index, node) in sections
        .iter()
        .enumerate()
        .skip(*header_index + 1)
        .take(MAX_HEADER_NEIGHBOR_SIBLINGS)
    {
        if is_brand_title(node) {
            brands.push((node.id_str().to_string(), index));
        } else if is_compact_header_action(node) {
            actions.push((node.id_str().to_string(), index));
        } else if !is_search_rail(node) {
            break;
        }
    }

    let ([brand], [action]) = (brands.as_slice(), actions.as_slice()) else {
        return None;
    };
    if brand.1 >= action.1 {
        return None;
    }

    Some(HeaderAdoption {
        header_id: header.id_str().to_string(),
        children: vec![brand.clone(), action.clone()],
    })
}

fn is_empty_header_shell(node: &PenNode) -> bool {
    if is_mobile_chrome(node)
        || !is_transparent_surface(node)
        || has_authored_position(node)
        || node.children().is_none_or(|children| !children.is_empty())
    {
        return false;
    }
    let Some(props) = container_props(node) else {
        return false;
    };
    if props.layout != Some(LayoutMode::Horizontal)
        || !matches!(
            props.width.as_ref(),
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        )
        || node.height_px().is_some_and(|height| height > 96.0)
    {
        return false;
    }

    let semantic = semantic_label(node);
    semantic_has_any(&semantic, &["header", "navbar", "nav", "navigation"])
}

fn is_brand_title(node: &PenNode) -> bool {
    let PenNode::Text(text) = node else {
        return false;
    };
    if has_authored_position(node) || !(18.0..=40.0).contains(&text.font_size.unwrap_or(16.0)) {
        return false;
    }
    semantic_has_any(&semantic_label(node), &["brand", "logo", "wordmark"])
}

fn is_compact_header_action(node: &PenNode) -> bool {
    if has_authored_position(node) {
        return false;
    }
    let semantic = match node {
        PenNode::IconFont(icon) => format!(
            "{} {}",
            semantic_label(node),
            icon.icon_font_name.to_ascii_lowercase()
        ),
        _ => semantic_label(node),
    };
    if !semantic_has_any(
        &semantic,
        &[
            "action",
            "button",
            "cart",
            "bag",
            "menu",
            "profile",
            "account",
            "avatar",
            "notification",
            "bell",
            "favorite",
            "wishlist",
            "search",
        ],
    ) {
        return false;
    }

    match node {
        PenNode::IconFont(icon) => {
            let icon_semantic = icon.icon_font_name.to_ascii_lowercase();
            semantic_has_any(
                &format!("{semantic} {icon_semantic}"),
                &[
                    "cart",
                    "bag",
                    "menu",
                    "profile",
                    "account",
                    "avatar",
                    "notification",
                    "bell",
                    "favorite",
                    "wishlist",
                    "search",
                ],
            ) && node.width_px().is_some_and(|width| width <= 48.0)
                && node.height_px().is_some_and(|height| height <= 48.0)
        }
        _ if node.is_container() => {
            node.width_px()
                .is_some_and(|width| width > 0.0 && width <= 64.0)
                && node
                    .height_px()
                    .is_some_and(|height| height > 0.0 && height <= 64.0)
                && node
                    .children()
                    .is_some_and(|children| !children.is_empty() && children.len() <= 2)
                && has_icon_descendant(node)
                && !has_text_descendant(node)
        }
        _ => false,
    }
}

fn is_search_rail(node: &PenNode) -> bool {
    node.is_container()
        && semantic_has_any(&semantic_label(node), &["search"])
        && container_props(node).is_some_and(|props| {
            matches!(
                props.width.as_ref(),
                Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
            )
        })
}

fn is_ordinary_root_leaf(node: &PenNode) -> bool {
    if !matches!(node, PenNode::Text(_) | PenNode::IconFont(_))
        || has_authored_position(node)
        || is_mobile_chrome(node)
        || is_intentional_full_bleed_role(node)
    {
        return false;
    }
    let semantic = semantic_label(node);
    !has_system_chrome_semantics(&semantic)
        && !semantic_has_any(
            &semantic,
            &[
                "header", "navbar", "brand", "logo", "wordmark", "hero", "banner", "cover",
            ],
        )
        && !is_compact_header_action(node)
}

fn has_system_chrome_semantics(semantic: &str) -> bool {
    matches!(
        semantic.trim(),
        "time"
            | "wifi"
            | "wi-fi"
            | "cellular"
            | "cellular connection"
            | "battery"
            | "battery capacity"
            | "system status"
            | "status icon"
    )
}

fn has_icon_descendant(node: &PenNode) -> bool {
    matches!(node, PenNode::IconFont(_))
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_icon_descendant))
}

fn has_text_descendant(node: &PenNode) -> bool {
    matches!(node, PenNode::Text(_))
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_text_descendant))
}

fn semantic_label(node: &PenNode) -> String {
    format!(
        "{} {}",
        node.base().role.as_deref().unwrap_or(""),
        node.base().name.as_deref().unwrap_or("")
    )
    .trim()
    .to_ascii_lowercase()
}

fn semantic_has_any(semantic: &str, candidates: &[&str]) -> bool {
    semantic
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| candidates.contains(&word))
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
    let numeric_min_height_is_mobile = props
        .limits
        .min_height
        .is_some_and(|height| height.is_finite() && height >= 568.0);
    let tall_or_screen_structured = numeric_min_height_is_mobile
        || match props.height {
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
