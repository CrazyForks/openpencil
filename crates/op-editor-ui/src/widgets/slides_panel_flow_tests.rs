//! Flow tests for the left rail's slides tab: which documents get the
//! tab row, tab switching, click-to-navigate vs drag-to-reorder, and
//! the scroll seam.

use super::*;
use crate::widgets::slides_panel::DRAG_THRESHOLD_PX;
use op_editor_core::scene_template_catalog::TemplateScene;

const THREE_BOARDS: &str = r#"{"version":"1.0.0","children":[
    {"type":"frame","id":"slide-1","name":"Cover","x":0,"y":0,"width":1920,"height":1080},
    {"type":"frame","id":"slide-2","name":"Agenda","x":2100,"y":0,"width":1920,"height":1080},
    {"type":"frame","id":"slide-3","name":"封面之后的一页","x":4200,"y":0,"width":1920,"height":1080}
]}"#;

const PANEL: Rect = Rect {
    origin: Point2D { x: 0.0, y: 48.0 },
    size: Point2D { x: 240.0, y: 700.0 },
};

fn deck_state(source: &str) -> EditorState {
    let document = jian_ops_schema::load_str(source)
        .expect("fixture parses")
        .value;
    let mut state = EditorState::from_document(document);
    state.editor_ui.scenario = Some(TemplateScene::Slides);
    state.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    state
}

/// The three boards as the layout pass resolves them.
fn scene() -> LayoutScene {
    use crate::layout_scene::{NodeKind, SceneNode, ScenePage};
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

fn laid_out(state: &EditorState) -> (Vec<BoardChip>, SlidesPanelLayout) {
    let chips = slides(state);
    let layout = layout(state, &chips, &scene(), PANEL).expect("the rail has room");
    (chips, layout)
}

fn row_centre(layout: &SlidesPanelLayout, index: usize) -> Point2D {
    let rect = layout.row_rect(index);
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn any_document_with_boards_gets_a_tab_row() {
    let deck = deck_state(THREE_BOARDS);
    assert!(tab_row_visible(&deck));
    assert_eq!(slides_tab_label_key(&deck), "slidesPanel.tabSlides");

    // An untagged design page with frames on it is exactly the case the
    // navigator used to be missing from.
    let mut ordinary = deck_state(THREE_BOARDS);
    ordinary.editor_ui.scenario = None;
    assert!(tab_row_visible(&ordinary));
    assert!(tab_row(&ordinary, PANEL).is_some());
    assert_eq!(slides_tab_label_key(&ordinary), "slidesPanel.tabSlides");

    let mut carousel = deck_state(THREE_BOARDS);
    carousel.editor_ui.scenario = Some(TemplateScene::Carousel);
    assert!(tab_row_visible(&carousel));
}

#[test]
fn the_scenario_names_the_tab_without_gating_it() {
    let mut cards = deck_state(THREE_BOARDS);
    cards.editor_ui.scenario = Some(TemplateScene::Card);
    assert!(tab_row_visible(&cards));
    assert_eq!(slides_tab_label_key(&cards), "slidesPanel.tabCards");
}

#[test]
fn presenting_and_empty_decks_show_no_tab_row() {
    let mut deck = deck_state(THREE_BOARDS);
    deck.editor_ui.enter_preview();
    assert!(!tab_row_visible(&deck), "the rail is gone while presenting");
    deck.editor_ui.exit_preview();
    assert!(tab_row_visible(&deck));

    let empty = deck_state(r#"{"version":"1.0.0","children":[]}"#);
    assert!(!tab_row_visible(&empty));
}

#[test]
fn the_layers_tree_keeps_the_whole_rail_without_a_tab_row() {
    let empty = deck_state(r#"{"version":"1.0.0","children":[]}"#);
    assert_eq!(layers_content_rect(&empty, PANEL), PANEL);

    let deck = deck_state(THREE_BOARDS);
    let content = layers_content_rect(&deck, PANEL);
    assert_eq!(
        content.origin.y,
        PANEL.origin.y + crate::widgets::slides_panel::SLIDES_TAB_ROW_HEIGHT
    );
}

#[test]
fn a_stale_slides_tab_cannot_strand_a_document_with_nothing_to_list() {
    let mut empty = deck_state(r#"{"version":"1.0.0","children":[]}"#);
    empty.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    assert!(!slides_tab_active(&empty));
    assert!(layout(&empty, &slides(&empty), &scene(), PANEL).is_none());
}

#[test]
fn the_list_carries_the_documents_board_order_and_names() {
    let deck = deck_state(THREE_BOARDS);
    let chips = slides(&deck);
    assert_eq!(
        chips.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ["slide-1", "slide-2", "slide-3"]
    );
    assert_eq!(chips[2].name, "封面之后的一页");
}

#[test]
fn every_board_reports_its_own_aspect() {
    let deck = deck_state(THREE_BOARDS);
    let chips = slides(&deck);
    let aspects = board_aspects(&chips, &scene());
    assert_eq!(aspects.len(), chips.len(), "one aspect per board");
    assert!(aspects.iter().all(|a| (a - 1920.0 / 1080.0).abs() < 0.001));
    // No scene yet (a freshly opened document) falls back to 16:9 for
    // every row rather than collapsing the list.
    let unresolved = board_aspects(&chips, &LayoutScene::default());
    assert_eq!(unresolved.len(), chips.len());
    assert!(unresolved
        .iter()
        .all(|a| (a - DEFAULT_BOARD_ASPECT).abs() < 0.001));
}

#[test]
fn clicking_a_row_activates_that_slide() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    let point = row_centre(&layout, 1);
    assert_eq!(
        press(&mut deck, &layout, point),
        SlidesPress::Claimed(Some(SlidesPanelTarget::Slide(1)))
    );
    assert_eq!(release(&mut deck, &layout), SlidesRelease::Activate(1));
    assert_eq!(deck.editor_ui.slides_panel.pressed, None);
}

#[test]
fn a_press_off_the_rail_is_not_the_panels() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    assert_eq!(
        press(&mut deck, &layout, Point2D::new(900.0, 400.0)),
        SlidesPress::Missed
    );
    assert_eq!(release(&mut deck, &layout), SlidesRelease::Idle);
}

#[test]
fn releasing_off_the_pressed_row_cancels() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    press(&mut deck, &layout, row_centre(&layout, 0));
    // The cursor wandered onto a different row without travelling far
    // enough to be a drag: neither slide activates.
    deck.editor_ui.slides_panel.hover = Some(SlidesPanelTarget::Slide(2));
    assert_eq!(release(&mut deck, &layout), SlidesRelease::Cancelled);
}

#[test]
fn dragging_a_row_past_a_neighbour_reorders() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    press(&mut deck, &layout, row_centre(&layout, 0));
    let target = row_centre(&layout, 2);
    assert!(cursor_move(&mut deck, &layout, target), "the drag moved");
    assert_eq!(
        release(&mut deck, &layout),
        SlidesRelease::Reorder { from: 0, to: 1 },
        "dropping on row 2's lower half lands after row 1"
    );
}

#[test]
fn a_drag_dropped_where_it_started_changes_nothing() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    let start = row_centre(&layout, 1);
    press(&mut deck, &layout, start);
    cursor_move(
        &mut deck,
        &layout,
        Point2D::new(start.x, start.y + DRAG_THRESHOLD_PX + 1.0),
    );
    assert_eq!(release(&mut deck, &layout), SlidesRelease::Cancelled);
}

#[test]
fn the_reorder_is_the_shared_deck_move_command() {
    // The panel commits through the shared deck reorder, so the two
    // navigators can never write the deck order differently.
    let mut deck = deck_state(THREE_BOARDS);
    let before: Vec<String> = slides(&deck).into_iter().map(|c| c.id).collect();
    assert!(crate::widgets::deck_boards::apply_reorder(
        &mut deck, "slide-1", 2
    ));
    let after: Vec<String> = slides(&deck).into_iter().map(|c| c.id).collect();
    assert_eq!(before, ["slide-1", "slide-2", "slide-3"]);
    assert_eq!(after, ["slide-2", "slide-3", "slide-1"]);
}

#[test]
fn the_tabs_switch_and_dropping_the_slides_tab_drops_its_gesture() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    let tabs = tab_row(&deck, PANEL).expect("a deck has tabs");
    let layers_tab = Point2D::new(
        tabs.layers.origin.x + tabs.layers.size.x / 2.0,
        tabs.layers.origin.y + tabs.layers.size.y / 2.0,
    );
    press(&mut deck, &layout, layers_tab);
    assert_eq!(
        release(&mut deck, &layout),
        SlidesRelease::SelectTab(LeftPanelTab::Layers)
    );

    deck.editor_ui.slides_panel.hover = Some(SlidesPanelTarget::Slide(1));
    assert!(select_tab(&mut deck, LeftPanelTab::Layers));
    assert_eq!(deck.editor_ui.slides_panel.tab, LeftPanelTab::Layers);
    assert_eq!(
        deck.editor_ui.slides_panel.hover, None,
        "a hover belonging to a hidden list does not survive"
    );
    assert!(
        !select_tab(&mut deck, LeftPanelTab::Layers),
        "re-selecting the shown tab is not a change"
    );
}

#[test]
fn the_tab_row_takes_clicks_while_the_layers_tab_owns_the_rail() {
    let mut deck = deck_state(THREE_BOARDS);
    deck.editor_ui.slides_panel.tab = LeftPanelTab::Layers;
    let tabs = tab_row(&deck, PANEL).expect("a deck has tabs");
    let slides_tab = Point2D::new(
        tabs.slides.origin.x + tabs.slides.size.x / 2.0,
        tabs.slides.origin.y + tabs.slides.size.y / 2.0,
    );
    assert!(tab_cursor_move(&mut deck, &tabs, slides_tab));
    assert_eq!(
        deck.editor_ui.slides_panel.hover,
        Some(SlidesPanelTarget::SlidesTab)
    );
    deck.editor_ui.slides_panel.pressed = Some(SlidesPanelTarget::SlidesTab);
    assert_eq!(
        tab_release(&mut deck),
        SlidesRelease::SelectTab(LeftPanelTab::Slides)
    );
}

#[test]
fn the_footer_button_asks_to_present() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    let point = Point2D::new(
        layout.present.origin.x + layout.present.size.x / 2.0,
        layout.present.origin.y + layout.present.size.y / 2.0,
    );
    assert_eq!(
        press(&mut deck, &layout, point),
        SlidesPress::Claimed(Some(SlidesPanelTarget::Present))
    );
    assert_eq!(release(&mut deck, &layout), SlidesRelease::Present);
}

#[test]
fn the_wheel_scrolls_the_list_only_over_the_rail() {
    let mut deck = deck_state(THREE_BOARDS);
    let tall = SlidesPanelLayout::new(
        PANEL,
        SlidesPanelTabs::new(PANEL, LeftPanelTab::Slides, "Layers", "Slides"),
        &[DEFAULT_BOARD_ASPECT; 20],
        0.0,
    )
    .expect("layout");
    let over = Point2D::new(120.0, 300.0);
    assert_eq!(scroll(&mut deck, Some(&tall), over, -120.0), Some(true));
    assert!(deck.editor_ui.slides_panel.scroll.offset > 0.0);
    assert_eq!(
        scroll(&mut deck, Some(&tall), Point2D::new(900.0, 300.0), -120.0),
        None,
        "off the rail the wheel belongs to the canvas"
    );
}

/// The tab row's mode has to reach the product through the LOCALE, not
/// just through a hand-passed label: the flow is the only place either
/// host resolves the pair from, so this is where "Vietnamese at 180 px
/// shows icons" is either true or a lie the unit tests cannot catch.
#[test]
fn the_tab_row_mode_follows_the_documents_own_labels() {
    let mut deck = deck_state(THREE_BOARDS);
    let narrow = Rect {
        origin: PANEL.origin,
        size: Point2D::new(180.0, PANEL.size.y),
    };

    deck.editor_ui.locale = op_editor_core::Locale::EnUs;
    let (layers, slides) = tab_labels(&deck);
    assert_eq!((layers, slides), ("Layers", "Slides"));
    assert!(
        !tab_row(&deck, narrow).expect("tab row").compact,
        "English fits the minimum rail, so it keeps its words"
    );

    deck.editor_ui.locale = op_editor_core::Locale::Vi;
    let (layers, slides) = tab_labels(&deck);
    assert!(
        !layers.is_empty() && !slides.is_empty(),
        "the Vietnamese catalogue answers for both tabs"
    );
    assert!(
        tab_row(&deck, narrow).expect("tab row").compact,
        "Vietnamese does not fit the minimum rail, so it falls back to icons"
    );
    assert!(
        !tab_row(&deck, PANEL).expect("tab row").compact,
        "and gets its words back at the default width"
    );
}

/// The scenario still only picks the WORD, and the word is what the
/// row is measured against — so a scenario rename can flip the mode.
#[test]
fn the_scenario_label_is_the_one_the_row_is_measured_against() {
    let mut cards = deck_state(THREE_BOARDS);
    cards.editor_ui.scenario = Some(TemplateScene::Card);
    let (_, slides) = tab_labels(&cards);
    assert_eq!(
        slides,
        crate::widgets::editor_state_ext::translate(&cards.editor_ui, "slidesPanel.tabCards")
    );
}
