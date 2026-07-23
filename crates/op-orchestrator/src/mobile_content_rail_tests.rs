use crate::mobile_content_rail::repair_mobile_content_rails_for_all_roots;
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::{json, Value};

fn insert(value: Value) -> VecDocSink {
    let tree: PenNode = serde_json::from_value(value).expect("fixture");
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    sink
}

fn find<'a>(node: &'a PenNode, id: &str) -> Option<&'a PenNode> {
    if node.id_str() == id {
        return Some(node);
    }
    node.children()?.iter().find_map(|child| find(child, id))
}

fn node_json(sink: &VecDocSink, id: &str) -> Value {
    let node = sink
        .state
        .active_children()
        .iter()
        .find_map(|root| find(root, id))
        .expect("node exists");
    serde_json::to_value(node).expect("node json")
}

fn text(id: &str, content: &str) -> Value {
    json!({"type":"text","id":id,"content":content,"width":"fit_content","height":"fit_content"})
}

#[test]
fn mixed_mobile_sections_converge_on_existing_content_rail() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":1285,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","width":"fill_container","height":44},
            {"type":"frame","id":"hero","width":"fill_container","layout":"vertical",
             "children":[{"type":"frame","id":"hero-card","cornerRadius":24,
                          "fill":[{"type":"solid","color":"#222222"}],"children":[text("hero-t","68°")]}]},
            {"type":"frame","id":"week","width":"fill_container","layout":"vertical","padding":[0,24],
             "children":[text("week-t","7-Day")]},
            {"type":"frame","id":"details","width":"fill_container","layout":"horizontal",
             "children":[{"type":"frame","id":"aqi","cornerRadius":16,
                          "fill":[{"type":"solid","color":"#181818"}],"children":[text("aqi-t","AQI")]}]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","width":"fill_container","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert_eq!(
        node_json(&sink, "hero")["padding"],
        json!([0.0, 24.0, 0.0, 24.0])
    );
    assert_eq!(
        node_json(&sink, "details")["padding"],
        json!([0.0, 24.0, 0.0, 24.0])
    );
    assert_eq!(node_json(&sink, "week")["padding"], json!([0.0, 24.0]));
    assert!(node_json(&sink, "status").get("padding").is_none());
    assert!(node_json(&sink, "nav").get("padding").is_none());
}

#[test]
fn clipped_scroller_repairs_header_and_leading_edge_only() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":390,"height":844,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"rail-section","width":"fill_container","layout":"vertical",
             "children":[
                {"type":"frame","id":"header","name":"Section Header","layout":"horizontal",
                 "children":[text("title","Popular")]},
                {"type":"frame","id":"viewport","layout":"horizontal","clipContent":true,
                 "children":[{"type":"frame","id":"cards","layout":"horizontal",
                              "children":[{"type":"frame","id":"card","cornerRadius":16,
                                           "children":[text("card-t","Kyoto")]}]}]}
             ]},
            {"type":"frame","id":"body","padding":[0,20],"children":[text("body-t","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert!(node_json(&sink, "rail-section").get("padding").is_none());
    assert_eq!(
        node_json(&sink, "header")["padding"],
        json!([0.0, 20.0, 0.0, 20.0])
    );
    assert_eq!(
        node_json(&sink, "viewport")["padding"],
        json!([0.0, 0.0, 0.0, 20.0])
    );
}

#[test]
fn full_bleed_surface_existing_root_rail_and_desktop_are_untouched() {
    let mut sink = insert(json!({
        "type":"frame","id":"mobile","width":375,"height":812,"layout":"vertical","padding":[0,16],
        "children":[
            {"type":"frame","id":"filled","width":"fill_container","cornerRadius":20,
             "fill":[{"type":"solid","color":"#111111"}],"children":[text("filled-t","Hero")]},
            {"type":"frame","id":"content","children":[text("content-t","Content")]}
        ]
    }));
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert!(
        sink.applied.is_empty(),
        "root-owned rail must not be stacked"
    );

    let mut desktop = insert(json!({
        "type":"frame","id":"desktop","width":1440,"height":900,"layout":"vertical",
        "children":[
            {"type":"frame","id":"a","children":[text("a-t","A")]},
            {"type":"frame","id":"b","children":[text("b-t","B")]}
        ]
    }));
    repair_mobile_content_rails_for_all_roots(&mut desktop);
    assert!(desktop.applied.is_empty(), "desktop roots are out of scope");

    let mut token_rail = insert(json!({
        "type":"frame","id":"token-root","width":375,"height":812,"layout":"vertical",
        "padding":"$spacing-page",
        "children":[
            {"type":"frame","id":"token-a","children":[text("token-a-t","A")]},
            {"type":"frame","id":"token-b","children":[text("token-b-t","B")]}
        ]
    }));
    repair_mobile_content_rails_for_all_roots(&mut token_rail);
    assert!(
        token_rail.applied.is_empty(),
        "variable-owned root padding must never be overwritten"
    );
}

#[test]
fn repair_is_idempotent_and_preserves_asymmetric_authored_padding() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"plain","children":[text("plain-t","Plain")]},
            {"type":"frame","id":"authored","padding":[4,8,6,24],"children":[text("auth-t","Authored")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert_eq!(
        node_json(&sink, "plain")["padding"],
        json!([0.0, 24.0, 0.0, 24.0])
    );
    assert_eq!(
        node_json(&sink, "authored")["padding"],
        json!([4.0, 8.0, 6.0, 24.0])
    );

    sink.applied.clear();
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert!(
        sink.applied.is_empty(),
        "a second repair pass must be a no-op"
    );
}

#[test]
fn mixed_bottom_nav_shell_is_demoted_before_content_rail_repair() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":1285,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"forecast","padding":[0,24],"children":[text("forecast-t","Forecast")]},
            {"type":"frame","id":"mixed","name":"Bottom Navigation Bar","role":"bottom-tab-bar",
             "layout":"vertical","fill":[{"type":"solid","color":"#151515"}],"cornerRadius":20,
             "children":[
                {"type":"frame","id":"alert","children":[text("alert-t","Flood warning")]},
                {"type":"frame","id":"metrics","children":[text("metrics-t","Humidity 88%")]},
                {"type":"frame","id":"real-nav","name":"Bottom Tab Bar","role":"bottom-tab-bar",
                 "layout":"horizontal","height":72,"children":[]}
             ]}
        ]
    }));

    crate::cleanup::repair_mobile_structural_chrome_for_all_roots(&mut sink);
    repair_mobile_content_rails_for_all_roots(&mut sink);

    let root = sink
        .state
        .active_children()
        .iter()
        .find(|node| node.id_str() == "root")
        .expect("root");
    assert_eq!(
        root.children()
            .expect("root children")
            .iter()
            .map(PenNodeExt::id_str)
            .collect::<Vec<_>>(),
        vec!["status", "forecast", "mixed", "real-nav"]
    );
    let mixed = node_json(&sink, "mixed");
    assert_eq!(mixed["name"], "App Content");
    assert!(mixed.get("role").is_none());
    assert!(mixed.get("fill").is_none());
    assert_eq!(mixed["padding"], json!([0.0, 24.0, 0.0, 24.0]));
    assert!(
        node_json(&sink, "real-nav").get("padding").is_some(),
        "real nav keeps its own normalized internal chrome padding"
    );
}
