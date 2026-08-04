//! Web-host routing for the deck filmstrip — the twin of native's
//! `op-host-native/src/widget_host/deck_filmstrip_tests.rs`. Both hosts
//! run the same shared flow, and these tests are what proves it: the pair
//! has silently drifted apart every time only one side was covered.

use super::WidgetHost;
use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_ui::Point2D;

const VW: f32 = 1_400.0;
const VH: f32 = 900.0;

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

fn host_with(scenario: Option<TemplateScene>) -> WidgetHost {
    let document = jian_ops_schema::load_str(THREE_BOARD_DECK)
        .expect("parse deck fixture")
        .value;
    let mut host = WidgetHost::new();
    host.editor_state = op_editor_core::EditorState::from_document(document);
    host.editor_state.editor_ui.scenario = scenario;
    host.editor_state_dirty = true;
    host.last_viewport_w = VW;
    host.last_viewport_h = VH;
    host
}

fn chip_centre(host: &mut WidgetHost, index: usize) -> Point2D {
    let strip = host
        .deck_filmstrip_frame(VW, VH)
        .expect("a deck shows a filmstrip");
    let rect = strip.layout.chip_rect(index);
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn only_a_deck_document_gets_a_filmstrip() {
    let mut deck = host_with(Some(TemplateScene::Slides));
    let strip = deck
        .deck_filmstrip_frame(VW, VH)
        .expect("a deck shows a filmstrip");
    assert_eq!(strip.chips.len(), 3);
    assert_eq!(strip.chips[1].name, "议程");

    let mut ordinary = host_with(None);
    assert!(ordinary.deck_filmstrip_frame(VW, VH).is_none());
}

#[test]
fn clicking_a_chip_frames_that_board() {
    let mut host = host_with(Some(TemplateScene::Slides));
    let point = chip_centre(&mut host, 2);
    let before = host.editor_state.viewport;

    host.apply_press(point.x, point.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);

    let after = host.editor_state.viewport;
    assert_ne!(after, before, "the camera must move to the slide");
    let canvas =
        op_editor_ui::widgets::host_canvas_geometry::canvas_rect(&host.editor_state, VW, VH);
    let centre_doc_x = (canvas.size.x / 2.0 - after.pan_x) / after.zoom;
    assert!(
        (centre_doc_x - 5160.0).abs() < 30.0,
        "camera centre {centre_doc_x} is not on board 3"
    );
    assert!(
        host.editor_state.history.past.is_empty(),
        "navigating is camera-only and must not touch history"
    );
}

#[test]
fn dragging_a_chip_reorders_the_deck_and_undo_puts_it_back() {
    let mut host = host_with(Some(TemplateScene::Slides));
    let from = chip_centre(&mut host, 0);
    let to = chip_centre(&mut host, 2);

    host.apply_press(from.x, from.y, VW, VH);
    host.apply_cursor_move(to.x + 4.0, to.y);
    host.apply_release_with_viewport(VW, VH);

    assert_eq!(
        active_page_boards(&host.editor_state),
        ["slide-2", "slide-3", "slide-1"]
    );
    assert!(host.editor_state.undo(), "a reorder is undoable");
    assert_eq!(
        active_page_boards(&host.editor_state),
        ["slide-1", "slide-2", "slide-3"]
    );
}

#[test]
fn a_reorder_leaves_every_board_where_it_sat_on_the_canvas() {
    let mut host = host_with(Some(TemplateScene::Slides));
    let geometry = |host: &WidgetHost| -> Vec<(String, f64, f64)> {
        let mut rows: Vec<(String, f64, f64)> = host
            .editor_state
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
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        rows
    };
    let before = geometry(&host);
    let from = chip_centre(&mut host, 2);
    let to = chip_centre(&mut host, 0);

    host.apply_press(from.x, from.y, VW, VH);
    host.apply_cursor_move(to.x - 4.0, to.y);
    host.apply_release_with_viewport(VW, VH);

    assert_eq!(
        active_page_boards(&host.editor_state),
        ["slide-3", "slide-1", "slide-2"]
    );
    assert_eq!(
        geometry(&host),
        before,
        "a reorder changes the sequence only — no board may move on the canvas"
    );
}

#[test]
fn a_press_on_the_strip_never_reaches_the_canvas() {
    let mut host = host_with(Some(TemplateScene::Slides));
    host.zoom_to_fit(VW, VH);
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
    let mut host = host_with(Some(TemplateScene::Slides));
    let point = chip_centre(&mut host, 1);

    host.apply_cursor_move(point.x, point.y);
    assert_eq!(host.editor_state.editor_ui.deck_filmstrip.hover, Some(1));

    let strip = host.deck_filmstrip_rect(VW, VH).expect("a filmstrip");
    host.apply_cursor_move(strip.origin.x + 4.0, strip.origin.y - 60.0);
    assert_eq!(host.editor_state.editor_ui.deck_filmstrip.hover, None);
}
