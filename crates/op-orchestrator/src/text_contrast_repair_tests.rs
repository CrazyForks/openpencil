use super::*;
use op_design_lint::node_util::Variables;
use serde_json::json;

/// The palette shape a GENERATED document actually carries: token names as
/// `design_system` emits them, and each value a per-theme array.
///
/// This fixture previously used invented names (`color-text`) and a single
/// `{"value": …}` object. Both were wrong, and because the code under test
/// shared the same wrong assumptions the tests passed while the pass repaired
/// nothing in production. Copied from a real run (2026-08-02).
fn palette() -> Variables {
    [
        ("color-text-primary", "#0F172A", "#F1F5F9"),
        ("color-text-muted", "#94A3B8", "#94A3B8"),
        ("color-surface", "#FFFFFF", "#1E293B"),
        ("color-surface-2", "#F1F5F9", "#334155"),
        ("color-bg-deep", "#0B1220", "#020617"),
    ]
    .into_iter()
    .map(|(name, light, dark)| {
        (
            name.to_string(),
            serde_json::from_value(json!({
                "type": "color",
                "value": [
                    {"value": light, "theme": {"Mode": "Light"}},
                    {"value": dark, "theme": {"Mode": "Dark"}},
                ],
            }))
            .expect("variable"),
        )
    })
    .collect()
}

fn light_theme() -> op_design_lint::node_util::Theme {
    let themes = [(
        "Mode".to_string(),
        vec!["Light".to_string(), "Dark".to_string()],
    )]
    .into_iter()
    .collect();
    op_design_lint::node_util::default_theme(Some(&themes))
}

#[test]
fn white_text_on_a_light_board_is_repointed_at_the_ink_token() {
    // The shipped defect: title fill `$color-surface` (#FFFFFF) on a
    // `$color-surface-2` (#F1F5F9) board — 1.10:1, effectively blank.
    let token = best_token("#F1F5F9", &palette(), &light_theme()).expect("a readable token exists");
    assert_eq!(token, "color-text-primary");
}

#[test]
fn a_dark_board_gets_a_light_token_rather_than_the_ink_one() {
    // The reason the judgement is contrast and not the variable name: white
    // on dark is correct, and the shipped deck template's closing slide does
    // exactly this. A name-based rule ("text must not use color-surface")
    // would break it.
    let token = best_token("#0B1220", &palette(), &light_theme()).expect("a readable token exists");
    let hex = token_hex(&token, &palette(), &light_theme()).expect("token resolves");
    let ratio = op_design_lint::color::color_contrast(&hex, "#0B1220");
    assert!(ratio >= TARGET_RATIO, "{token} gives only {ratio:.2}:1");
    assert_ne!(
        token, "color-text-primary",
        "ink on a dark board stays unreadable"
    );
}

#[test]
fn the_chosen_token_always_clears_the_threshold() {
    for bg in ["#FFFFFF", "#F1F5F9", "#0B1220", "#64748B"] {
        let Some(token) = best_token(bg, &palette(), &light_theme()) else {
            continue;
        };
        let hex = token_hex(&token, &palette(), &light_theme()).expect("resolves");
        let ratio = op_design_lint::color::color_contrast(&hex, bg);
        assert!(
            ratio >= TARGET_RATIO,
            "bg {bg} -> {token} is only {ratio:.2}:1"
        );
    }
}

#[test]
fn a_palette_with_nothing_readable_repairs_nothing() {
    // Never invent a colour: if the document's own palette offers no
    // readable token, leave the fill alone rather than fabricating one.
    let flat: Variables = [
        ("color-text-primary", "#FEFEFE"),
        ("color-surface", "#FFFFFF"),
    ]
    .into_iter()
    .map(|(name, hex)| {
        (
            name.to_string(),
            serde_json::from_value(json!({
                "type": "color",
                "value": [{"value": hex, "theme": {"Mode": "Light"}}],
            }))
            .expect("variable"),
        )
    })
    .collect();
    assert_eq!(best_token("#FFFFFF", &flat, &light_theme()), None);
}

#[test]
fn non_colour_and_malformed_variables_are_ignored() {
    let odd: Variables = [
        (
            "color-text-primary",
            json!({"type": "number", "value": [{"value": 12, "theme": {"Mode": "Light"}}]}),
        ),
        (
            "color-surface",
            json!({"type": "color", "value": [{"value": "not-a-hex", "theme": {"Mode": "Light"}}]}),
        ),
    ]
    .into_iter()
    .map(|(name, value)| {
        (
            name.to_string(),
            serde_json::from_value(value).expect("variable"),
        )
    })
    .collect();
    assert_eq!(token_hex("color-text-primary", &odd, &light_theme()), None);
    assert_eq!(token_hex("color-surface", &odd, &light_theme()), None);
    assert_eq!(best_token("#FFFFFF", &odd, &light_theme()), None);
}

/// The pass end-to-end against a document, not just its colour picker.
///
/// Every earlier check here exercised `best_token` in isolation, and two real
/// generation runs failed to exercise the pass at all — the first because the
/// pass was broken, the second because that run's model happened to pick
/// readable colours, so nothing triggered. A repair whose only evidence is
/// "the output looked fine" has not been verified; this makes the trigger
/// unavoidable.
#[test]
fn invisible_text_in_a_document_is_actually_repaired() {
    use crate::test_support::VecDocSink;
    use op_editor_core::{EditorCommand, NodeId};

    let mut sink = VecDocSink::new();
    sink.state.doc.variables = Some(palette());
    sink.state.doc.themes = Some(
        [(
            "Mode".to_string(),
            vec!["Light".to_string(), "Dark".to_string()],
        )]
        .into_iter()
        .collect(),
    );

    // A light board carrying white text: the shipped cover defect exactly.
    let tree: jian_ops_schema::node::PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "board",
        "name": "Cover",
        "width": 1920,
        "height": 1080,
        "fill": [{"type": "solid", "color": "$color-surface-2"}],
        "children": [{
            "type": "text",
            "id": "title",
            "name": "Title",
            "content": "看不见的标题",
            "fontSize": 64,
            "fill": [{"type": "solid", "color": "$color-surface"}]
        }]
    }))
    .expect("tree");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = op_editor_core::PenNodeExt::id_str(&sink.state.active_children()[0]).to_string();

    let repaired = repair_text_contrast(&mut sink, &root_id);
    assert_eq!(repaired, 1, "the invisible title must be repaired");

    // The fill now points at ink, and the ratio actually clears the bar.
    let json = serde_json::to_value(&sink.state.active_children()[0]).expect("serialize");
    let fill = json["children"][0]["fill"][0]["color"]
        .as_str()
        .expect("text fill");
    assert_eq!(fill, "$color-text-primary");
    let ink = token_hex("color-text-primary", &palette(), &light_theme()).expect("ink");
    let bg = token_hex("color-surface-2", &palette(), &light_theme()).expect("bg");
    assert!(op_design_lint::color::color_contrast(&ink, &bg) >= TARGET_RATIO);
}

#[test]
fn readable_text_is_left_untouched() {
    use crate::test_support::VecDocSink;
    use op_editor_core::{EditorCommand, NodeId};

    let mut sink = VecDocSink::new();
    sink.state.doc.variables = Some(palette());
    let tree: jian_ops_schema::node::PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "board",
        "name": "Cover",
        "width": 1920,
        "height": 1080,
        "fill": [{"type": "solid", "color": "$color-surface"}],
        "children": [{
            "type": "text",
            "id": "title",
            "name": "Title",
            "content": "看得见的标题",
            "fontSize": 64,
            "fill": [{"type": "solid", "color": "$color-text-primary"}]
        }]
    }))
    .expect("tree");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = op_editor_core::PenNodeExt::id_str(&sink.state.active_children()[0]).to_string();

    assert_eq!(
        repair_text_contrast(&mut sink, &root_id),
        0,
        "already-readable text must not be rewritten"
    );
}
