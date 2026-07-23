//! Deterministic mobile content-rail ownership.
//!
//! Generated mobile screens commonly mix full-width chrome with individually
//! padded content sections. A root-level gutter cannot represent that shape:
//! it would inset status/navigation chrome and destroy intentional horizontal
//! scrollers. This pass therefore repairs only transparent root-direct content
//! sections, and gives clipped horizontal scrollers a leading rail while
//! keeping their trailing edge flush.

use crate::types::DocSink;
use jian_ops_schema::node::{
    container::{ContainerProps, LayoutMode},
    Padding, PenNode,
};
use jian_ops_schema::sizing::SizingBehavior;
use op_editor_core::{EditorCommand, LayoutPropValue, NodeId, PenNodeExt};
use std::collections::BTreeMap;

const DEFAULT_MOBILE_RAIL: f64 = 24.0;
const MIN_MOBILE_WIDTH: f64 = 320.0;
const MAX_MOBILE_WIDTH: f64 = 480.0;

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
    let repairs = {
        let Some(root) = sink
            .state()
            .active_children()
            .iter()
            .find(|node| node.id_str() == root_id)
        else {
            return;
        };
        collect_repairs(root)
    };

    for repair in repairs {
        sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(repair.node_id),
            property: "padding".to_string(),
            value: LayoutPropValue::NumberArray(repair.padding),
        });
    }
}

#[derive(Debug, PartialEq)]
struct RailRepair {
    node_id: String,
    padding: Vec<f64>,
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
    let rail = infer_content_rail(sections);
    let mut repairs = Vec::new();

    for section in sections {
        if !is_repairable_content_section(section)
            || has_expression_padding(section)
            || horizontal_padding(section).is_some_and(nonzero_pair)
        {
            continue;
        }

        let scrollers: Vec<&PenNode> = section
            .children()
            .into_iter()
            .flatten()
            .filter(|child| is_clipped_horizontal_scroller(child))
            .collect();
        if scrollers.is_empty() {
            repairs.push(RailRepair {
                node_id: section.id_str().to_string(),
                padding: padding_with_horizontal_rail(section, rail),
            });
            continue;
        }

        // A horizontal rail needs a flush trailing edge so the last card can
        // scroll offscreen. Keep the section full-width, inset its short
        // header siblings, and add only a leading inset to each viewport.
        for child in section.children().into_iter().flatten() {
            if is_clipped_horizontal_scroller(child) {
                if !has_expression_padding(child)
                    && horizontal_padding(child).is_none_or(|pair| !nonzero_pair(pair))
                {
                    repairs.push(RailRepair {
                        node_id: child.id_str().to_string(),
                        padding: padding_with_leading_rail(child, rail),
                    });
                }
            } else if is_scroller_header(child)
                && !has_expression_padding(child)
                && horizontal_padding(child).is_none_or(|pair| !nonzero_pair(pair))
            {
                repairs.push(RailRepair {
                    node_id: child.id_str().to_string(),
                    padding: padding_with_horizontal_rail(child, rail),
                });
            }
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
        if is_mobile_chrome(section) {
            continue;
        }
        let Some((left, right)) = horizontal_padding(section) else {
            continue;
        };
        if (left - right).abs() > 0.5 || !(16.0..=28.0).contains(&left) {
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

fn is_repairable_content_section(node: &PenNode) -> bool {
    node.is_container()
        && !is_mobile_chrome(node)
        && is_transparent_surface(node)
        && has_text_or_icon_descendant(node)
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
        "status-bar" | "bottom-tab-bar" | "bottom-nav" | "tab-bar" | "tabbar"
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

fn has_text_or_icon_descendant(node: &PenNode) -> bool {
    matches!(node, PenNode::Text(_) | PenNode::IconFont(_))
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_text_or_icon_descendant))
}

fn is_clipped_horizontal_scroller(node: &PenNode) -> bool {
    container_props(node).is_some_and(|props| {
        props.layout == Some(LayoutMode::Horizontal) && props.clip_content == Some(true)
    })
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
