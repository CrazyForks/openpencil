//! Regression tests for folding an inline formatting context into one
//! positioned styled text node (the rich-inline-text overlap fix).

use super::*;
use crate::HtmlImportOptions;
use jian_ops_schema::node::PenNode;

/// The regression this fix targets: a paragraph whose children are all inline
/// (bare text + `<a>` + `<code>`) that wraps across several lines. The
/// extractor now folds it into ONE positioned text node carrying styled
/// `segments`, so the block imports as a single wrapped run instead of one node
/// per inline child stacked at the block origin. Before the fix each wrapped
/// child's rect was the union of its line boxes — a full-width, multi-line box
/// whose top-left was the block's left edge — so consecutive children shared an
/// origin and the text rendered as an overlapping smear.
#[test]
fn folded_inline_block_is_one_styled_text_node_without_overlap() {
    // Two wrapped runs that, under the old per-child capture, carried the
    // SAME rect origin (0, 0) — the exact shape that produced the smear.
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "p",
        "rect": { "x": 0, "y": 0, "w": 300, "h": 80 },
        "styles": { "font-size": "16px" },
        "children": [
          { "kind": "text", "rect": { "x": 0, "y": 0, "w": 300, "h": 80 },
            "lines": 4,
            "text": "The rendering engine paints onto a Painter surface.",
            "styles": { "font-size": "16px", "color": "rgb(31, 35, 40)" },
            "segments": [
              { "text": "The ", "styles": { "color": "rgb(31, 35, 40)" } },
              { "text": "rendering", "styles": { "color": "rgb(9, 105, 218)",
                "text-decoration-line": "underline" }, "href": "https://example.com/j" },
              { "text": " engine paints onto a ", "styles": { "color": "rgb(31, 35, 40)" } },
              { "text": "Painter", "styles": { "font-family": "ui-monospace",
                "font-size": "13.6px" } },
              { "text": " surface.", "styles": { "color": "rgb(31, 35, 40)" } }
            ] }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let children = root.children.as_ref().unwrap();
    // ONE text node for the whole inline context — not a node per inline run.
    assert_eq!(
        children.len(),
        1,
        "the block must fold to a single text node"
    );
    let PenNode::Text(text) = &children[0] else {
        panic!("folded inline content must be a text node")
    };
    use jian_ops_schema::node::text::{TextContent, TextGrowth};
    use jian_ops_schema::sizing::SizingBehavior;
    let TextContent::Styled(segments) = &text.content else {
        panic!("mixed inline content must import as styled runs, not plain")
    };
    assert_eq!(segments.len(), 5);
    // The link run keeps its colour, underline, and href.
    let link = &segments[1];
    assert_eq!(link.text, "rendering");
    assert_eq!(link.href.as_deref(), Some("https://example.com/j"));
    assert_eq!(link.fill.as_deref(), Some("#0969da"));
    assert_eq!(link.underline, Some(true));
    // The code run keeps its monospace family and smaller size.
    let code = &segments[3];
    assert_eq!(code.text, "Painter");
    assert_eq!(code.font_family.as_deref(), Some("ui-monospace"));
    assert!(code
        .font_size
        .is_some_and(|size| (size - 13.6).abs() < 0.01));
    // Multi-line → wrapped mode: keep the captured width, grow the height.
    assert!(matches!(text.width, Some(SizingBehavior::Number(w)) if w == 300.0));
    assert_eq!(text.text_growth, Some(TextGrowth::FixedWidth));
}

/// Cross-version: a capture from the currently-installed extension carries no
/// `segments`, so each inline child is still its own text node. Those payloads
/// must keep importing unchanged — the folded shape is additive.
#[test]
fn text_node_without_segments_stays_plain() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "p",
        "rect": { "x": 0, "y": 0, "w": 300, "h": 20 },
        "children": [
          { "kind": "text", "rect": { "x": 0, "y": 0, "w": 120, "h": 20 },
            "text": "plain run", "lines": 1, "styles": {} }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Text(text) = &root.children.as_ref().unwrap()[0] else {
        panic!("text run")
    };
    use jian_ops_schema::node::text::TextContent;
    assert_eq!(text.content, TextContent::Plain("plain run".to_string()));
}

/// A single unstyled run (a lone `<span>` with no overrides) reduces to plain
/// text — no need to pay for styled content the run does not use.
#[test]
fn single_trivial_segment_reduces_to_plain_text() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "p",
        "rect": { "x": 0, "y": 0, "w": 300, "h": 20 },
        "children": [
          { "kind": "text", "rect": { "x": 0, "y": 0, "w": 120, "h": 20 },
            "text": "just text", "lines": 1, "styles": {},
            "segments": [ { "text": "just text", "styles": {} } ] }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Text(text) = &root.children.as_ref().unwrap()[0] else {
        panic!("text run")
    };
    use jian_ops_schema::node::text::TextContent;
    assert_eq!(text.content, TextContent::Plain("just text".to_string()));
}

/// Source-level guards for the inline-fold contract (see
/// `folded_inline_block_is_one_styled_text_node_without_overlap`). The
/// extractor needs a live DOM, so the invariants are pinned where they are
/// written.
#[test]
fn snapshot_extractor_pins_its_inline_fold_contract() {
    // A block that lays its children out as one inline flow folds to a single
    // text node instead of a node per inline child.
    assert!(
        SNAPSHOT_EXTRACTOR_JS.contains("isInlineTextBlock(element, computed)"),
        "inline blocks must be detected and folded"
    );
    // The folded node is positioned at the inline content's own box (a range
    // over the element's contents), so padding is excluded and every line box
    // is counted — never one union box per child stacked at the block origin.
    assert!(
        SNAPSHOT_EXTRACTOR_JS.contains("range.selectNodeContents(element)"),
        "the folded box must come from the inline content, not per-child rects"
    );
    // Per-run styling (link colour + href, code's monospace family) rides on
    // `segments`, so folding does not flatten the paragraph to one style.
    assert!(
        SNAPSHOT_EXTRACTOR_JS.contains("segments: emitted"),
        "folded inline runs must carry their styled segments"
    );
}
