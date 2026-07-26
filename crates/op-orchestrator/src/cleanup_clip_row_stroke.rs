//! Clipping-row stroke padding repair — pad a `clipContent` horizontal row so
//! a stroked child's outline isn't cropped.

use super::*;

pub(super) fn pad_clipping_horizontal_row_for_stroke(sink: &mut dyn DocSink, root_id: &str) {
    let repairs: Vec<ClipRowStrokePaddingRepair> = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return;
        };
        let mut repairs = Vec::new();
        collect_clip_row_stroke_padding_repairs(root, &mut repairs);
        repairs
    };

    for repair in repairs {
        sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id: repair.node_id,
            property: "padding".to_string(),
            value: LayoutPropValue::NumberArray(vec![
                repair.padding[0],
                repair.padding[1],
                repair.padding[2],
                repair.padding[3],
            ]),
        });
    }
}

#[derive(Debug, Clone)]
pub(super) struct ClipRowStrokePaddingRepair {
    node_id: NodeId,
    padding: [f64; 4],
}

pub(super) fn collect_clip_row_stroke_padding_repairs(
    node: &PenNode,
    repairs: &mut Vec<ClipRowStrokePaddingRepair>,
) {
    if let Some(repair) = clip_row_stroke_padding_repair(node) {
        repairs.push(repair);
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_clip_row_stroke_padding_repairs(child, repairs);
        }
    }
}

pub(super) fn clip_row_stroke_padding_repair(node: &PenNode) -> Option<ClipRowStrokePaddingRepair> {
    let props = frame_container_props(node)?;
    if props.layout.as_ref() != Some(&LayoutMode::Horizontal) || props.clip_content != Some(true) {
        return None;
    }
    let max_stroke = node
        .children()?
        .iter()
        .filter_map(node_stroke_width)
        .max_by(f64::total_cmp)?;
    let mut padding = props
        .padding
        .as_ref()
        .map(padding_sides)
        .unwrap_or([0.0, 0.0, 0.0, 0.0]);
    let stroke_padding = max_stroke.ceil();
    if padding.iter().all(|side| *side >= stroke_padding) {
        return None;
    }
    padding[0] = padding[0].max(stroke_padding);
    padding[2] = padding[2].max(stroke_padding);
    padding[3] = padding[3].max(stroke_padding);
    // A fill-width clipped row can be an intentional horizontal rail: keep
    // its trailing edge flush so the next item may remain visibly cropped.
    // A fit-content row, however, hugs its children and has no overflow
    // affordance to preserve. Its trailing clip would only shave the outer
    // half of the last child's stroke (notably a bordered avatar).
    if matches!(
        props.width.as_ref(),
        Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
    ) {
        padding[1] = padding[1].max(stroke_padding);
    }
    Some(ClipRowStrokePaddingRepair {
        node_id: NodeId::new(node.id_str().to_string()),
        padding,
    })
}
