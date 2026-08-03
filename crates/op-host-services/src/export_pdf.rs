//! PDF multi-page export. Skia ships a built-in PDF backend
//! (`skia_safe::pdf::new_document`) — each [`ScenePage`] becomes one
//! PDF page laid out at its bounding rect. Mirrors the TS app's
//! global-export → exportDocumentPdf flow, except the TS app
//! hand-rolls a PDF stream (Catalog + Pages + Image XObjects with
//! DCTDecode JPEG blobs); skia's backend emits real vector PDF ops,
//! which keeps glyphs + shapes selectable + zoom-clean.
//!
//! The export consumes a layout-resolved [`LayoutScene`] — every node
//! kind paints through the shared `crate::export::paint_nodes`, the
//! same live-canvas scene painter the raster path uses (see
//! `export/scene_painter.rs`), so PDF / PNG / screenshots stay in
//! lockstep with the editor canvas.

use op_editor_ui::layout_scene::{LayoutScene, SceneNode, ScenePage};
use op_editor_ui::Rect;
use std::path::Path as StdPath;

use crate::export::ExportError;

const PDF_MARGIN: f32 = 16.0;

fn render_pdf_item(
    bounds: Rect,
    paint: impl FnOnce(&skia_safe::Canvas),
) -> Result<Vec<u8>, ExportError> {
    let mut buf = Vec::new();
    {
        let mut pdf = skia_safe::pdf::new_document(&mut buf, None);
        let page_size = skia_safe::Size::new(
            bounds.size.x + PDF_MARGIN * 2.0,
            bounds.size.y + PDF_MARGIN * 2.0,
        );
        let mut page = pdf.begin_page(page_size, None);
        let canvas = page.canvas();
        canvas.translate((PDF_MARGIN - bounds.origin.x, PDF_MARGIN - bounds.origin.y));
        paint(canvas);
        pdf = page.end_page();
        pdf.close();
    }
    if buf.is_empty() {
        Err(ExportError::PdfEncoderEmpty)
    } else {
        Ok(buf)
    }
}

/// Render one resolved page as a single tightly cropped PDF page.
pub fn render_page_pdf_bytes(page: &ScenePage) -> Result<Vec<u8>, ExportError> {
    let bounds = crate::export::page_bounds(page).ok_or_else(|| ExportError::PageEmpty {
        page_id: page.id.clone(),
    })?;
    render_pdf_item(bounds, |canvas| {
        crate::export::paint_nodes(canvas, &page.children)
    })
}

/// Render one node from its resolved containing page as a single PDF page.
pub fn render_node_on_page_pdf_bytes(
    page: &ScenePage,
    node_id: &str,
) -> Result<Vec<u8>, ExportError> {
    let node = page
        .find(node_id)
        .ok_or_else(|| ExportError::NodeNotFoundOnPage {
            node_id: node_id.to_string(),
            page_id: page.id.clone(),
        })?;
    if node.hidden {
        return Err(ExportError::NodeHidden {
            node_id: node_id.to_string(),
        });
    }
    let mut acc = crate::export::BoundsAcc::new();
    crate::export::collect_bounds(node, glam::Affine2::IDENTITY, &mut acc);
    let bounds = acc
        .into_rect()
        .ok_or_else(|| ExportError::NodePaintsNothing {
            node_id: node_id.to_string(),
        })?;
    render_pdf_item(bounds, |canvas| crate::export::paint_node(canvas, node))
}

/// In-memory PDF variant: render each requested node as one PDF page
/// (cropped to that node's painted bounds). Returns the raw PDF bytes
/// (`%PDF-` magic through `%%EOF`) so the caller can base64-encode or
/// write to a file. Errors when `node_ids` is empty, the active page is
/// missing, or none of the requested nodes have paintable content.
/// Unknown node ids are skipped silently (the caller is expected to have
/// pre-validated them or to surface the per-id errors separately).
pub fn render_nodes_pdf_bytes(
    scene: &LayoutScene,
    node_ids: &[&str],
    _scale: f32,
) -> Result<Vec<u8>, ExportError> {
    let page = scene.active_page().ok_or(ExportError::NoActivePage)?;
    if node_ids.is_empty() {
        return Err(ExportError::NoNodeIdsRequested);
    }
    // Collect (node, bounds) pairs for requested ids — skip unknowns and
    // nodes that paint nothing (mirrors export_node_raster's approach).
    let mut pairs = Vec::new();
    for &id in node_ids {
        let Some(node) = page.find(id) else { continue };
        let mut acc = crate::export::BoundsAcc::new();
        crate::export::collect_bounds(node, glam::Affine2::IDENTITY, &mut acc);
        let Some(bounds) = acc.into_rect() else {
            continue;
        };
        pairs.push((node, bounds));
    }
    if pairs.is_empty() {
        return Err(ExportError::NoRequestedNodesPaint);
    }
    // Find a common page size: the largest single-node bounds + margin on each
    // side, so every page in the PDF is the same dimensions.
    let (page_w, page_h) = {
        let mut max_w = 0.0_f32;
        let mut max_h = 0.0_f32;
        for (_, b) in &pairs {
            max_w = max_w.max(b.size.x);
            max_h = max_h.max(b.size.y);
        }
        (max_w + PDF_MARGIN * 2.0, max_h + PDF_MARGIN * 2.0)
    };
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut pdf = skia_safe::pdf::new_document(&mut buf, None);
        for (node, bounds) in &pairs {
            let mut on_page = pdf.begin_page(skia_safe::Size::new(page_w, page_h), None);
            let canvas = on_page.canvas();
            // Position the node's content with PDF_MARGIN inset from the page
            // edges, the same translation strategy as the multi-page exporter.
            canvas.translate((PDF_MARGIN - bounds.origin.x, PDF_MARGIN - bounds.origin.y));
            crate::export::paint_node(canvas, node);
            pdf = on_page.end_page();
        }
        pdf.close();
    }
    Ok(buf)
}

/// Emit every page in `scene` as one PDF page each. Every page emits
/// at the SAME size — the union of all page bounds plus a 16-pt
/// margin — so PDF viewers don't jump between heterogeneous page
/// sizes on scroll/zoom. Each page's content is positioned within
/// that frame at its own `(origin.x, origin.y)`; empty pages are
/// skipped. Returns Err when no page has paintable content.
pub fn export_pdf(scene: &LayoutScene, target: &StdPath) -> Result<(), ExportError> {
    let bounds_per_page: Vec<_> = scene.pages.iter().map(crate::export::page_bounds).collect();
    let any_some = bounds_per_page.iter().any(Option::is_some);
    if !any_some {
        return Err(ExportError::NothingToExport);
    }
    let (page_w, page_h) = {
        let mut max_w = 0.0_f32;
        let mut max_h = 0.0_f32;
        for b in bounds_per_page.iter().flatten() {
            max_w = max_w.max(b.size.x);
            max_h = max_h.max(b.size.y);
        }
        (max_w + PDF_MARGIN * 2.0, max_h + PDF_MARGIN * 2.0)
    };
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut pdf = skia_safe::pdf::new_document(&mut buf, None);
        for (page, bounds_opt) in scene.pages.iter().zip(bounds_per_page.iter()) {
            let Some(bounds) = bounds_opt else { continue };
            let mut on_page = pdf.begin_page(skia_safe::Size::new(page_w, page_h), None);
            let canvas = on_page.canvas();
            canvas.translate((PDF_MARGIN - bounds.origin.x, PDF_MARGIN - bounds.origin.y));
            crate::export::paint_nodes(canvas, &page.children);
            pdf = on_page.end_page();
        }
        pdf.close();
    }
    std::fs::write(target, &buf).map_err(|e| ExportError::Write(e.to_string()))
}

/// Emit one PDF page per deck board — the deck flavour of [`export_pdf`].
///
/// A deck is authored and presented one board at a time, so its PDF has to
/// be one board per page: the page-level [`export_pdf`] would render the
/// whole page bounding box, i.e. every slide plus the canvas gaps between
/// them, onto a single sheet.
///
/// Two properties are load-bearing and both are inherited rather than
/// re-derived:
///
/// * **Page order is the slideshow's order** — [`active_page_boards`]
///   (document child order) is the single source of truth for which board
///   comes after which. Sorting by geometry here would let the exported
///   file disagree with what Preview presents the moment an author nudges
///   a slide on the canvas.
/// * **No margin, page size == board size** — a projector or PDF viewer
///   scales the page to fill the screen, so any inset would letterbox the
///   slide and change its aspect ratio. Content that overflows the board
///   is cropped by the MediaBox, which is exactly what presenting does.
///
/// [`active_page_boards`]: op_editor_core::preview_slideshow::active_page_boards
pub fn export_deck_pdf(
    state: &op_editor_core::EditorState,
    target: &StdPath,
) -> Result<(), ExportError> {
    let boards = op_editor_core::preview_slideshow::active_page_boards(state);
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    // Without a resolvable active page no board can be looked up at all —
    // that is "nothing to export", not a separate failure mode.
    let Some(page) = scene.active_page() else {
        return Err(ExportError::NothingToExport);
    };
    let slides: Vec<&SceneNode> = boards
        .iter()
        .filter_map(|id| page.find(id))
        // Hidden boards are skipped for the same reason the slideshow
        // never presents them. A degenerate board is dropped too: skia
        // cannot open a zero-extent page.
        .filter(|node| !node.hidden && node.bounds.size.x > 0.0 && node.bounds.size.y > 0.0)
        .collect();
    if slides.is_empty() {
        return Err(ExportError::NothingToExport);
    }
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut pdf = skia_safe::pdf::new_document(&mut buf, None);
        for node in slides {
            let bounds = node.bounds;
            let mut on_page =
                pdf.begin_page(skia_safe::Size::new(bounds.size.x, bounds.size.y), None);
            let canvas = on_page.canvas();
            canvas.translate((-bounds.origin.x, -bounds.origin.y));
            crate::export::paint_node(canvas, node);
            pdf = on_page.end_page();
        }
        pdf.close();
    }
    std::fs::write(target, &buf).map_err(|e| ExportError::Write(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::test_support::filled_rect;
    use op_editor_ui::layout_scene::NodeKind;
    use op_editor_ui::layout_scene::{SceneNode, ScenePage};
    use op_editor_ui::{Color, Rect};

    fn two_page_scene() -> LayoutScene {
        LayoutScene {
            pages: vec![
                ScenePage {
                    id: "page-one".into(),
                    name: "Page One".into(),
                    children: vec![filled_rect(
                        "page-one-node",
                        0.0,
                        0.0,
                        20.0,
                        20.0,
                        Color::BLACK,
                    )],
                },
                ScenePage {
                    id: "page-two".into(),
                    name: "Page Two".into(),
                    children: vec![filled_rect(
                        "page-two-node",
                        100.0,
                        120.0,
                        40.0,
                        30.0,
                        Color::BLACK,
                    )],
                },
            ],
            active_page_index: 0,
        }
    }

    #[test]
    fn render_page_pdf_bytes_crops_one_resolved_page() {
        let scene = two_page_scene();
        let bytes = render_page_pdf_bytes(&scene.pages[1]).expect("page PDF");
        assert_eq!(&bytes[..5], b"%PDF-");
    }

    #[test]
    fn render_node_on_page_pdf_bytes_accepts_non_active_page() {
        let scene = two_page_scene();
        let bytes =
            render_node_on_page_pdf_bytes(&scene.pages[1], "page-two-node").expect("node PDF");
        assert_eq!(&bytes[..5], b"%PDF-");
    }

    #[test]
    fn pdf_export_emits_one_page_per_scene_page() {
        let mut ellipse = SceneNode::leaf("n20", NodeKind::Ellipse);
        ellipse.bounds = Rect::xywh(0.0, 0.0, 40.0, 40.0);
        ellipse.fill = Some(Color {
            r: 0.8,
            g: 0.2,
            b: 0.4,
            a: 1.0,
        });
        let scene = LayoutScene {
            pages: vec![
                ScenePage {
                    id: "p1".into(),
                    name: "Page 1".into(),
                    children: vec![filled_rect(
                        "n10",
                        0.0,
                        0.0,
                        60.0,
                        40.0,
                        Color {
                            r: 0.5,
                            g: 0.5,
                            b: 0.5,
                            a: 1.0,
                        },
                    )],
                },
                ScenePage {
                    id: "p2".into(),
                    name: "Page 2".into(),
                    children: vec![ellipse],
                },
            ],
            active_page_index: 0,
        };
        let tmp = std::env::temp_dir().join(format!("op-export-pdf-{}.pdf", std::process::id()));
        let res = export_pdf(&scene, &tmp);
        assert!(res.is_ok(), "export_pdf failed: {res:?}");
        let bytes = std::fs::read(&tmp).unwrap();
        // PDF header magic: %PDF-
        assert_eq!(&bytes[..5], b"%PDF-", "missing %PDF- header");
        // Should contain the EOF marker.
        let tail = &bytes[bytes.len().saturating_sub(8)..];
        assert!(
            tail.windows(5).any(|w| w == b"%%EOF"),
            "missing %%EOF trailer"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn pdf_export_skips_empty_pages_but_fails_when_all_empty() {
        let scene = LayoutScene {
            pages: vec![
                ScenePage {
                    id: "p1".into(),
                    name: "P1".into(),
                    children: Vec::new(),
                },
                ScenePage {
                    id: "p2".into(),
                    name: "P2".into(),
                    children: Vec::new(),
                },
            ],
            active_page_index: 0,
        };
        let tmp = std::env::temp_dir().join(format!("op-pdf-empty-{}.pdf", std::process::id()));
        let res = export_pdf(&scene, &tmp);
        assert!(res.is_err(), "expected Err on all-empty, got {res:?}");
        assert_eq!(res.unwrap_err().to_string(), "nothing to export");
    }

    // ─── Deck (Slides scenario) export ─────────────────────────────────

    use op_editor_core::scene_template_catalog::TemplateScene;
    use op_editor_core::EditorState;

    /// A deck state from a canonical document body — the same fixture
    /// shape `export_batch::tests::deck_state` uses, plus the scenario tag
    /// without which the document is not a deck at all.
    fn deck_state(children_json: &str) -> EditorState {
        let doc = jian_ops_schema::load_str(&format!(
            r#"{{"version":"1.0.0","children":[{children_json}]}}"#
        ))
        .expect("fixture JSON parses")
        .value;
        let mut state = EditorState::from_document(doc);
        state.editor_ui.scenario = Some(TemplateScene::Slides);
        state
    }

    fn board_json(id: &str, x: f32, y: f32, w: f32, h: f32, extra: &str) -> String {
        format!(
            r##"{{"type":"frame","id":"{id}","name":"{id}","x":{x},"y":{y},
                 "width":{w},"height":{h},
                 "fill":[{{"type":"solid","color":"#204080"}}]{extra}}}"##
        )
    }

    fn temp_pdf(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "op-deck-pdf-{tag}-{}-{nanos}.pdf",
            std::process::id()
        ))
    }

    /// Number of page objects in a PDF. skia writes each page dictionary
    /// uncompressed, so the type key is greppable in the raw bytes.
    fn count_pages(bytes: &[u8]) -> usize {
        let text = String::from_utf8_lossy(bytes);
        text.matches("/Type /Page\n").count() + text.matches("/Type /Page ").count()
    }

    /// `(width, height)` of every `/MediaBox [x0 y0 x1 y1]` in the file,
    /// in page order. Parsed rather than string-matched so the assertions
    /// state the geometry instead of skia's number formatting.
    fn media_boxes(bytes: &[u8]) -> Vec<(f32, f32)> {
        let text = String::from_utf8_lossy(bytes).into_owned();
        text.match_indices("/MediaBox")
            .filter_map(|(at, _)| {
                let rest = &text[at + "/MediaBox".len()..];
                let open = rest.find('[')?;
                let close = rest.find(']')?;
                let nums: Vec<f32> = rest[open + 1..close]
                    .split_whitespace()
                    .filter_map(|n| n.parse::<f32>().ok())
                    .collect();
                (nums.len() == 4).then(|| (nums[2] - nums[0], nums[3] - nums[1]))
            })
            .collect()
    }

    #[test]
    fn deck_pdf_emits_one_page_per_visible_board() {
        let state = deck_state(&format!(
            "{},{},{}",
            board_json("s1", 0.0, 0.0, 320.0, 180.0, ""),
            board_json("s2", 400.0, 0.0, 320.0, 180.0, ""),
            board_json("s3", 800.0, 0.0, 320.0, 180.0, "")
        ));
        let tmp = temp_pdf("count");

        export_deck_pdf(&state, &tmp).expect("deck PDF");

        let bytes = std::fs::read(&tmp).unwrap();
        assert_eq!(&bytes[..5], b"%PDF-", "missing %PDF- header");
        assert_eq!(count_pages(&bytes), 3, "one page per board");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn deck_pdf_pages_are_exactly_board_sized_with_no_margin() {
        // Two deliberately different sizes: a single shared page size (what
        // the non-deck exporter emits) could not satisfy both.
        let state = deck_state(&format!(
            "{},{}",
            board_json("s1", 0.0, 0.0, 320.0, 180.0, ""),
            board_json("s2", 500.0, 40.0, 640.0, 360.0, "")
        ));
        let tmp = temp_pdf("mediabox");

        export_deck_pdf(&state, &tmp).expect("deck PDF");

        let bytes = std::fs::read(&tmp).unwrap();
        assert_eq!(
            media_boxes(&bytes),
            vec![(320.0, 180.0), (640.0, 360.0)],
            "each page must be its own board's size, un-inset"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn deck_pdf_skips_hidden_boards() {
        let state = deck_state(&format!(
            "{},{},{}",
            board_json("s1", 0.0, 0.0, 320.0, 180.0, ""),
            board_json("s2", 400.0, 0.0, 640.0, 360.0, r#","visible":false"#),
            board_json("s3", 1200.0, 0.0, 200.0, 100.0, "")
        ));
        let tmp = temp_pdf("hidden");

        export_deck_pdf(&state, &tmp).expect("deck PDF");

        let bytes = std::fs::read(&tmp).unwrap();
        assert_eq!(count_pages(&bytes), 2, "hidden board must not get a page");
        assert_eq!(
            media_boxes(&bytes),
            vec![(320.0, 180.0), (200.0, 100.0)],
            "the visible boards keep their own sizes and order"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn deck_pdf_reports_nothing_to_export_when_every_board_is_hidden() {
        let state = deck_state(&format!(
            "{},{}",
            board_json("s1", 0.0, 0.0, 320.0, 180.0, r#","visible":false"#),
            board_json("s2", 400.0, 0.0, 320.0, 180.0, r#","visible":false"#)
        ));
        let tmp = temp_pdf("all-hidden");

        let res = export_deck_pdf(&state, &tmp);

        assert!(res.is_err(), "expected Err, got {res:?}");
        assert_eq!(res.unwrap_err().to_string(), "nothing to export");
        assert!(!tmp.exists(), "no file should be written");
    }

    #[test]
    fn deck_pdf_reports_nothing_to_export_without_any_board() {
        // Loose annotation beside the deck, no frames: nothing is a slide.
        let state = deck_state(
            r##"{"type":"rectangle","id":"note","x":0,"y":0,"width":40,"height":40,
                "fill":[{"type":"solid","color":"#ffffff"}]}"##,
        );
        let tmp = temp_pdf("no-boards");

        let res = export_deck_pdf(&state, &tmp);

        assert!(res.is_err(), "expected Err, got {res:?}");
        assert_eq!(res.unwrap_err().to_string(), "nothing to export");
        assert!(!tmp.exists(), "no file should be written");
    }

    #[test]
    fn deck_pdf_page_order_follows_document_children_not_geometry() {
        // The second child sits above-left of the first on the canvas, so a
        // geometry sort would swap them. Sizes differ so the page order is
        // readable straight off the MediaBox list.
        let state = deck_state(&format!(
            "{},{}",
            board_json("s1", 900.0, 900.0, 320.0, 180.0, ""),
            board_json("s2", 0.0, 0.0, 640.0, 360.0, "")
        ));
        let tmp = temp_pdf("order");

        export_deck_pdf(&state, &tmp).expect("deck PDF");

        let bytes = std::fs::read(&tmp).unwrap();
        assert_eq!(
            media_boxes(&bytes),
            vec![(320.0, 180.0), (640.0, 360.0)],
            "authored child order wins over canvas position"
        );
        let _ = std::fs::remove_file(&tmp);
    }
}
