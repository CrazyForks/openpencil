use super::spacing::detect_edge_section_padding;
use crate::issue::{FixProperty, IssueCategory, IssueSeverity};
use jian_ops_schema::node::PenNode;
use serde_json::json;

fn node(value: serde_json::Value) -> PenNode {
    serde_json::from_value(value).expect("fixture must deserialize as PenNode")
}

#[test]
fn flags_each_unpadded_content_section_with_default_rail() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "categories", "role": "categories",
                "children": [{"type": "text", "id": "t1", "content": "Categories"}]
            },
            {
                "type": "frame", "id": "list", "role": "list",
                "children": [{"type": "text", "id": "t2", "content": "Item"}]
            }
        ]
    }));

    let issues = detect_edge_section_padding(&root);

    assert_eq!(issues.len(), 2);
    assert_eq!(issues[0].node_id, "categories");
    assert_eq!(issues[1].node_id, "list");
    for issue in issues {
        assert_eq!(issue.category, IssueCategory::EdgeSectionPadding);
        assert_eq!(issue.property, FixProperty::Padding);
        assert_eq!(issue.severity, IssueSeverity::Warning);
        assert_eq!(issue.current_value, json!(null));
        assert_eq!(issue.suggested_value, json!([0, 24, 0, 24]));
    }
}

#[test]
fn padded_sibling_infers_rail_without_suppressing_unpadded_section() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "unpadded", "role": "categories",
                "children": [{"type": "text", "id": "t1", "content": "Categories"}]
            },
            {
                "type": "frame", "id": "established", "role": "list",
                "padding": [0, 20, 0, 20],
                "children": [{"type": "text", "id": "t2", "content": "Item"}]
            }
        ]
    }));

    let issues = detect_edge_section_padding(&root);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].node_id, "unpadded");
    assert_eq!(issues[0].suggested_value, json!([0, 20, 0, 20]));
}

#[test]
fn rail_inference_prefers_modal_value_then_value_nearest_policy_default() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "unpadded",
                "children": [{"type": "text", "id": "t1", "content": "Title"}]
            },
            {
                "type": "frame", "id": "rail-a", "padding": [0, 22, 0, 22],
                "children": [{"type": "text", "id": "t2", "content": "A"}]
            },
            {
                "type": "frame", "id": "rail-b", "padding": [0, 22, 0, 22],
                "children": [{"type": "text", "id": "t3", "content": "B"}]
            },
            {
                "type": "frame", "id": "rail-c", "padding": [0, 24, 0, 24],
                "children": [{"type": "text", "id": "t4", "content": "C"}]
            }
        ]
    }));

    let issues = detect_edge_section_padding(&root);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].suggested_value, json!([0, 22, 0, 22]));
}

#[test]
fn preserves_section_vertical_padding() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "section", "padding": [12, 0, 8, 0],
                "children": [{"type": "text", "id": "t1", "content": "Categories"}]
            },
            {
                "type": "frame", "id": "established", "padding": [0, 24, 0, 24],
                "children": [{"type": "text", "id": "t2", "content": "Item"}]
            }
        ]
    }));

    let issues = detect_edge_section_padding(&root);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].node_id, "section");
    assert_eq!(issues[0].current_value, json!([12, 0, 8, 0]));
    assert_eq!(issues[0].suggested_value, json!([12, 24, 8, 24]));
}

#[test]
fn skips_full_bleed_chrome_and_image_only_sections() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "status", "role": "status-bar",
                "children": [{"type": "text", "id": "time", "content": "9:41"}]
            },
            {
                "type": "frame", "id": "hero", "role": "hero",
                "children": [{"type": "text", "id": "welcome", "content": "Welcome"}]
            },
            {
                "type": "frame", "id": "media", "role": "gallery",
                "children": [{"type": "image", "id": "img", "src": "a.png"}]
            },
            {
                "type": "frame", "id": "nav", "role": "bottom-tab-bar",
                "children": [{"type": "icon_font", "id": "home", "iconFontName": "home"}]
            }
        ]
    }));

    assert!(detect_edge_section_padding(&root).is_empty());
}

#[test]
fn skips_surfaced_root_direct_card_without_a_safe_outer_owner() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "card",
                "fill": [{"type":"solid","color":"#111111"}],
                "stroke": {"thickness":1,"fill":[{"type":"solid","color":"#333333"}]},
                "children": [{"type": "text", "id": "card-title", "content": "Hero"}]
            },
            {
                "type": "frame", "id": "established", "padding": [0,24],
                "children": [{"type": "text", "id": "body", "content": "Body"}]
            }
        ]
    }));

    assert!(detect_edge_section_padding(&root).is_empty());
}

#[test]
fn skips_non_mobile_root_and_non_content_section() {
    let non_mobile = node(json!({
        "type": "frame", "id": "card",
        "width": 393, "height": 200, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "s1",
                "children": [{"type": "text", "id": "t1", "content": "Title"}]
            },
            {
                "type": "frame", "id": "s2",
                "children": [{"type": "text", "id": "t2", "content": "Body"}]
            }
        ]
    }));
    assert!(detect_edge_section_padding(&non_mobile).is_empty());

    let no_content = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "s1",
                "children": [{"type": "rectangle", "id": "r1"}]
            },
            {
                "type": "frame", "id": "s2",
                "children": [{"type": "rectangle", "id": "r2"}]
            }
        ]
    }));
    assert!(detect_edge_section_padding(&no_content).is_empty());
}

#[test]
fn scroller_section_targets_transparent_header_only() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 375, "height": 812, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "hourly", "layout": "vertical",
                "children": [
                    {
                        "type": "frame", "id": "hourly-header", "role": "navbar",
                        "layout": "horizontal",
                        "children": [{"type": "text", "id": "title", "content": "Hourly"}]
                    },
                    {
                        "type": "frame", "id": "hourly-scroll",
                        "layout": "horizontal", "clipContent": true,
                        "children": [{
                            "type": "frame", "id": "hour-card",
                            "children": [{"type": "text", "id": "hour", "content": "Now"}]
                        }]
                    }
                ]
            },
            {
                "type": "frame", "id": "forecast", "padding": [0, 24, 0, 24],
                "children": [{"type": "text", "id": "forecast-title", "content": "7-Day"}]
            }
        ]
    }));

    let issues = detect_edge_section_padding(&root);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].node_id, "hourly-header");
    assert_eq!(issues[0].suggested_value, json!([0, 24, 0, 24]));
    assert!(issues[0]
        .reason
        .contains("without changing clipped scroll geometry"));
}

#[test]
fn scroller_section_skips_filled_header_and_scroller_geometry() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 375, "height": 812, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "hourly", "layout": "vertical",
                "children": [
                    {
                        "type": "frame", "id": "filled-header",
                        "fill": [{"type": "solid", "color": "#111111"}],
                        "children": [{"type": "text", "id": "title", "content": "Hourly"}]
                    },
                    {
                        "type": "frame", "id": "hourly-scroll",
                        "layout": "horizontal", "clipContent": true,
                        "children": [{"type": "text", "id": "hour", "content": "Now"}]
                    }
                ]
            },
            {
                "type": "frame", "id": "forecast", "padding": [0, 24, 0, 24],
                "children": [{"type": "text", "id": "forecast-title", "content": "7-Day"}]
            }
        ]
    }));

    assert!(detect_edge_section_padding(&root).is_empty());
}

#[test]
fn ignores_padding_expression_instead_of_clobbering_design_token() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 375, "height": 812, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "token-section", "padding": "$space-page",
                "children": [{"type": "text", "id": "title", "content": "Title"}]
            },
            {
                "type": "frame", "id": "forecast", "padding": [0, 24, 0, 24],
                "children": [{"type": "text", "id": "forecast-title", "content": "7-Day"}]
            }
        ]
    }));

    assert!(detect_edge_section_padding(&root).is_empty());
}
