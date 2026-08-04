use super::*;
use crate::widgets::deck_filmstrip::CHIP_W;
use op_editor_core::scene_template_catalog::TemplateScene;

const THREE_BOARDS: &str = r#"{"version":"1.0.0","children":[
    {"type":"frame","id":"slide-1","name":"Cover","x":0,"y":0,"width":1920,"height":1080},
    {"type":"frame","id":"slide-2","name":"Agenda","x":2100,"y":0,"width":1920,"height":1080},
    {"type":"frame","id":"slide-3","name":"封面之后的一页","x":4200,"y":0,"width":1920,"height":1080}
]}"#;

fn deck_state(source: &str) -> EditorState {
    let document = jian_ops_schema::load_str(source)
        .expect("fixture parses")
        .value;
    let mut state = EditorState::from_document(document);
    state.editor_ui.scenario = Some(TemplateScene::Slides);
    state
}

fn canvas() -> Rect {
    Rect {
        origin: Point2D::new(240.0, 48.0),
        size: Point2D::new(1000.0, 700.0),
    }
}

fn layout_for(chips: &[FilmstripChip]) -> FilmstripLayout {
    filmstrip_layout(chips, Some(0), canvas()).expect("the canvas has room")
}

fn chip_centre(layout: &FilmstripLayout, index: usize) -> Point2D {
    let rect = layout.chip_rect(index);
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn only_a_deck_document_lists_slides() {
    let deck = deck_state(THREE_BOARDS);
    let chips = filmstrip_chips(&deck).expect("a deck lists its boards");
    assert_eq!(
        chips.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ["slide-1", "slide-2", "slide-3"],
        "chip order is the document's child order"
    );
    assert_eq!(chips[2].name, "封面之后的一页");

    let mut ordinary = deck_state(THREE_BOARDS);
    ordinary.editor_ui.scenario = None;
    assert_eq!(filmstrip_chips(&ordinary), None);

    let mut carousel = deck_state(THREE_BOARDS);
    carousel.editor_ui.scenario = Some(TemplateScene::Carousel);
    assert_eq!(filmstrip_chips(&carousel), None);
}

#[test]
fn presenting_hides_the_strip() {
    let mut deck = deck_state(THREE_BOARDS);
    deck.editor_ui.enter_preview();
    assert_eq!(
        filmstrip_chips(&deck),
        None,
        "the canvas belongs to the presentation"
    );
    deck.editor_ui.exit_preview();
    assert!(filmstrip_chips(&deck).is_some());
}

#[test]
fn an_empty_deck_has_no_strip() {
    let empty = deck_state(r#"{"version":"1.0.0","children":[]}"#);
    assert_eq!(filmstrip_chips(&empty), None);
}

/// The three boards as the layout pass resolves them: 1920×1080 each,
/// laid out left to right with a gutter.
fn three_board_scene() -> crate::layout_scene::LayoutScene {
    use crate::layout_scene::{LayoutScene, NodeKind, SceneNode, ScenePage};
    let board = |id: &str, x: f32| {
        let mut node = SceneNode::leaf(id, NodeKind::Frame);
        node.bounds = Rect::xywh(x, 0.0, 1920.0, 1080.0);
        node
    };
    LayoutScene {
        pages: vec![ScenePage {
            id: "page-1".into(),
            name: "Page 1".into(),
            children: vec![
                board("slide-1", 0.0),
                board("slide-2", 2100.0),
                board("slide-3", 4200.0),
            ],
        }],
        active_page_index: 0,
    }
}

#[test]
fn the_current_slide_is_the_board_nearest_the_viewport_centre() {
    let mut deck = deck_state(THREE_BOARDS);
    let chips = filmstrip_chips(&deck).expect("boards");
    let scene = three_board_scene();

    // Camera parked on board 2 (its centre is doc (3060, 540)).
    deck.viewport.zoom = 0.25;
    deck.viewport.pan_x = canvas().size.x / 2.0 - 3060.0 * 0.25;
    deck.viewport.pan_y = canvas().size.y / 2.0 - 540.0 * 0.25;
    assert_eq!(
        active_chip_index(&chips, &scene, &deck, canvas()),
        Some(1),
        "the nearest board is the one on screen"
    );

    // Nudge the camera towards board 3 — the answer follows the camera,
    // not the selection or the document order.
    deck.viewport.pan_x = canvas().size.x / 2.0 - 5160.0 * 0.25;
    assert_eq!(active_chip_index(&chips, &scene, &deck, canvas()), Some(2));
}

#[test]
fn clicking_a_chip_activates_that_slide_on_release() {
    let mut deck = deck_state(THREE_BOARDS);
    let chips = filmstrip_chips(&deck).expect("boards");
    let layout = layout_for(&chips);
    let point = chip_centre(&layout, 2);

    assert_eq!(
        press(&mut deck, &layout, point),
        FilmstripPress::Claimed(Some(2))
    );
    assert_eq!(release(&mut deck, &layout), FilmstripRelease::Activate(2));
    // The gesture is closed — a second release does nothing.
    assert_eq!(release(&mut deck, &layout), FilmstripRelease::Idle);
}

#[test]
fn a_press_that_leaves_its_chip_activates_nothing() {
    let mut deck = deck_state(THREE_BOARDS);
    let chips = filmstrip_chips(&deck).expect("boards");
    let layout = layout_for(&chips);

    press(&mut deck, &layout, chip_centre(&layout, 1));
    // Straight down, off the strip: no horizontal travel, so this is not
    // a reorder either.
    let below = Point2D::new(
        chip_centre(&layout, 1).x,
        layout.strip.origin.y + layout.strip.size.y + 40.0,
    );
    cursor_move(&mut deck, &layout, below);
    assert_eq!(release(&mut deck, &layout), FilmstripRelease::Cancelled);
}

#[test]
fn a_press_on_the_strips_padding_is_swallowed_without_navigating() {
    let mut deck = deck_state(THREE_BOARDS);
    let chips = filmstrip_chips(&deck).expect("boards");
    let layout = layout_for(&chips);
    let padding = Point2D::new(layout.strip.origin.x + 2.0, layout.strip.origin.y + 2.0);

    assert_eq!(
        press(&mut deck, &layout, padding),
        FilmstripPress::Claimed(None),
        "the strip owns its own padding"
    );
    assert_eq!(release(&mut deck, &layout), FilmstripRelease::Idle);
}

#[test]
fn a_press_beside_the_strip_falls_through() {
    let mut deck = deck_state(THREE_BOARDS);
    let chips = filmstrip_chips(&deck).expect("boards");
    let layout = layout_for(&chips);
    let elsewhere = Point2D::new(canvas().origin.x + 4.0, canvas().origin.y + 4.0);

    assert_eq!(press(&mut deck, &layout, elsewhere), FilmstripPress::Missed);
    assert_eq!(deck.editor_ui.deck_filmstrip.pressed, None);
}

#[test]
fn dragging_a_chip_past_its_neighbour_reorders_on_release() {
    let mut deck = deck_state(THREE_BOARDS);
    let chips = filmstrip_chips(&deck).expect("boards");
    let layout = layout_for(&chips);

    press(&mut deck, &layout, chip_centre(&layout, 0));
    // Drop past chip 1's centre → slide 1 lands second.
    let drop = Point2D::new(chip_centre(&layout, 1).x + 4.0, chip_centre(&layout, 1).y);
    assert!(cursor_move(&mut deck, &layout, drop));
    assert_eq!(
        release(&mut deck, &layout),
        FilmstripRelease::Reorder { from: 0, to: 1 }
    );
}

#[test]
fn dropping_a_chip_back_where_it_started_is_not_a_reorder() {
    let mut deck = deck_state(THREE_BOARDS);
    let chips = filmstrip_chips(&deck).expect("boards");
    let layout = layout_for(&chips);

    press(&mut deck, &layout, chip_centre(&layout, 1));
    // Far enough to be a drag, not far enough to cross a neighbour.
    let nudge = Point2D::new(
        chip_centre(&layout, 1).x + CHIP_W / 4.0,
        chip_centre(&layout, 1).y,
    );
    cursor_move(&mut deck, &layout, nudge);
    assert_eq!(release(&mut deck, &layout), FilmstripRelease::Cancelled);
}

#[test]
fn the_carried_chip_hides_the_hover_wash_but_keeps_tracking_it() {
    let mut deck = deck_state(THREE_BOARDS);
    let chips = filmstrip_chips(&deck).expect("boards");
    let layout = layout_for(&chips);

    press(&mut deck, &layout, chip_centre(&layout, 0));
    cursor_move(&mut deck, &layout, chip_centre(&layout, 2));

    assert_eq!(
        deck.editor_ui.deck_filmstrip.hover,
        Some(2),
        "hover still tracks, so the release can tell a drop from a click"
    );
    let widget = filmstrip_widget(&chips, Some(0), &deck);
    assert_eq!(widget.hover, None, "the drop bar is the only landing hint");
    assert!(widget.drag.is_some());
}

#[test]
fn a_reorder_moves_the_board_in_child_order_and_touches_no_geometry() {
    let mut deck = deck_state(THREE_BOARDS);
    let before: Vec<(String, f64, f64)> = deck
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

    assert!(deck.apply(reorder_command("slide-1", 2)));

    assert_eq!(
        op_editor_core::preview_slideshow::active_page_boards(&deck),
        ["slide-2", "slide-3", "slide-1"],
        "the page order is the child order"
    );
    for (id, x, y) in before {
        let node = deck
            .active_children()
            .iter()
            .find(|node| op_editor_core::PenNodeExt::id_str(*node) == id)
            .expect("every board survives a reorder");
        let base = op_editor_core::PenNodeExt::base(node);
        assert_eq!(
            (base.x.unwrap_or(0.0), base.y.unwrap_or(0.0)),
            (x, y),
            "{id} moved on the canvas — a reorder must only change the sequence"
        );
    }
}
