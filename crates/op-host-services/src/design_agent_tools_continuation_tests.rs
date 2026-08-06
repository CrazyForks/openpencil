//! Sequential continuation regressions for the turn-scoped root contract.

use super::*;

#[test]
fn continuation_guard_survives_existing_only_batch_and_normalizes_later_roots() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(
        serde_json::from_value(serde_json::json!({
            "type": "frame", "id": "home", "name": "Nocturne 今夜",
            "width": 390, "height": 844,
            "fill": [{ "type": "solid", "color": "#050508" }],
            "children": [{ "type": "text", "id": "home-title", "content": "今夜天空" }]
        }))
        .expect("existing mobile screen"),
    );
    let mut guard = RootSeedGuard::from_prompt("mobile continuation");

    // DeepSeek may spend its first batch editing the old screen. That must
    // neither mutate its chrome nor consume the contract needed by siblings.
    let (first, first_mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r#"{"operations":"note=I(\"home\",{type:'text',name:'Existing screen note',content:'keep going',width:120,height:20})"}"#,
        None,
        Some(&mut guard),
    );
    assert!(!first.is_error, "first batch failed: {}", first.content);
    assert!(first_mutated);
    assert_eq!(state.active_children().len(), 1);
    assert!(state.active_children()[0]
        .children()
        .into_iter()
        .flatten()
        .all(|child| child.base().role.as_deref() != Some("status-bar")));

    let (second, second_mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r##"{"operations":"star=I(null,{type:'frame',name:'星图',width:1512,height:982,fill:[{type:'solid',color:'#16002E'}]})"}"##,
        None,
        Some(&mut guard),
    );
    assert!(!second.is_error, "second batch failed: {}", second.content);
    assert!(second_mutated);

    let (third, third_mutated) = execute_design_tool_with_root_seed_guard(
        &mut state,
        "batch_design",
        r#"{"operations":"plan=I(null,{type:'frame',name:'观测计划',width:375,height:812})"}"#,
        None,
        Some(&mut guard),
    );
    assert!(!third.is_error, "third batch failed: {}", third.content);
    assert!(third_mutated);

    for name in ["星图", "观测计划"] {
        let root = state
            .active_children()
            .iter()
            .find(|node| node.base().name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("missing generated screen {name}"));
        assert_eq!(
            (root.width_px(), root.height_px()),
            (Some(390.0), Some(844.0)),
            "{name} must inherit the live artboard"
        );
        assert_eq!(
            op_editor_core::first_solid_fill_hex(root),
            Some("#050508"),
            "{name} must inherit the live background"
        );
        assert_eq!(
            root.children()
                .and_then(|children| children.first())
                .and_then(|child| child.base().role.as_deref()),
            Some("status-bar"),
            "{name} must receive canonical mobile chrome"
        );
    }
}
