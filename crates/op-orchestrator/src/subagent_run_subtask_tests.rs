//! `run_subtask` end-to-end tests (insert/reveal/variable-binding/self-check).

use super::*;

#[test]
fn run_subtask_hoists_node_state_before_insert_subtree() {
    // A frame whose LLM output carries a `state` block should emit
    // a MergeAppState command BEFORE the InsertSubtree, and the inserted
    // nodes must carry no residual `state`.
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
        r#"I(null, {"type":"frame","name":"Card","x":0,"y":0,"width":1200,"height":200,
              "state":{"count":{"type":"int","default":0}},
              "children":[{"type":"text","content":"Hero","fontSize":18}]});"#
            .into(),
    )]);
    let mut plan = plan();
    plan.subtasks = vec![subtask()];
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &subtask(),
        &plan,
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    assert!(
        outcome.error.is_none(),
        "subtask must succeed: {:?}",
        outcome.error
    );
    // MergeAppState must precede InsertSubtree.
    let merge_pos = sink
        .applied
        .iter()
        .position(|c| matches!(c, EditorCommand::MergeAppState { .. }));
    let insert_pos = sink
        .applied
        .iter()
        .position(|c| matches!(c, EditorCommand::InsertSubtree { .. }));
    assert!(merge_pos.is_some(), "MergeAppState must be emitted");
    assert!(
        merge_pos.unwrap() < insert_pos.unwrap(),
        "MergeAppState must precede InsertSubtree"
    );
    // The inserted nodes must have state drained.
    let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
        panic!("last command must be InsertSubtree");
    };
    let PenNode::Frame(f) = &nodes[0] else {
        panic!()
    };
    assert!(f.state.is_none(), "inserted node must have state stripped");
}

#[test]
fn run_subtask_ok_applies_insert_subtree() {
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(NODE_SCRIPT.into())]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    assert_eq!(outcome.node_count, 1);
    assert!(outcome.error.is_none());
    assert!(matches!(
        sink.applied.last(),
        Some(EditorCommand::InsertSubtree { .. })
    ));
}

/// Reduced-complexity retry rung still uses script-gen; it narrows the
/// skill set only. The parser must therefore accept the same nested
/// `I(parent, node)` forest as the full attempt.
#[test]
fn run_subtask_reduced_complexity_still_uses_script_gen_nested_forest() {
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(NODE_SCRIPT.into())]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        true,
        false,
    ));
    assert_eq!(outcome.node_count, 1);
    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
        panic!("expected InsertSubtree, got {:?}", sink.applied.last());
    };
    assert_eq!(nodes.len(), 1);
    let children = nodes[0].children().expect("script-gen frame has children");
    assert_eq!(children.len(), 1);
    assert!(
        !nodes[0].id_str().is_empty(),
        "script-gen must assign a fresh root id"
    );
    assert_ne!(
        nodes[0].id_str(),
        "hero-1",
        "reduced retry must not use the retired flat-JSONL parser"
    );
}

#[test]
fn run_subtask_binds_generated_exact_color_to_document_variable() {
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
        r##"I(null, {"type":"rectangle","width":100,"height":50,"fill":[{"type":"solid","color":"#F8FAFC"}]});"##
            .into(),
    )]);
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::MergeThemePreset {
        variables: crate::semantic_palette::palette_variables(),
        themes: crate::semantic_palette::palette_themes(),
    });

    let outcome = block_on(run_subtask(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));

    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
        panic!("expected InsertSubtree, got {:?}", sink.applied.last());
    };
    assert_eq!(
        op_editor_core::fills::first_solid_fill_hex(&nodes[0]),
        Some("$color-bg-deep")
    );
}

#[test]
fn run_subtask_binds_generated_near_color_to_document_variable() {
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
        r##"I(null, {"type":"rectangle","width":100,"height":50,"fill":[{"type":"solid","color":"#FFF8F0"}]});"##
            .into(),
    )]);
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::MergeThemePreset {
        variables: crate::semantic_palette::palette_variables(),
        themes: crate::semantic_palette::palette_themes(),
    });

    let outcome = block_on(run_subtask(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));

    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
        panic!("expected InsertSubtree, got {:?}", sink.applied.last());
    };
    assert_eq!(
        op_editor_core::fills::first_solid_fill_hex(&nodes[0]),
        Some("$color-surface-3")
    );
}

#[test]
fn run_subtask_staggers_reveals_for_live_inserted_nodes() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(NODE_SCRIPT.into())]);
    let mut sink = VecDocSink::new();

    let outcome = block_on(run_subtask_with_reveal_at(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
        Some(epoch),
        1_234,
        None,
    ));

    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    assert_eq!(outcome.node_count, 1);
    let live_ids = collect_active_node_ids(sink.state());
    let snapshot = op_editor_core::agent_indicators::snapshot_at(1_250);
    assert_eq!(snapshot.reveals.len(), 2);
    assert!(
        snapshot
            .reveals
            .keys()
            .all(|id| live_ids.contains(id.as_str())),
        "reveals must reference live document ids"
    );
    let first = snapshot.reveals.values().min().copied().unwrap();
    let last = snapshot.reveals.values().max().copied().unwrap();
    assert_eq!(first, 1_234);
    assert!(last > first, "subtree nodes should reveal progressively");
    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

#[test]
fn run_subtask_zero_node_on_garbage() {
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text("the model refused".into())]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    assert_eq!(outcome.node_count, 0);
    assert!(outcome.error.is_some());
}

#[test]
fn skeleton_of_bare_rectangles_is_content_not_blank() {
    // 骨架屏 = frame 根 + 一排无子矩形线条;矩形虽带 ContainerProps
    // 但它是视觉本体,不能整批判空(ab-v9 全模型踩中)。
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
        r#"I(null, {"type":"frame","name":"Skeleton","width":"fill_container","height":"fit_content","children":[{"type":"rectangle","width":"fill_container","height":16,"cornerRadius":8},{"type":"rectangle","width":205,"height":16,"cornerRadius":8}]});"#
            .into(),
    )]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    assert_eq!(outcome.node_count, 1);
}

#[test]
fn run_subtask_rejects_blank_container_root() {
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
        r#"I(null, {"type":"frame","name":"Blank","x":0,"y":0,"width":390,"height":112,"children":[]});"#
            .into(),
    )]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));

    assert_eq!(outcome.node_count, 0);
    assert!(outcome
        .error
        .as_deref()
        .unwrap_or_default()
        .contains("blank container"));
    assert!(sink.applied.is_empty());
}

#[test]
fn run_subtask_auto_fixes_self_check_product_overflow() {
    let mut mobile_plan = plan();
    mobile_plan.root_frame.width = 390.0;
    mobile_plan.root_frame.height = 844.0;
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
        r#"I(null, {
          "type":"frame","name":"Popular Section","width":"fill_container","height":"fit_content","layout":"vertical","children":[
            {"type":"frame","name":"Popular Now Cards","width":"fill_container","height":"fit_content","layout":"horizontal","gap":20,"children":[
              {"type":"frame","role":"card","width":170,"height":220,"children":[
                {"type":"image","width":170,"height":120,"imageSearchQuery":"pasta plate"},
                {"type":"text","content":"Truffle Carbonara"}
              ]},
              {"type":"frame","role":"card","width":170,"height":220,"children":[
                {"type":"image","width":170,"height":120,"imageSearchQuery":"burger plate"},
                {"type":"text","content":"Smash Deluxe"}
              ]},
            ]}
          ]
        });"#
        .into(),
    )]);
    let mut sink = VecDocSink::new();

    let outcome = block_on(run_subtask(
        &subtask(),
        &mobile_plan,
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));

    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    assert_eq!(outcome.node_count, 1);
    let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
        panic!("expected InsertSubtree");
    };
    let fixed_json = serde_json::to_value(nodes).expect("serialize nodes");
    let row = &fixed_json[0]["children"][0];
    assert_eq!(row["gap"].as_f64(), Some(12.0));
    assert_eq!(
        row["children"][0]["width"],
        serde_json::json!("fill_container")
    );
    assert_eq!(
        row["children"][1]["width"],
        serde_json::json!("fill_container")
    );
}

#[test]
fn run_subtask_normalizes_section_root_for_parent_layout() {
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(
        r#"I(null, {"type":"frame","name":"Section","x":0,"y":0,"width":390,"height":112,"children":[{"type":"text","content":"Pizza","fontSize":18}]});"#
            .into(),
    )]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));

    assert_eq!(outcome.node_count, 1);
    let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
        panic!("expected InsertSubtree");
    };
    let PenNode::Frame(frame) = &nodes[0] else {
        panic!("expected frame root");
    };
    assert!(frame.base.x.is_none());
    assert!(frame.base.y.is_none());
    assert!(matches!(
        frame.container.width,
        Some(jian_ops_schema::sizing::SizingBehavior::Keyword(
            jian_ops_schema::sizing::SizingKeyword::FillContainer
        ))
    ));
    assert!(matches!(
        frame.container.height,
        Some(jian_ops_schema::sizing::SizingBehavior::Keyword(
            jian_ops_schema::sizing::SizingKeyword::FitContent
        ))
    ));
}

#[test]
fn run_subtask_zero_node_on_llm_error() {
    let llm = ScriptedLlm::new(vec![ScriptResponse::Fail(LlmError {
        message: "rate limited".into(),
        aborted: false,
    })]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    assert_eq!(outcome.node_count, 0);
    assert_eq!(outcome.error.as_deref(), Some("rate limited"));
}

#[test]
fn run_subtask_emits_subtask_skills_progress() {
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(NODE_SCRIPT.into())]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<crate::types::Progress> = Vec::new();
    let mut on_progress = |p: crate::types::Progress| events.push(p);
    let outcome = block_on(run_subtask_with_progress(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
        Some(&mut on_progress),
    ));
    assert_eq!(outcome.node_count, 1);
    assert!(
        events.iter().any(|p| matches!(
            p,
            crate::types::Progress::SubtaskSkills { id, .. } if id == "hero"
        )),
        "expected a SubtaskSkills event, got {events:?}"
    );
}

/// End-to-end: when the LLM emits a frame with role="input", run_subtask
/// must insert a text_input node (not a frame) into the document.
/// promote_forest runs AFTER post_pass_forest and BEFORE binding, so the
/// widget lands in the live document tree.
#[test]
fn run_subtask_promotes_role_input_frame_to_text_input() {
    // The LLM output contains a section frame whose only child is a
    // role="input" field with a muted placeholder and icon children.
    let llm_script = r##"I(null, {
      "type":"frame","name":"Login Form","width":1200,"height":400,
       "layout":"vertical","children":[
         {"type":"frame","role":"input","width":320,"height":48,
          "fill":[{"type":"solid","color":"#f3f4f6"}],"children":[
            {"type":"icon_font","iconFontName":"mail","width":20,"height":20},
            {"type":"text","content":"Email address",
             "fill":[{"type":"solid","color":"#9ca3af"}]}
          ]}
       ]
    });"##;
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(llm_script.into())]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));

    assert!(
        outcome.error.is_none(),
        "unexpected error: {:?}",
        outcome.error
    );
    assert_eq!(outcome.node_count, 1);

    let Some(EditorCommand::InsertSubtree { nodes, .. }) = sink.applied.last() else {
        panic!("expected InsertSubtree, got {:?}", sink.applied.last());
    };
    // The outer section frame is NOT a widget — it stays a frame.
    let PenNode::Frame(section) = &nodes[0] else {
        panic!("outer section must remain a frame");
    };
    // The inner role="input" child must have been promoted to text_input.
    let children = section.children.as_ref().expect("section has children");
    assert_eq!(children.len(), 1, "exactly one child (the promoted input)");
    let PenNode::TextInput(ti) = &children[0] else {
        panic!("role=input child must become TextInput after promotion");
    };
    // script-gen's batch_design executor remaps every id to a fresh one
    // (unlike the flat-JSONL path, which keeps the model-authored id), so
    // this only asserts an id WAS assigned, not its literal value.
    assert!(!ti.base.id.is_empty(), "promoted node must carry an id");
    assert!(ti.base.role.is_none(), "role cleared after promotion");
    assert_eq!(ti.leading_icon.as_deref(), Some("mail"));
    assert_eq!(ti.placeholder.as_deref(), Some("Email address"));
}
