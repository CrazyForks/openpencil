use crate::mobile_content_rail::{
    repair_mobile_content_rails, repair_mobile_content_rails_for_all_roots,
};
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
            {"type":"frame","id":"nav","role":"bottom-navigation-bar","height":72}
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
fn clipped_scroller_moves_duplicate_inner_leading_rail_to_viewport() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"hourly","width":"fill_container","layout":"vertical",
             "children":[
                {"type":"frame","id":"header","name":"Header Row","layout":"horizontal",
                 "children":[text("title","Hourly Forecast")]},
                {"type":"frame","id":"viewport","width":"fill_container","layout":"horizontal",
                 "clipContent":true,"children":[
                    {"type":"frame","id":"rail","width":"fit_content","layout":"horizontal",
                     "gap":8,"padding":[0,24],"children":[
                        {"type":"frame","id":"now","width":58,"cornerRadius":12,
                         "children":[text("now-t","NOW")]},
                        {"type":"frame","id":"later","width":58,"cornerRadius":12,
                         "children":[text("later-t","13:00")]}
                     ]}
                 ]}
             ]},
            {"type":"frame","id":"body","padding":[0,24],"children":[text("body-t","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert_eq!(
        node_json(&sink, "viewport")["padding"],
        json!([0.0, 0.0, 0.0, 24.0])
    );
    assert_eq!(
        node_json(&sink, "rail")["padding"],
        json!([0.0, 24.0, 0.0, 0.0]),
        "the viewport owns the leading rail while the lane keeps its end spacer"
    );

    sink.applied.clear();
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert!(
        sink.applied.is_empty(),
        "the single-owner scroller rail must be idempotent"
    );
}

#[test]
fn existing_double_scroller_rail_collapses_to_one_leading_owner() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"viewport","width":"fill_container","layout":"horizontal",
             "padding":[0,0,0,24],"clipContent":true,"children":[
                {"type":"frame","id":"rail","width":"fit_content","layout":"horizontal",
                 "padding":[0,24],"children":[
                    {"type":"frame","id":"a","width":58,"children":[text("a-t","A")]},
                    {"type":"frame","id":"b","width":58,"children":[text("b-t","B")]}
                 ]}
             ]},
            {"type":"frame","id":"body","padding":[0,24],"children":[text("body-t","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert_eq!(
        node_json(&sink, "viewport")["padding"],
        json!([0.0, 0.0, 0.0, 24.0])
    );
    assert_eq!(
        node_json(&sink, "rail")["padding"],
        json!([0.0, 24.0, 0.0, 0.0])
    );
}

#[test]
fn authored_small_viewport_inset_does_not_consume_the_inner_content_rail() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"viewport","width":"fill_container","layout":"horizontal",
             "padding":[0,0,0,8],"clipContent":true,"children":[
                {"type":"frame","id":"rail","width":"fit_content","layout":"horizontal",
                 "padding":[0,24],"children":[
                    {"type":"frame","id":"a","width":58,"children":[text("a-t","A")]},
                    {"type":"frame","id":"b","width":58,"children":[text("b-t","B")]}
                 ]}
             ]},
            {"type":"frame","id":"body","padding":[0,24],"children":[text("body-t","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert_eq!(
        node_json(&sink, "viewport")["padding"],
        json!([0.0, 0.0, 0.0, 8.0])
    );
    assert_eq!(
        node_json(&sink, "rail")["padding"],
        json!([0.0, 24.0]),
        "a non-canonical authored viewport inset is not proof of duplicate rail ownership"
    );
}

#[test]
fn clipped_scroller_does_not_rewrite_a_surfaced_child_card() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"viewport","width":"fill_container","layout":"horizontal",
             "clipContent":true,"children":[
                {"type":"frame","id":"card","width":"fit_content","layout":"horizontal",
                 "padding":[0,24],"cornerRadius":16,
                 "fill":[{"type":"solid","color":"#181818"}],"children":[
                    {"type":"text","id":"card-title","content":"Title"},
                    {"type":"text","id":"card-meta","content":"Meta"}
                 ]}
             ]},
            {"type":"frame","id":"body","padding":[0,24],"children":[text("body-t","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert_eq!(
        node_json(&sink, "viewport")["padding"],
        json!([0.0, 0.0, 0.0, 24.0])
    );
    assert_eq!(node_json(&sink, "card")["padding"], json!([0.0, 24.0]));
}

#[test]
fn root_direct_scroller_gets_leading_rail_without_trailing_inset() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"viewport","width":"fill_container","layout":"horizontal",
             "clipContent":true,"children":[
                {"type":"frame","id":"row","width":"fit_content","layout":"horizontal",
                 "children":[
                    {"type":"image","id":"image-a","width":120,"height":90,"src":"a.png"},
                    {"type":"image","id":"image-b","width":120,"height":90,"src":"b.png"}
                 ]}
             ]},
            {"type":"frame","id":"body","padding":[0,24],"children":[text("body-t","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert_eq!(
        node_json(&sink, "viewport")["padding"],
        json!([0.0, 0.0, 0.0, 24.0])
    );
}

#[test]
fn trailing_only_scroller_padding_still_gets_a_leading_rail() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"viewport","width":"fill_container","layout":"horizontal",
             "padding":[0,12,0,0],"clipContent":true,"children":[
                {"type":"frame","id":"row","width":"fit_content","layout":"horizontal",
                 "children":[
                    {"type":"image","id":"image-a","width":120,"height":90,"src":"a.png"},
                    {"type":"image","id":"image-b","width":120,"height":90,"src":"b.png"}
                 ]}
             ]},
            {"type":"frame","id":"body","padding":[0,24],"children":[text("body-t","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert_eq!(
        node_json(&sink, "viewport")["padding"],
        json!([0.0, 12.0, 0.0, 24.0])
    );
}

#[test]
fn deeper_scroller_is_not_closed_by_symmetric_ancestor_padding() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"section","width":"fill_container","layout":"vertical",
             "children":[
                {"type":"frame","id":"body-wrapper","width":"fill_container","layout":"vertical",
                 "children":[
                    {"type":"frame","id":"viewport","width":"fill_container","layout":"horizontal",
                     "clipContent":true,"children":[text("item","Item")]}
                 ]}
             ]},
            {"type":"frame","id":"body","padding":[0,24],"children":[text("body-t","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert!(node_json(&sink, "section").get("padding").is_none());
    assert!(node_json(&sink, "viewport").get("padding").is_none());
}

#[test]
fn progress_meter_inside_surfaced_card_does_not_suppress_outer_section_rail() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"forecast","width":"fill_container","layout":"vertical",
             "padding":[0,24],"children":[text("forecast-t","7-Day Forecast")]},
            {"type":"frame","id":"metrics","width":"fill_container","layout":"vertical",
             "children":[
                {"type":"frame","id":"metrics-row","width":"fill_container","layout":"horizontal",
                 "gap":12,"children":[
                    {"type":"frame","id":"sun-card","width":"fill_container","layout":"vertical",
                     "padding":14,"cornerRadius":16,
                     "fill":[{"type":"solid","color":"#181818"}],
                     "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#333333"}]},
                     "children":[text("sunset","Sunset 8:32 PM")]},
                    {"type":"frame","id":"environment-card","width":"fill_container",
                     "layout":"vertical","padding":14,"cornerRadius":16,
                     "fill":[{"type":"solid","color":"#181818"}],
                     "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#333333"}]},
                     "children":[
                        text("humidity","Humidity 78%"),
                        {"type":"frame","id":"progress-meter","width":"fill_container","height":6,
                         "layout":"horizontal","clipContent":true,
                         "fill":[{"type":"solid","color":"#222222"}],"children":[
                            {"type":"frame","id":"progress-fill","width":110,
                             "height":"fill_container",
                             "fill":[{"type":"solid","color":"#C4F82A"}],"children":[]}
                         ]}
                     ]}
                 ]}
             ]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert_eq!(
        node_json(&sink, "metrics")["padding"],
        json!([0.0, 24.0, 0.0, 24.0])
    );
    assert!(
        node_json(&sink, "progress-meter").get("padding").is_none(),
        "a clipped meter inside a surfaced card is not a page-level scroller"
    );

    sink.applied.clear();
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert!(
        sink.applied.is_empty(),
        "the repaired outer section rail must be idempotent"
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
fn full_bleed_role_and_media_overlay_wrapper_are_untouched() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"hero-role","role":"hero","width":"fill_container",
             "children":[text("hero-role-t","Welcome")]},
            {"type":"frame","id":"media-overlay","width":"fill_container","layout":"none",
             "children":[
                {"type":"image","id":"cover","width":"fill_container","height":240,"src":"cover.png"},
                text("overlay","Featured story")
             ]},
            {"type":"frame","id":"body","padding":[0,24],"children":[text("body-t","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert!(node_json(&sink, "hero-role").get("padding").is_none());
    assert!(node_json(&sink, "media-overlay").get("padding").is_none());
    assert!(node_json(&sink, "nav").get("padding").is_none());
}

#[test]
fn edge_spanning_rounded_card_gets_a_transparent_rail_wrapper() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"direct-card","name":"Weather Card","role":"card",
             "width":"fill_container","layout":"vertical","padding":16,"cornerRadius":20,
             "fill":[{"type":"solid","color":"#181818"}],"children":[
                {"type":"image","id":"cover","width":"fill_container","height":160,"src":"cover.png"},
                text("card-t","68°")
             ]},
            {"type":"frame","id":"body","padding":[0,24],"children":[text("body-t","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    let root = sink.state.active_children().first().expect("root");
    assert_eq!(
        root.children()
            .expect("root children")
            .iter()
            .map(PenNodeExt::id_str)
            .collect::<Vec<_>>(),
        vec!["status", "direct-card__content_rail", "body", "nav"]
    );
    let wrapper = node_json(&sink, "direct-card__content_rail");
    assert_eq!(wrapper["padding"], json!([0.0, 24.0, 0.0, 24.0]));
    assert_eq!(
        wrapper["children"][0]["id"], "direct-card",
        "the surfaced card moves under the rail owner"
    );
    assert_eq!(node_json(&sink, "direct-card")["padding"], json!(16.0));

    sink.applied.clear();
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert!(sink.applied.is_empty(), "surface wrapping is idempotent");
}

#[test]
fn absolute_surfaced_card_is_not_reparented_into_a_flow_wrapper() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"absolute-card","name":"Floating Card","role":"card",
             "x":0,"y":120,"width":"fill_container","height":180,"cornerRadius":20,
             "fill":[{"type":"solid","color":"#181818"}],"children":[text("card-t","Floating")]},
            {"type":"frame","id":"body","padding":[0,24],"children":[text("body-t","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    let root = sink.state.active_children().first().expect("root");
    assert_eq!(
        root.children()
            .expect("root children")
            .iter()
            .map(PenNodeExt::id_str)
            .collect::<Vec<_>>(),
        vec!["status", "absolute-card", "body", "nav"]
    );
    assert!(node_json(&sink, "absolute-card").get("padding").is_none());
}

#[test]
fn empty_rounded_decorative_surface_remains_full_bleed() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"decorative-surface","width":"fill_container","height":180,
             "cornerRadius":24,"fill":[{"type":"solid","color":"#181818"}],"children":[]},
            {"type":"frame","id":"body","padding":[0,24],"children":[text("body-t","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    let root = sink.state.active_children().first().expect("root");
    assert_eq!(
        root.children()
            .expect("root children")
            .iter()
            .map(PenNodeExt::id_str)
            .collect::<Vec<_>>(),
        vec!["status", "decorative-surface", "body", "nav"]
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

#[test]
fn v4_like_empty_header_adopts_brand_and_action_and_wraps_loose_title() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":390,"height":844,"layout":"vertical",
        "gap":16,"children":[
            {"type":"frame","id":"status","role":"status-bar","width":"fill_container",
             "height":62,"layout":"none","children":[]},
            {"type":"frame","id":"header","name":"Header","width":"fill_container",
             "height":"fit_content","layout":"horizontal","gap":12,
             "padding":[12,24,4,24],"justifyContent":"space_between",
             "alignItems":"center","children":[]},
            {"type":"text","id":"brand","name":"Brand","content":"Nook",
             "width":"fit_content","height":"fit_content","fontSize":24},
            {"type":"frame","id":"search-rail","name":"Search row Content Rail",
             "width":"fill_container","height":"fit_content","layout":"vertical",
             "padding":[0,24,0,24],"children":[
                {"type":"frame","id":"search","name":"Search row","width":"fill_container",
                 "height":34,"layout":"horizontal","children":[
                    {"type":"icon_font","id":"search-icon","name":"Search icon",
                     "iconFontName":"search","width":18,"height":18}
                 ]}
             ]},
            {"type":"frame","id":"cart","name":"Cart button","width":40,"height":40,
             "layout":"horizontal","fill":[{"type":"solid","color":"#111318"}],
             "children":[
                {"type":"icon_font","id":"cart-icon","name":"Cart icon",
                 "iconFontName":"shopping-bag","width":20,"height":20}
             ]},
            {"type":"frame","id":"hero","name":"Hero banner","role":"hero",
             "width":"fill_container","height":220,"children":[text("hero-title","Featured")]},
            {"type":"text","id":"section-title","name":"Section title",
             "content":"Quick categories","width":"fit_content","height":"fit_content",
             "fontSize":18},
            {"type":"frame","id":"categories","name":"Category row",
             "width":"fill_container","layout":"horizontal","padding":[0,24],
             "children":[text("category","Home")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72,"children":[]}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert_eq!(
        node_json(&sink, "header")["children"]
            .as_array()
            .expect("header children")
            .iter()
            .map(|child| child["id"].as_str().expect("child id"))
            .collect::<Vec<_>>(),
        vec!["brand", "cart"]
    );
    let root = sink.state.active_children().first().expect("root");
    assert_eq!(
        root.children()
            .expect("root children")
            .iter()
            .map(PenNodeExt::id_str)
            .collect::<Vec<_>>(),
        vec![
            "status",
            "header",
            "search-rail",
            "hero",
            "section-title__content_rail",
            "categories",
            "nav"
        ]
    );
    assert_eq!(
        node_json(&sink, "section-title__content_rail")["padding"],
        json!([0.0, 24.0, 0.0, 24.0])
    );
    assert_eq!(
        node_json(&sink, "section-title__content_rail")["children"][0]["id"],
        "section-title"
    );
    assert_eq!(
        node_json(&sink, "search-rail")["padding"],
        json!([0.0, 24.0, 0.0, 24.0]),
        "the intervening search rail stays owned by the page, not the header"
    );

    sink.applied.clear();
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert!(
        sink.applied.is_empty(),
        "the normalized v4-like structure must be idempotent"
    );
}

#[test]
fn ambiguous_empty_header_region_is_not_reparented() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":390,"height":844,"layout":"vertical",
        "children":[
            {"type":"frame","id":"header","name":"Header","width":"fill_container",
             "height":"fit_content","layout":"horizontal","children":[]},
            {"type":"text","id":"brand","name":"Brand","content":"Nook",
             "width":"fit_content","height":"fit_content","fontSize":24},
            {"type":"frame","id":"cart","name":"Cart button","width":40,"height":40,
             "layout":"horizontal","children":[
                {"type":"icon_font","id":"cart-icon","iconFontName":"shopping-bag",
                 "width":20,"height":20}
             ]},
            {"type":"frame","id":"profile","name":"Profile button","width":40,"height":40,
             "layout":"horizontal","children":[
                {"type":"icon_font","id":"profile-icon","iconFontName":"user",
                 "width":20,"height":20}
             ]},
            {"type":"frame","id":"body","width":"fill_container","padding":[0,24],
             "children":[text("body-title","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72,"children":[]}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    assert!(
        sink.applied.is_empty(),
        "two actions make ownership ambiguous"
    );
    assert!(node_json(&sink, "header")["children"]
        .as_array()
        .expect("header children")
        .is_empty());
    let root = sink.state.active_children().first().expect("root");
    assert_eq!(
        root.children()
            .expect("root children")
            .iter()
            .map(PenNodeExt::id_str)
            .collect::<Vec<_>>(),
        vec!["header", "brand", "cart", "profile", "body", "nav"]
    );
}

#[test]
fn loose_leaf_rails_exclude_system_chrome_and_full_bleed_semantics() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":390,"height":844,"layout":"vertical",
        "children":[
            {"type":"icon_font","id":"wifi","name":"Wifi","role":"status-bar",
             "iconFontName":"wifi","width":18,"height":18},
            {"type":"text","id":"hero-copy","name":"Hero title","role":"hero",
             "content":"Edge to edge","fontSize":28},
            {"type":"text","id":"section-title","name":"Section title",
             "content":"Popular","fontSize":18},
            {"type":"icon_font","id":"accent","name":"Spark accent",
             "iconFontName":"sparkles","width":20,"height":20},
            {"type":"icon_font","id":"bottom-home","name":"Bottom home",
             "role":"bottom-nav","iconFontName":"home","width":24,"height":24}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    let root = sink.state.active_children().first().expect("root");
    assert_eq!(
        root.children()
            .expect("root children")
            .iter()
            .map(PenNodeExt::id_str)
            .collect::<Vec<_>>(),
        vec![
            "wifi",
            "hero-copy",
            "section-title__content_rail",
            "accent__content_rail",
            "bottom-home"
        ]
    );
    for wrapper_id in ["section-title__content_rail", "accent__content_rail"] {
        assert_eq!(
            node_json(&sink, wrapper_id)["padding"],
            json!([0.0, 24.0, 0.0, 24.0])
        );
    }
    assert_eq!(node_json(&sink, "wifi")["role"], "status-bar");
    assert_eq!(node_json(&sink, "hero-copy")["role"], "hero");
    assert_eq!(node_json(&sink, "bottom-home")["role"], "bottom-nav");

    sink.applied.clear();
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert!(sink.applied.is_empty(), "leaf wrapping must be idempotent");
}

#[test]
fn e2e_like_duplicate_app_content_shells_merge_in_script_order() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":390,"height":"fit_content",
        "minHeight":844,"layout":"vertical","gap":16,"children":[
            {"type":"frame","id":"root-header","name":"Header","width":"fill_container",
             "height":"fit_content","layout":"horizontal","padding":[0,24],
             "children":[text("root-brand","Mono Market")]},
            {"type":"frame","id":"content-a","name":"App Content",
             "width":"fill_container","height":"fit_content","layout":"vertical",
             "gap":32,"padding":[40,24,36,24],"children":[
                {"type":"frame","id":"header","name":"Header","role":"navbar",
                 "width":"fill_container","layout":"horizontal",
                 "children":[text("brand","Mono Market")]},
                {"type":"frame","id":"featured","name":"Featured Product",
                 "width":"fill_container","children":[text("product","Sweater")]}
             ]},
            {"type":"frame","id":"content-b","name":"app-content",
             "width":"fill_container","height":487,"layout":"vertical",
             "gap":32,"padding":[40,24,36,24],"children":[
                {"type":"frame","id":"benefits","name":"Benefits",
                 "children":[text("benefit","Original design")]},
                {"type":"frame","id":"cta","name":"CTA","role":"cta-section",
                 "children":[text("cta-label","Shop now")]},
                {"type":"frame","id":"footer","name":"Footer","role":"footer",
                 "children":[text("footer-copy","About us")]}
             ]}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);

    let root = sink.state.active_children().first().expect("root");
    assert_eq!(
        root.children()
            .expect("root children")
            .iter()
            .map(PenNodeExt::id_str)
            .collect::<Vec<_>>(),
        vec!["root-header", "content-a"]
    );
    assert_eq!(
        node_json(&sink, "content-a")["children"]
            .as_array()
            .expect("merged content")
            .iter()
            .map(|child| child["id"].as_str().expect("child id"))
            .collect::<Vec<_>>(),
        vec!["header", "featured", "benefits", "cta", "footer"]
    );
    assert_eq!(
        sink.applied.len(),
        1,
        "the whole merge lands as one atomic command"
    );
    assert!(matches!(
        &sink.applied[0],
        EditorCommand::Batch { commands } if commands.len() == 4
    ));

    sink.applied.clear();
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert!(
        sink.applied.is_empty(),
        "the real three-child minHeight shape must converge in its first pass"
    );
}

#[test]
fn fit_content_min_height_mobile_detection_keeps_width_and_height_bounds_strict() {
    let fixture = |root_id: &str, width: u32, min_height: u32| {
        json!({
            "type":"frame","id":root_id,"width":width,"height":"fit_content",
            "minHeight":min_height,"layout":"vertical","children":[
                {"type":"frame","id":format!("header-{root_id}"),"name":"Header",
                 "width":"fill_container","layout":"horizontal","padding":[0,24],
                 "children":[text(&format!("brand-{root_id}"),"Brand")]},
                {"type":"frame","id":format!("content-a-{root_id}"),"name":"App Content",
                 "width":"fill_container","layout":"vertical","padding":[0,24],
                 "children":[text(&format!("a-{root_id}"),"A")]},
                {"type":"frame","id":format!("content-b-{root_id}"),"name":"App Content",
                 "width":"fill_container","layout":"vertical","padding":[0,24],
                 "children":[text(&format!("b-{root_id}"),"B")]}
            ]
        })
    };

    for (label, root) in [
        ("desktop width", fixture("desktop", 768, 844)),
        ("short minHeight", fixture("short", 390, 500)),
    ] {
        let mut sink = insert(root);
        repair_mobile_content_rails_for_all_roots(&mut sink);
        assert!(
            sink.applied.is_empty(),
            "{label} must remain outside the mobile minHeight contract"
        );
        assert_eq!(
            sink.state.active_children()[0]
                .children()
                .expect("root children")
                .len(),
            3
        );
    }
}

#[test]
fn duplicate_page_shell_merge_declines_name_and_attribute_mismatches() {
    let mut different_name = insert(json!({
        "type":"frame","id":"root","width":390,"height":844,"layout":"vertical",
        "children":[
            {"type":"frame","id":"a","name":"App Content","width":"fill_container",
             "layout":"vertical","gap":24,"padding":[0,24],
             "children":[text("a-title","A")]},
            {"type":"frame","id":"b","name":"Main Content","width":"fill_container",
             "layout":"vertical","gap":24,"padding":[0,24],
             "children":[text("b-title","B")]}
        ]
    }));
    repair_mobile_content_rails_for_all_roots(&mut different_name);
    assert!(
        different_name.applied.is_empty(),
        "different semantic shell names never merge"
    );

    let mut different_padding = insert(json!({
        "type":"frame","id":"root","width":390,"height":844,"layout":"vertical",
        "children":[
            {"type":"frame","id":"a","name":"Page Content","width":"fill_container",
             "layout":"vertical","gap":24,"padding":[0,24],
             "children":[text("a-title","A")]},
            {"type":"frame","id":"b","name":"page_content","width":"fill_container",
             "layout":"vertical","gap":24,"padding":[0,20],
             "children":[text("b-title","B")]}
        ]
    }));
    repair_mobile_content_rails_for_all_roots(&mut different_padding);
    assert!(
        different_padding.applied.is_empty(),
        "different shell padding blocks the merge"
    );
}

#[test]
fn duplicate_page_shell_merge_is_idempotent() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":390,"height":844,"layout":"vertical",
        "children":[
            {"type":"frame","id":"a","name":"Content Root","width":"fill_container",
             "height":"fit_content","layout":"vertical","padding":[0,24],
             "children":[text("a-title","A")]},
            {"type":"frame","id":"b","name":"content-root","width":"fill_container",
             "height":120,"layout":"vertical","padding":[0,24,0,24],
             "children":[text("b-title","B")]}
        ]
    }));

    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert_eq!(
        node_json(&sink, "a")["children"].as_array().unwrap().len(),
        2
    );
    sink.applied.clear();
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert!(sink.applied.is_empty(), "the merged shell stays converged");
}

#[test]
fn independent_sections_and_multiscreen_roots_are_not_merged() {
    let mut sections = insert(json!({
        "type":"frame","id":"root","width":390,"height":844,"layout":"vertical",
        "children":[
            {"type":"frame","id":"benefits-a","name":"Benefits","width":"fill_container",
             "layout":"vertical","padding":[0,24],"children":[text("benefit-a","A")]},
            {"type":"frame","id":"benefits-b","name":"Benefits","width":"fill_container",
             "layout":"vertical","padding":[0,24],"children":[text("benefit-b","B")]}
        ]
    }));
    repair_mobile_content_rails_for_all_roots(&mut sections);
    assert!(
        sections.applied.is_empty(),
        "ordinary same-name sections are not page shells"
    );

    let screen = |root_id: &str, suffix: &str| {
        json!({
            "type":"frame","id":root_id,"width":390,"height":844,"layout":"vertical",
            "children":[
                {"type":"frame","id":format!("status-{suffix}"),"role":"status-bar",
                 "height":62,"children":[]},
                {"type":"frame","id":format!("content-a-{suffix}"),"name":"App Content",
                 "width":"fill_container","layout":"vertical","padding":[0,24],
                 "children":[text(&format!("a-{suffix}"),"A")]},
                {"type":"frame","id":format!("content-b-{suffix}"),"name":"App Content",
                 "width":"fill_container","layout":"vertical","padding":[0,24],
                 "children":[text(&format!("b-{suffix}"),"B")]}
            ]
        })
    };
    let mut screens = VecDocSink::new();
    screens.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![
            serde_json::from_value(screen("screen-a", "a")).expect("screen a"),
            serde_json::from_value(screen("screen-b", "b")).expect("screen b"),
        ],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    screens.applied.clear();
    repair_mobile_content_rails_for_all_roots(&mut screens);
    assert!(
        screens.applied.is_empty(),
        "whole-finalize shell merging is disabled for multi-screen documents"
    );
}

// -- Selected-frame append path: `root_id` nests under an existing top-level
// mobile screen rather than being a top-level active root itself. --

#[test]
fn appended_nested_section_gets_its_content_rail_repaired() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"existing","padding":[0,24],"children":[text("existing-t","Existing")]},
            {"type":"frame","id":"appended","width":"fill_container","layout":"horizontal",
             "clipContent":true,"children":[
                {"type":"frame","id":"appended-rail","width":"fit_content","layout":"horizontal",
                 "padding":[0,24],"children":[
                    {"type":"frame","id":"a","width":58,"children":[text("a-t","A")]},
                    {"type":"frame","id":"b","width":58,"children":[text("b-t","B")]}
                 ]}
             ]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    // `appended` is not a top-level active root — it is a direct child of
    // `root` — mirroring `inserted_root_ids` from a selected-frame append.
    repair_mobile_content_rails(&mut sink, "appended");

    assert_eq!(
        node_json(&sink, "appended")["padding"],
        json!([0.0, 0.0, 0.0, 24.0]),
        "the nested viewport must gain the same leading rail a top-level one would"
    );
    assert_eq!(
        node_json(&sink, "appended-rail")["padding"],
        json!([0.0, 24.0, 0.0, 0.0]),
        "the redundant inner rail collapses just like the top-level case"
    );
}

#[test]
fn appended_nested_section_inside_desktop_frame_is_untouched() {
    let mut sink = insert(json!({
        "type":"frame","id":"desktop","width":1440,"height":900,"layout":"vertical",
        "children":[
            {"type":"frame","id":"existing","children":[text("existing-t","Existing")]},
            {"type":"frame","id":"appended","width":"fill_container","layout":"horizontal",
             "clipContent":true,"children":[
                {"type":"frame","id":"appended-rail","width":"fit_content","layout":"horizontal",
                 "padding":[0,24],"children":[
                    {"type":"frame","id":"a","width":58,"children":[text("a-t","A")]},
                    {"type":"frame","id":"b","width":58,"children":[text("b-t","B")]}
                 ]}
             ]}
        ]
    }));

    repair_mobile_content_rails(&mut sink, "appended");

    assert!(
        sink.applied.is_empty(),
        "the enclosing top-level root is not a mobile screen, so the append is out of scope"
    );
}

#[test]
fn appended_section_repair_never_touches_a_sibling_double_rail_scroller() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"existing-viewport","width":"fill_container","layout":"horizontal",
             "padding":[0,0,0,24],"clipContent":true,"children":[
                {"type":"frame","id":"existing-rail","width":"fit_content","layout":"horizontal",
                 "padding":[0,24],"children":[
                    {"type":"frame","id":"x","width":58,"children":[text("x-t","X")]},
                    {"type":"frame","id":"y","width":58,"children":[text("y-t","Y")]}
                 ]}
             ]},
            {"type":"frame","id":"appended","width":"fill_container","layout":"horizontal",
             "clipContent":true,"children":[
                {"type":"frame","id":"appended-rail","width":"fit_content","layout":"horizontal",
                 "padding":[0,24],"children":[
                    {"type":"frame","id":"a","width":58,"children":[text("a-t","A")]},
                    {"type":"frame","id":"b","width":58,"children":[text("b-t","B")]}
                 ]}
             ]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    // `existing-viewport` already carries the exact same double-rail
    // duplicate the appended section has, but it was not part of this
    // append — the fix must be scoped to `appended`'s own subtree only.
    repair_mobile_content_rails(&mut sink, "appended");

    assert_eq!(
        node_json(&sink, "appended")["padding"],
        json!([0.0, 0.0, 0.0, 24.0])
    );
    assert_eq!(
        node_json(&sink, "appended-rail")["padding"],
        json!([0.0, 24.0, 0.0, 0.0])
    );
    assert_eq!(
        node_json(&sink, "existing-viewport")["padding"],
        json!([0.0, 0.0, 0.0, 24.0]),
        "sibling scroller outside the inserted subtree keeps its original padding"
    );
    assert_eq!(
        node_json(&sink, "existing-rail")["padding"],
        json!([0.0, 24.0]),
        "sibling scroller's inner rail is out of mutation scope and must be left alone"
    );
}

#[test]
fn appended_section_repair_is_idempotent() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"existing","padding":[0,24],"children":[text("existing-t","Existing")]},
            {"type":"frame","id":"appended","width":"fill_container","layout":"horizontal",
             "clipContent":true,"children":[
                {"type":"frame","id":"appended-rail","width":"fit_content","layout":"horizontal",
                 "padding":[0,24],"children":[
                    {"type":"frame","id":"a","width":58,"children":[text("a-t","A")]},
                    {"type":"frame","id":"b","width":58,"children":[text("b-t","B")]}
                 ]}
             ]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    repair_mobile_content_rails(&mut sink, "appended");
    sink.applied.clear();
    repair_mobile_content_rails(&mut sink, "appended");

    assert!(
        sink.applied.is_empty(),
        "a second repair pass over the same nested append must be a no-op"
    );
}

#[test]
fn appended_inner_lane_alone_does_not_get_its_rail_stripped() {
    let mut sink = insert(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {"type":"frame","id":"viewport","width":"fill_container","layout":"horizontal",
             "clipContent":true,"children":[
                {"type":"frame","id":"lane","width":"fit_content","layout":"horizontal",
                 "padding":[0,24],"children":[
                    {"type":"frame","id":"a","width":58,"children":[text("a-t","A")]},
                    {"type":"frame","id":"b","width":58,"children":[text("b-t","B")]}
                 ]}
             ]},
            {"type":"frame","id":"body","padding":[0,24],"children":[text("body-t","Body")]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    // `viewport` already exists (no leading padding of its own) and `lane`
    // is the append's inserted root — i.e. root_id lands on the INNER lane,
    // not the outer scroller. The outer add and inner clear are an atomic
    // pair; since the outer viewport is out of scope, neither half may
    // apply, or `lane`'s own canonical rail gets wiped with nothing put in
    // its place.
    repair_mobile_content_rails(&mut sink, "lane");

    assert_eq!(
        node_json(&sink, "lane")["padding"],
        json!([0.0, 24.0]),
        "the inner lane's own rail must survive when its outer viewport pairing is out of scope"
    );
    assert!(
        node_json(&sink, "viewport").get("padding").is_none(),
        "the outer viewport is out of scope for this append and must not be touched either"
    );
}
