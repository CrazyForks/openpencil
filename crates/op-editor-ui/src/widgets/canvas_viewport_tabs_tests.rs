use super::widget_children_to_paint;
use crate::layout_scene::{NodeKind, SceneNode, SceneWidget, SceneWidgetOption};

fn tabs_node(value: Option<&str>) -> SceneNode {
    let mut node = SceneNode::leaf("tabs", NodeKind::Frame);
    node.widget = Some(SceneWidget {
        kind: "tabs".into(),
        value_str: value.map(str::to_owned),
        options: vec![
            SceneWidgetOption {
                value: "overview".into(),
                label: "Overview".into(),
            },
            SceneWidgetOption {
                value: "details".into(),
                label: "Details".into(),
            },
        ],
        ..Default::default()
    });
    node.children = vec![
        SceneNode::leaf("overview-panel", NodeKind::Frame),
        SceneNode::leaf("details-panel", NodeKind::Frame),
    ];
    node
}

#[test]
fn tabs_paint_only_the_authored_active_panel() {
    let node = tabs_node(Some("details"));
    let visible = widget_children_to_paint(&node);
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id, "details-panel");
}

#[test]
fn tabs_live_value_switches_panel_and_invalid_value_falls_back_first() {
    let mut node = tabs_node(Some("missing"));
    assert_eq!(widget_children_to_paint(&node)[0].id, "overview-panel");

    node.widget.as_mut().unwrap().value_str = Some("details".into());
    assert_eq!(widget_children_to_paint(&node)[0].id, "details-panel");
}
