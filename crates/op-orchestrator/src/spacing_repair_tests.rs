use super::*;
use serde_json::json;

fn node(v: Value) -> PenNode {
    serde_json::from_value(v).expect("valid PenNode")
}

fn card(id: &str) -> Value {
    json!({"type":"frame","id":id,"layout":"vertical","padding":24,
           "fill":[{"type":"solid","color":"#141414"}],
           "children":[{"type":"text","id":format!("{id}t"),"content":"x"}]})
}

#[test]
fn transparent_metrics_strip_loses_both_paddings() {
    // test0703.op shape: "Key Metrics" (transparent, padded [24,32]) inside a
    // [32,40]-padded gap-20 main column — its padding starved the cards.
    let mut root = node(json!({
        "type":"frame","id":"main","layout":"vertical","gap":20,"padding":[32,40],
        "children":[
            {"type":"frame","id":"metrics","layout":"horizontal","gap":16,"padding":[24,32],
             "children":[card("c1"), card("c2")]}
        ]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let v = serde_json::to_value(&root).unwrap();
    assert!(
        v["children"][0].get("padding").is_none(),
        "wrapper padding fully stripped: {}",
        v["children"][0]
    );
    // The CARDS keep their own padding — they paint a surface.
    assert_eq!(
        v["children"][0]["children"][0]["padding"].as_f64(),
        Some(24.0)
    );
}

#[test]
fn landing_sections_in_unpadded_root_keep_self_padding() {
    // Landing shape: root column has NO padding and NO gap — each section's
    // [0,24] self-padding IS the page inset. Must stay.
    let mut root = node(json!({
        "type":"frame","id":"page","layout":"vertical",
        "children":[
            {"type":"frame","id":"hero","layout":"vertical","padding":[64,24],
             "children":[{"type":"text","id":"h","content":"Hi"}]}
        ]
    }));
    assert!(!strip_wrapper_double_inset(&mut root));
}

#[test]
fn gapped_but_unpadded_column_strips_only_vertical() {
    // Parent gaps 24 but pads 0 horizontally: the wrapper's horizontal
    // padding is its only inset (keep); vertical padding double-spaces (strip).
    let mut root = node(json!({
        "type":"frame","id":"col","layout":"vertical","gap":24,
        "children":[
            {"type":"frame","id":"wrap","layout":"horizontal","padding":[16,24],
             "children":[card("c1")]}
        ]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let v = serde_json::to_value(&root).unwrap();
    assert_eq!(
        v["children"][0]["padding"],
        json!([0.0, 24.0, 0.0, 24.0]),
        "vertical stripped, horizontal kept"
    );
}

#[test]
fn painted_wrappers_are_not_touched() {
    // A filled panel / stroked card is a SURFACE — its padding is design.
    let mut root = node(json!({
        "type":"frame","id":"main","layout":"vertical","gap":20,"padding":[32,40],
        "children":[
            {"type":"frame","id":"panel","layout":"vertical","padding":24,
             "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#2A2A2A"}]},
             "children":[{"type":"text","id":"t","content":"x"}]}
        ]
    }));
    assert!(!strip_wrapper_double_inset(&mut root));
}

#[test]
fn divider_bar_header_strips_horizontal_keeps_vertical() {
    // A top bar whose only paint is its bottom hairline is a DIVIDER bar —
    // its [20,32] padding inside the [32,40]-padded column misaligns the
    // title 32px deeper than the sections below; horizontal strips, the
    // vertical breathing against its own rule stays.
    let mut root = node(json!({
        "type":"frame","id":"main","layout":"vertical","gap":20,"padding":[32,40],
        "children":[
            {"type":"frame","id":"bar","layout":"horizontal","padding":[20,32],
             "stroke":{"thickness":{"bottom":1},"fill":[{"type":"solid","color":"#2A2A2A"}]},
             "children":[{"type":"text","id":"t","content":"Title"}]}
        ]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let v = serde_json::to_value(&root).unwrap();
    assert_eq!(
        v["children"][0]["padding"],
        json!([20.0, 0.0, 20.0, 0.0]),
        "horizontal stripped, vertical kept: {}",
        v["children"][0]
    );
}

#[test]
fn side_stroked_panel_keeps_all_padding() {
    // A LEFT-accent stroke is a surface treatment, not a divider — padding
    // is design, untouched.
    let mut root = node(json!({
        "type":"frame","id":"main","layout":"vertical","gap":20,"padding":[32,40],
        "children":[
            {"type":"frame","id":"panel","layout":"vertical","padding":[16,20],
             "stroke":{"thickness":{"left":2},"fill":[{"type":"solid","color":"#C9A962"}]},
             "children":[{"type":"text","id":"t","content":"Note"}]}
        ]
    }));
    assert!(!strip_wrapper_double_inset(&mut root));
}

#[test]
fn nested_transparent_chain_converges_on_one_gutter_owner() {
    // rail(24) > sheet(24) > inner(24): every structural layer re-adding the
    // rail's gutter. Only the rail may keep it — otherwise the card lands
    // 48px inside its siblings instead of 24.
    let mut root = node(json!({
        "type":"frame","id":"rail","layout":"vertical","padding":[0,24],
        "children":[
            {"type":"frame","id":"sheet","layout":"vertical","padding":[16,24],
             "children":[
                 {"type":"frame","id":"inner","layout":"vertical","padding":[8,24],
                  "children":[card("c")]}
             ]}
        ]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let v = serde_json::to_value(&root).unwrap();
    let sheet = &v["children"][0];
    let inner = &sheet["children"][0];
    assert_eq!(sheet["padding"], json!([16.0, 0.0, 16.0, 0.0]));
    assert_eq!(inner["padding"], json!([8.0, 0.0, 8.0, 0.0]));
    assert_eq!(
        inner["children"][0]["padding"],
        json!(24.0),
        "the opaque card's own inset survives the whole chain"
    );
}

#[test]
fn nested_chain_strip_is_a_fixed_point() {
    let mut root = node(json!({
        "type":"frame","id":"rail","layout":"vertical","padding":[0,24],
        "children":[
            {"type":"frame","id":"sheet","layout":"vertical","padding":[16,24],
             "children":[
                 {"type":"frame","id":"inner","layout":"vertical","padding":[8,24],
                  "children":[card("c")]}
             ]}
        ]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let once = serde_json::to_value(&root).unwrap();
    assert!(
        !strip_wrapper_double_inset(&mut root),
        "a second run must find nothing left to strip"
    );
    assert_eq!(once, serde_json::to_value(&root).unwrap());
}

#[test]
fn gutter_claim_does_not_cross_a_card_surface() {
    // rail(24) > card(fill, padding 8) > lane(padding 12): the lane sits
    // inside a real surface whose own inset is too small to claim a gutter,
    // so nothing authorizes stripping it. Were the rail's claim to leak
    // through the card, the lane would lose its 12px.
    let mut root = node(json!({
        "type":"frame","id":"rail","layout":"vertical","padding":[0,24],
        "children":[
            {"type":"frame","id":"surface","layout":"vertical","padding":8,
             "fill":[{"type":"solid","color":"#141414"}],
             "children":[
                 {"type":"frame","id":"lane","layout":"vertical","padding":[8,12],
                  "children":[card("c")]}
             ]}
        ]
    }));
    let v = serde_json::to_value(&root).unwrap();
    let before = v["children"][0].clone();
    strip_wrapper_double_inset(&mut root);
    assert_eq!(
        serde_json::to_value(&root).unwrap()["children"][0],
        before,
        "nothing under a painted card is a rail-gutter duplicate"
    );
}

#[test]
fn uniformly_wrapped_siblings_stay_equal_width() {
    // All three sections inset through an identical transparent wrapper. The
    // gutter moves to the single owner (the rail), so the sections stay equal
    // to each other — the pass never introduces the asymmetry it removes.
    let wrapper = |id: &str| {
        json!({"type":"frame","id":id,"layout":"vertical","padding":[16,24],
               "children":[card(&format!("{id}-c"))]})
    };
    let mut root = node(json!({
        "type":"frame","id":"rail","layout":"vertical","padding":[0,24],"gap":20,
        "children":[wrapper("a"), wrapper("b"), wrapper("c")]
    }));
    assert!(strip_wrapper_double_inset(&mut root));
    let v = serde_json::to_value(&root).unwrap();
    let pads: Vec<_> = v["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["padding"].clone())
        .collect();
    assert_eq!(
        pads,
        vec![Value::Null; 3],
        "every sibling loses the same padding, so their widths stay identical"
    );
}
