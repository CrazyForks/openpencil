use crate::import_warning::ImportWarning;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use jian_ops_schema::node::base::{BoolOrExpression, NumberOrExpression, PenNodeBase};
use jian_ops_schema::node::container::CornerRadius;
use jian_ops_schema::node::{
    CheckboxNode, FrameNode, ImageFitMode, ImageNode, ImageSrc, PenNode, ProgressNode,
    RadioGroupNode, RectangleNode, SelectNode, SelectOption, SliderNode, TextAreaNode,
    TextInputNode,
};
use jian_ops_schema::sizing::{SizeLimits, SizingBehavior, SizingKeyword};
use jian_ops_schema::style::{BlendMode, PenEffect, PenFill, PenStroke, SolidFillBody};

use crate::css::cascade::ComputedStyle;
use crate::dom::{DomElement, DomNode};
use crate::mapper::{container_props_from, MapCtx};

/// `path` carries the ancestor chain because a replaced element can depend on
/// its parent: an `<img>` inside `<picture>` selects its source from the
/// sibling `<source>` list.
pub fn map_special(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    style: &ComputedStyle,
) -> Option<Option<PenNode>> {
    let element = *path.last()?;
    let node = match element.tag.as_str() {
        "img" => map_image(context, path, element, style),
        "svg" => map_svg(context, element, style),
        "input" => map_input(context, element, style),
        "textarea" => map_text_area(context, element, style),
        "select" => map_select(context, element, style),
        "progress" => map_progress(context, element, style),
        "hr" => map_horizontal_rule(context, style),
        "iframe" | "video" | "canvas" => map_placeholder(context, element, style),
        _ => return None,
    };
    Some(Some(node))
}

pub(crate) fn map_radio_group(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    style: &ComputedStyle,
) -> Option<Option<PenNode>> {
    let current = *path.last()?;
    if current.tag != "input" || current.attr("type") != Some("radio") {
        return None;
    }
    let name = current.attr("name");
    let radios: Vec<&DomElement> = path
        .get(path.len().saturating_sub(2))
        .map(|parent| {
            parent
                .children
                .iter()
                .filter_map(|child| match child {
                    DomNode::Element(element)
                        if element.tag == "input"
                            && element.attr("type") == Some("radio")
                            && element.attr("name") == name =>
                    {
                        Some(element)
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_else(|| vec![current]);
    if radios
        .first()
        .is_some_and(|first| !std::ptr::eq(*first, current))
    {
        return Some(None);
    }
    let options = radios
        .iter()
        .map(|radio| {
            let value = radio.attr("value").unwrap_or("on").to_string();
            SelectOption {
                label: radio.attr("aria-label").unwrap_or(&value).to_string(),
                value,
            }
        })
        .collect();
    let value = radios
        .iter()
        .find(|radio| has_attr(radio, "checked"))
        .and_then(|radio| radio.attr("value"))
        .map(str::to_string);
    let visual = visual_props(context, style);
    let node = PenNode::RadioGroup(RadioGroupNode {
        base: base(context, "radio_group", style),
        width: visual.width,
        height: visual.height,
        limits: visual.limits,
        value,
        options: Some(options),
        fill: visual.fill,
        stroke: visual.stroke,
        effects: visual.effects,
        corner_radius: visual.corner_radius,
        ..Default::default()
    });
    Some(Some(finish(context, node)))
}

struct VisualProps {
    width: Option<SizingBehavior>,
    height: Option<SizingBehavior>,
    limits: SizeLimits,
    fill: Option<Vec<PenFill>>,
    stroke: Option<PenStroke>,
    effects: Option<Vec<PenEffect>>,
    corner_radius: Option<CornerRadius>,
}

fn visual_props(context: &mut MapCtx<'_>, style: &ComputedStyle) -> VisualProps {
    let container = container_props_from(style, context);
    VisualProps {
        width: container.width,
        height: container.height,
        limits: container.limits,
        fill: container.fill,
        stroke: container.stroke,
        effects: container.effects,
        corner_radius: container.corner_radius,
    }
}

/// Every replaced element ends here, so the in-flow outcome `apply_base_style`
/// produces is parked on the context for `map_element::finish_leaf` to drain.
fn base(context: &mut MapCtx<'_>, name: &str, style: &ComputedStyle) -> PenNodeBase {
    let mut base = PenNodeBase {
        id: context.generate_id(),
        name: Some(name.to_string()),
        ..Default::default()
    };
    crate::mapper::apply_base_style(&mut base, style, context);
    base
}

fn finish(context: &mut MapCtx<'_>, node: PenNode) -> PenNode {
    context.node_count += 1;
    node
}

fn map_image(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    element: &DomElement,
    style: &ComputedStyle,
) -> PenNode {
    let source = crate::srcset::resolve_image_source(context, path, element);
    let visual = visual_props(context, style);
    let object_fit = match style.get("object-fit") {
        Some("cover") => Some(ImageFitMode::Crop),
        Some("contain") => Some(ImageFitMode::Fit),
        Some("fill") => Some(ImageFitMode::Fill),
        Some("scale-down") => {
            context.warn_once(ImportWarning::ObjectFitScaleDown);
            Some(ImageFitMode::Fit)
        }
        Some("none") => {
            context.warn_once(ImportWarning::ObjectFitNoneIgnored);
            None
        }
        _ => None,
    };
    if style
        .get("object-position")
        .is_some_and(|value| !is_default_object_position(value))
    {
        context.warn_once(ImportWarning::ObjectPositionIgnored);
    }
    let mut image_base = base(context, "img", style);
    image_base.blend_mode = image_blend_mode(context, style);
    // `aspect-ratio` applies to replaced elements too, and only after the
    // legacy `width` / `height` attributes have supplied their fallback.
    let mut width = visual.width.or_else(|| numeric_attr(element, "width"));
    let mut height = visual.height.or_else(|| numeric_attr(element, "height"));
    crate::mapper::apply_aspect_ratio_axes(&mut width, &mut height, &visual.limits, style, context);
    let node = PenNode::Image(ImageNode {
        base: image_base,
        src: ImageSrc::from(source),
        object_fit,
        width,
        height,
        corner_radius: visual.corner_radius,
        effects: visual.effects,
        exposure: None,
        contrast: None,
        saturation: None,
        temperature: None,
        tint: None,
        highlights: None,
        shadows: None,
        image_prompt: None,
        image_search_query: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        limits: visual.limits,
    });
    finish(context, node)
}

fn is_default_object_position(value: &str) -> bool {
    let parts: Vec<_> = value
        .split_ascii_whitespace()
        .map(str::to_ascii_lowercase)
        .collect();
    match parts.as_slice() {
        [value] => matches!(value.as_str(), "center" | "50%"),
        [first, second] => {
            matches!(first.as_str(), "center" | "50%")
                && matches!(second.as_str(), "center" | "50%")
        }
        _ => false,
    }
}

fn image_blend_mode(context: &mut MapCtx<'_>, style: &ComputedStyle) -> Option<BlendMode> {
    let value = style.get("mix-blend-mode")?;
    match crate::mapper::map_blend_mode(value) {
        Some(BlendMode::Normal) => None,
        Some(mode) => Some(mode),
        None => {
            context.warn_once(ImportWarning::ImageMixBlendModeUnsupported);
            None
        }
    }
}

fn map_svg(context: &mut MapCtx<'_>, element: &DomElement, style: &ComputedStyle) -> PenNode {
    context.warn_once(ImportWarning::InlineSvgPlaceholder);
    let source = serialize_element(element);
    let src = format!("data:image/svg+xml;base64,{}", STANDARD.encode(source));
    let visual = visual_props(context, style);
    let mut image_base = base(context, "svg", style);
    image_base.blend_mode = image_blend_mode(context, style);
    let node = PenNode::Image(ImageNode {
        base: image_base,
        src: ImageSrc::from(src),
        object_fit: None,
        width: visual.width.or_else(|| numeric_attr(element, "width")),
        height: visual.height.or_else(|| numeric_attr(element, "height")),
        corner_radius: visual.corner_radius,
        effects: visual.effects,
        exposure: None,
        contrast: None,
        saturation: None,
        temperature: None,
        tint: None,
        highlights: None,
        shadows: None,
        image_prompt: None,
        image_search_query: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        limits: visual.limits,
    });
    finish(context, node)
}

fn map_input(context: &mut MapCtx<'_>, element: &DomElement, style: &ComputedStyle) -> PenNode {
    let input_type = element.attr("type").unwrap_or("text").to_ascii_lowercase();
    let visual = visual_props(context, style);
    let node = match input_type.as_str() {
        "text" | "email" | "password" | "search" | "url" | "tel" => {
            PenNode::TextInput(TextInputNode {
                base: base(context, "input", style),
                width: visual.width,
                height: visual.height,
                limits: visual.limits,
                placeholder: element.attr("placeholder").map(str::to_string),
                value: element.attr("value").map(str::to_string),
                fill: visual.fill,
                stroke: visual.stroke,
                effects: visual.effects,
                corner_radius: visual.corner_radius,
                ..Default::default()
            })
        }
        "checkbox" => PenNode::Checkbox(CheckboxNode {
            base: base(context, "checkbox", style),
            width: visual.width,
            height: visual.height,
            limits: visual.limits,
            checked: Some(BoolOrExpression::Bool(has_attr(element, "checked"))),
            label: None,
            fill: visual.fill,
            stroke: visual.stroke,
            effects: visual.effects,
            corner_radius: visual.corner_radius,
            ..Default::default()
        }),
        "range" => PenNode::Slider(SliderNode {
            base: base(context, "slider", style),
            width: visual.width,
            height: visual.height,
            limits: visual.limits,
            min: float_attr(element, "min"),
            max: float_attr(element, "max"),
            step: float_attr(element, "step"),
            value: float_attr(element, "value").map(NumberOrExpression::Number),
            fill: visual.fill,
            stroke: visual.stroke,
            effects: visual.effects,
            corner_radius: visual.corner_radius,
            ..Default::default()
        }),
        "radio" => PenNode::RadioGroup(RadioGroupNode {
            base: base(context, "radio_group", style),
            width: visual.width,
            height: visual.height,
            limits: visual.limits,
            value: has_attr(element, "checked")
                .then(|| element.attr("value").unwrap_or("on").to_string()),
            options: Some(vec![SelectOption {
                value: element.attr("value").unwrap_or("on").to_string(),
                label: element.attr("aria-label").unwrap_or("on").to_string(),
            }]),
            fill: visual.fill,
            stroke: visual.stroke,
            effects: visual.effects,
            corner_radius: visual.corner_radius,
            ..Default::default()
        }),
        _ => {
            context.warn_once(ImportWarning::InputTypeFallback);
            PenNode::TextInput(TextInputNode {
                base: base(context, "input", style),
                width: visual.width,
                height: visual.height,
                limits: visual.limits,
                placeholder: element.attr("placeholder").map(str::to_string),
                value: element.attr("value").map(str::to_string),
                fill: visual.fill,
                stroke: visual.stroke,
                effects: visual.effects,
                corner_radius: visual.corner_radius,
                ..Default::default()
            })
        }
    };
    finish(context, node)
}

fn map_text_area(context: &mut MapCtx<'_>, element: &DomElement, style: &ComputedStyle) -> PenNode {
    let visual = visual_props(context, style);
    let node = PenNode::TextArea(TextAreaNode {
        base: base(context, "textarea", style),
        width: visual.width,
        height: visual.height,
        limits: visual.limits,
        placeholder: element.attr("placeholder").map(str::to_string),
        value: Some(element_text(element)),
        fill: visual.fill,
        stroke: visual.stroke,
        effects: visual.effects,
        corner_radius: visual.corner_radius,
        ..Default::default()
    });
    finish(context, node)
}

fn map_select(context: &mut MapCtx<'_>, element: &DomElement, style: &ComputedStyle) -> PenNode {
    let visual = visual_props(context, style);
    let options: Vec<_> = element
        .children
        .iter()
        .filter_map(|child| match child {
            DomNode::Element(option) if option.tag == "option" => Some(option),
            _ => None,
        })
        .collect();
    let value = options
        .iter()
        .find(|option| has_attr(option, "selected"))
        .map(|option| {
            option
                .attr("value")
                .unwrap_or(&element_text(option))
                .to_string()
        });
    let options = options
        .into_iter()
        .map(|option| {
            let label = element_text(option);
            SelectOption {
                value: option.attr("value").unwrap_or(&label).to_string(),
                label,
            }
        })
        .collect();
    let node = PenNode::Select(SelectNode {
        base: base(context, "select", style),
        width: visual.width,
        height: visual.height,
        limits: visual.limits,
        placeholder: element.attr("placeholder").map(str::to_string),
        value,
        options: Some(options),
        fill: visual.fill,
        stroke: visual.stroke,
        effects: visual.effects,
        corner_radius: visual.corner_radius,
        ..Default::default()
    });
    finish(context, node)
}

fn map_progress(context: &mut MapCtx<'_>, element: &DomElement, style: &ComputedStyle) -> PenNode {
    let visual = visual_props(context, style);
    let node = PenNode::Progress(ProgressNode {
        base: base(context, "progress", style),
        width: visual.width,
        height: visual.height,
        limits: visual.limits,
        value: float_attr(element, "value").map(NumberOrExpression::Number),
        max: float_attr(element, "max"),
        fill: visual.fill,
        stroke: visual.stroke,
        effects: visual.effects,
        corner_radius: visual.corner_radius,
        ..Default::default()
    });
    finish(context, node)
}

fn map_horizontal_rule(context: &mut MapCtx<'_>, style: &ComputedStyle) -> PenNode {
    let mut container = container_props_from(style, context);
    container.width = container
        .width
        .or(Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)));
    container.height = container.height.or(Some(SizingBehavior::Number(1.0)));
    container.fill = container.fill.or_else(|| Some(vec![solid_fill("#e0e0e0")]));
    let node = PenNode::Rectangle(RectangleNode {
        base: base(context, "hr", style),
        container,
        children: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    });
    finish(context, node)
}

fn map_placeholder(
    context: &mut MapCtx<'_>,
    element: &DomElement,
    style: &ComputedStyle,
) -> PenNode {
    context.warn_once(ImportWarning::ElementPlaceholder {
        tag: element.tag.clone(),
    });
    let mut container = container_props_from(style, context);
    container.width = container.width.or_else(|| numeric_attr(element, "width"));
    container.height = container.height.or_else(|| numeric_attr(element, "height"));
    if container.fill.is_none() {
        container.fill = Some(vec![solid_fill("#f0f0f0")]);
    }
    let node = PenNode::Frame(FrameNode {
        base: base(context, &element.tag, style),
        container,
        children: Some(Vec::new()),
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        screen: None,
        breakpoint: None,
    });
    finish(context, node)
}

fn numeric_attr(element: &DomElement, name: &str) -> Option<SizingBehavior> {
    float_attr(element, name).map(SizingBehavior::Number)
}

fn float_attr(element: &DomElement, name: &str) -> Option<f64> {
    element.attr(name)?.parse().ok()
}

fn has_attr(element: &DomElement, name: &str) -> bool {
    element.attrs.iter().any(|(attr, _)| attr == name)
}

fn element_text(element: &DomElement) -> String {
    element
        .children
        .iter()
        .map(|child| match child {
            DomNode::Text(text) => text.clone(),
            DomNode::Element(child) => element_text(child),
        })
        .collect()
}

fn serialize_element(element: &DomElement) -> String {
    let mut source = format!("<{}", element.tag);
    for (name, value) in &element.attrs {
        source.push(' ');
        source.push_str(name);
        source.push_str("=\"");
        source.push_str(&escape_xml(value));
        source.push('"');
    }
    source.push('>');
    for child in &element.children {
        match child {
            DomNode::Text(text) => source.push_str(&escape_xml(text)),
            DomNode::Element(child) => source.push_str(&serialize_element(child)),
        }
    }
    source.push_str("</");
    source.push_str(&element.tag);
    source.push('>');
    source
}

// Intentionally local: op-html keeps a minimal op-* dep set, so this stays a
// private copy of the canonical `op_util::xml_escape::escape_html` (same four
// entities). Keep the two in sync.
fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn solid_fill(color: &str) -> PenFill {
    PenFill::Solid(SolidFillBody {
        color: color.to_string(),
        explain: None,
        opacity: None,
        blend_mode: None,
    })
}

#[cfg(test)]
#[path = "special_tests.rs"]
mod tests;
