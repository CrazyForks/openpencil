//! Native-host routing for the deck filmstrip.
//!
//! Windows-gated like the other tests that solve layout: building the
//! scene runs `jian_skia::SkiaMeasure`, which aborts the process under
//! Windows CI's DirectWrite.

#![cfg(all(test, not(target_os = "windows")))]

use super::WidgetHostNative;
use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::EditorState;
use op_editor_ui::Point2D;
use std::sync::{LazyLock, Mutex, MutexGuard};

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn test_lock() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

const VW: f32 = 1_400.0;
const VH: f32 = 900.0;

/// Three 16:9 boards side by side — the shape a generated deck has.
const THREE_BOARD_DECK: &str = r##"{
    "version": "1.0.0",
    "children": [
        { "type": "frame", "id": "slide-1", "name": "Cover", "x": 0, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#ffffff"}], "children": [] },
        { "type": "frame", "id": "slide-2", "name": "议程", "x": 2100, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#eeeeee"}], "children": [] },
        { "type": "frame", "id": "slide-3", "name": "Closing", "x": 4200, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#dddddd"}], "children": [] }
    ]
}"##;

fn host_with(scenario: Option<TemplateScene>) -> WidgetHostNative {
    let document = jian_ops_schema::load_str(THREE_BOARD_DECK)
        .expect("parse deck fixture")
        .value;
    let mut host = WidgetHostNative::new();
    let mut state = EditorState::from_document(document);
    state.editor_ui.scenario = scenario;
    host.install_imported_state(state);
    host.last_viewport_w = VW;
    host.last_viewport_h = VH;
    host
}

fn chip_centre(host: &mut WidgetHostNative, index: usize) -> Point2D {
    let strip = host
        .deck_filmstrip_frame(VW, VH)
        .expect("a deck shows a filmstrip");
    let rect = strip.layout.chip_rect(index);
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn board_geometry(host: &WidgetHostNative) -> Vec<(String, f64, f64)> {
    host.editor_state
        .active_children()
        .iter()
        .map(|node| {
            let base = op_editor_core::PenNodeExt::base(node);
            (
                base.id.clone(),
                base.x.unwrap_or(0.0),
                base.y.unwrap_or(0.0),
            )
        })
        .collect()
}

#[test]
fn only_a_deck_document_gets_a_filmstrip() {
    let _guard = test_lock();
    let mut deck = host_with(Some(TemplateScene::Slides));
    let strip = deck
        .deck_filmstrip_frame(VW, VH)
        .expect("a deck shows a filmstrip");
    assert_eq!(strip.chips.len(), 3);
    assert_eq!(strip.chips[1].name, "议程");

    let mut ordinary = host_with(None);
    assert!(
        ordinary.deck_filmstrip_frame(VW, VH).is_none(),
        "an untagged document is not a deck"
    );
}

#[test]
fn presenting_hides_the_strip_and_it_eats_no_clicks() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let strip_rect = host.deck_filmstrip_rect(VW, VH).expect("a filmstrip");
    let centre = Point2D::new(
        strip_rect.origin.x + strip_rect.size.x / 2.0,
        strip_rect.origin.y + strip_rect.size.y / 2.0,
    );

    assert!(host.enter_preview((VW, VH)), "the deck presents");
    assert!(host.preview_slideshow_active());
    assert!(
        host.deck_filmstrip_frame(VW, VH).is_none(),
        "the presentation owns the canvas"
    );

    // A press where the strip used to be must reach the presentation, not
    // a ghost hit-box: it advances the deck like any other board press.
    let before = host
        .editor_state
        .preview_slideshow()
        .map(|show| show.index());
    host.apply_press(centre.x, centre.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    assert_eq!(before, Some(0));
    assert_eq!(
        host.editor_state
            .preview_slideshow()
            .map(|show| show.index()),
        Some(1),
        "the press belonged to the presentation underneath"
    );
}

#[test]
fn clicking_a_chip_frames_that_board() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let point = chip_centre(&mut host, 2);
    let before = host.editor_state.viewport;

    host.apply_press(point.x, point.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);

    let after = host.editor_state.viewport;
    assert_ne!(after, before, "the camera must move to the slide");
    // Board 3 spans x 4200..6120; framing it puts that span's centre at
    // the centre of the canvas region.
    let canvas =
        op_editor_ui::widgets::host_canvas_geometry::canvas_rect(host.editor_state(), VW, VH);
    let centre_doc_x = (canvas.size.x / 2.0 - after.pan_x) / after.zoom;
    assert!(
        (centre_doc_x - 5160.0).abs() < 30.0,
        "camera centre {centre_doc_x} is not on board 3"
    );
    let board_on_screen = 1920.0 * after.zoom;
    assert!(
        board_on_screen > canvas.size.x * 0.7 && board_on_screen <= canvas.size.x,
        "the board should fill the canvas region, not sit lost in it: \
         {board_on_screen} of {}",
        canvas.size.x
    );
    assert_eq!(
        host.editor_state.history.past.len(),
        0,
        "navigating is camera-only and must not touch history"
    );
}

#[test]
fn dragging_a_chip_reorders_the_deck_and_undo_puts_it_back() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let from = chip_centre(&mut host, 0);
    let to = chip_centre(&mut host, 2);

    host.apply_press(from.x, from.y, VW, VH);
    host.apply_cursor_move(to.x + 4.0, to.y);
    host.apply_release_with_viewport(VW, VH);

    assert_eq!(
        active_page_boards(&host.editor_state),
        ["slide-2", "slide-3", "slide-1"],
        "the page order follows the chips"
    );

    assert!(host.apply_undo(), "a reorder is undoable");
    assert_eq!(
        active_page_boards(&host.editor_state),
        ["slide-1", "slide-2", "slide-3"],
        "undo restores the authored order"
    );
}

#[test]
fn a_reorder_leaves_every_board_where_it_sat_on_the_canvas() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let before: std::collections::BTreeMap<String, (f64, f64)> = board_geometry(&host)
        .into_iter()
        .map(|(id, x, y)| (id, (x, y)))
        .collect();
    let from = chip_centre(&mut host, 2);
    let to = chip_centre(&mut host, 0);

    host.apply_press(from.x, from.y, VW, VH);
    host.apply_cursor_move(to.x - 4.0, to.y);
    host.apply_release_with_viewport(VW, VH);

    assert_eq!(
        active_page_boards(&host.editor_state),
        ["slide-3", "slide-1", "slide-2"]
    );
    let after: std::collections::BTreeMap<String, (f64, f64)> = board_geometry(&host)
        .into_iter()
        .map(|(id, x, y)| (id, (x, y)))
        .collect();
    assert_eq!(
        after, before,
        "a reorder changes the sequence only — no board may move on the canvas"
    );
}

#[test]
fn a_press_on_the_strip_never_reaches_the_canvas() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    // Frame the whole deck so a board really does lie under the strip.
    host.fit_content_to_viewport(VW, VH);
    let point = chip_centre(&mut host, 1);
    host.editor_state.selection = op_editor_core::SelectionState::empty();

    host.apply_press(point.x, point.y, VW, VH);

    assert!(
        host.editor_state.selection.is_empty(),
        "a chip press must not select the board painted behind the strip"
    );
}

#[test]
fn the_chip_under_the_cursor_takes_the_hover_wash() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let point = chip_centre(&mut host, 1);

    host.apply_cursor_move(point.x, point.y);
    assert_eq!(host.editor_state.editor_ui.deck_filmstrip.hover, Some(1));

    let strip = host.deck_filmstrip_rect(VW, VH).expect("a filmstrip");
    host.apply_cursor_move(strip.origin.x + 4.0, strip.origin.y - 60.0);
    assert_eq!(
        host.editor_state.editor_ui.deck_filmstrip.hover, None,
        "leaving the strip drops the wash"
    );
}
