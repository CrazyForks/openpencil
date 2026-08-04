//! Native-host routing for the left rail's slides tab.
//!
//! Windows-gated like the other tests that solve layout: building the
//! scene runs `jian_skia::SkiaMeasure`, which aborts the process under
//! Windows CI's DirectWrite.

#![cfg(all(test, not(target_os = "windows")))]

use super::WidgetHostNative;
use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::{EditorState, LeftPanelTab, SlidesPanelTarget};
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
    // After the install, so it survives whatever the install resets.
    host.editor_state.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    host.last_viewport_w = VW;
    host.last_viewport_h = VH;
    host
}

fn row_centre(host: &mut WidgetHostNative, index: usize) -> Point2D {
    let slides = host
        .slides_panel_frame(VW, VH)
        .expect("a deck shows the slides tab");
    let rect = slides.layout.row_rect(index);
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

/// Each board's authored position, keyed by id so the comparison is
/// about GEOMETRY and not about the order — the order is exactly what a
/// reorder is allowed to change.
fn board_geometry(host: &WidgetHostNative) -> std::collections::BTreeMap<String, (f64, f64)> {
    host.editor_state
        .active_children()
        .iter()
        .map(|node| {
            let base = op_editor_core::PenNodeExt::base(node);
            (
                base.id.clone(),
                (base.x.unwrap_or(0.0), base.y.unwrap_or(0.0)),
            )
        })
        .collect()
}

#[test]
fn only_a_deck_document_gets_the_slides_tab() {
    let _guard = test_lock();
    let mut deck = host_with(Some(TemplateScene::Slides));
    let slides = deck
        .slides_panel_frame(VW, VH)
        .expect("a deck shows the slides tab");
    assert_eq!(slides.chips.len(), 3);
    assert_eq!(slides.chips[1].name, "议程");
    assert!(deck.slides_tab_row(VH).is_some());

    let mut ordinary = host_with(None);
    assert!(
        ordinary.slides_panel_frame(VW, VH).is_none(),
        "an untagged document is layers-only"
    );
    assert!(
        ordinary.slides_tab_row(VH).is_none(),
        "and shows no tab row to switch with"
    );
}

#[test]
fn a_collapsed_sidebar_has_no_slides_tab_at_all() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    host.editor_state.editor_ui.sidebar_open = false;
    assert!(host.slides_panel_frame(VW, VH).is_none());
    assert!(host.slides_tab_row(VH).is_none());
}

#[test]
fn presenting_hides_the_rail_and_it_eats_no_clicks() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let point = row_centre(&mut host, 1);
    assert!(host.enter_preview((VW, VH)), "the deck presents");
    assert!(host.preview_slideshow_active());
    assert!(
        host.slides_panel_frame(VW, VH).is_none(),
        "the presentation owns the screen"
    );
    assert!(host.slides_tab_row(VH).is_none());
    // A press where a row used to be reaches the presentation.
    let before = host
        .editor_state
        .preview_slideshow()
        .map(|show| show.index());
    host.apply_press(point.x, point.y, VW, VH);
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
fn clicking_a_row_frames_that_board() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let point = row_centre(&mut host, 2);
    let before = host.editor_state.viewport;

    host.apply_press(point.x, point.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);

    assert_ne!(host.editor_state.viewport, before, "the camera moved");
    // The third board's centre is doc (4200 + 960, 540); framing it puts
    // that point at the centre of the canvas region.
    let (cx0, cy0, cw, ch) = host.canvas_region(VW, VH);
    let viewport = host.editor_state.viewport;
    let screen_x = 5_160.0 * viewport.zoom + viewport.pan_x;
    let screen_y = 540.0 * viewport.zoom + viewport.pan_y;
    assert!(
        (screen_x - cw / 2.0).abs() < 2.0 && (screen_y - ch / 2.0).abs() < 2.0,
        "board 3 is centred in the canvas region ({cx0},{cy0},{cw},{ch}) — got ({screen_x},{screen_y})"
    );
    assert!(
        host.editor_state.history.past.is_empty(),
        "navigating a deck is camera-only and never lands on the undo stack"
    );
}

#[test]
fn dragging_a_row_reorders_the_deck_without_moving_any_board() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let geometry_before = board_geometry(&host);
    assert_eq!(
        active_page_boards(&host.editor_state),
        ["slide-1", "slide-2", "slide-3"]
    );

    let from = row_centre(&mut host, 0);
    let to = row_centre(&mut host, 2);
    host.apply_press(from.x, from.y, VW, VH);
    host.apply_cursor_move(to.x, to.y);
    host.apply_release_with_viewport(VW, VH);

    assert_eq!(
        active_page_boards(&host.editor_state),
        ["slide-2", "slide-1", "slide-3"],
        "the first slide moved past the second"
    );
    assert_eq!(
        board_geometry(&host),
        geometry_before,
        "a reorder rewrites child ORDER only — no board moved on the canvas"
    );

    assert!(host.apply_undo(), "the reorder is undoable");
    assert_eq!(
        active_page_boards(&host.editor_state),
        ["slide-1", "slide-2", "slide-3"]
    );
    assert_eq!(board_geometry(&host), geometry_before);
}

#[test]
fn the_footer_button_starts_the_presentation() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let button = {
        let slides = host.slides_panel_frame(VW, VH).expect("slides tab");
        Point2D::new(
            slides.layout.present.origin.x + slides.layout.present.size.x / 2.0,
            slides.layout.present.origin.y + slides.layout.present.size.y / 2.0,
        )
    };
    host.apply_press(button.x, button.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    assert!(
        host.preview_slideshow_active(),
        "the footer starts the same slideshow the top bar's Play does"
    );
}

#[test]
fn the_tab_row_switches_the_rail_both_ways() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let tabs = host.slides_tab_row(VH).expect("tab row");
    let layers_tab = Point2D::new(
        tabs.layers.origin.x + tabs.layers.size.x / 2.0,
        tabs.layers.origin.y + tabs.layers.size.y / 2.0,
    );
    let slides_tab = Point2D::new(
        tabs.slides.origin.x + tabs.slides.size.x / 2.0,
        tabs.slides.origin.y + tabs.slides.size.y / 2.0,
    );

    host.apply_press(layers_tab.x, layers_tab.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.tab,
        LeftPanelTab::Layers
    );
    assert!(
        host.slides_panel_frame(VW, VH).is_none(),
        "the layer tree owns the rail now"
    );

    // The tab row still takes clicks while the tree owns the rail.
    host.apply_press(slides_tab.x, slides_tab.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.tab,
        LeftPanelTab::Slides
    );
    assert!(host.slides_panel_frame(VW, VH).is_some());
}

#[test]
fn the_layer_tree_starts_below_the_tab_row() {
    let _guard = test_lock();
    let mut deck = host_with(Some(TemplateScene::Slides));
    deck.editor_state.editor_ui.slides_panel.tab = LeftPanelTab::Layers;
    let tabs = deck.slides_tab_row(VH).expect("tab row");
    let content = deck.layers_content_rect(VH);
    assert_eq!(content.origin.y, tabs.row.origin.y + tabs.row.size.y);

    let ordinary = host_with(None);
    let full = ordinary.layers_content_rect(VH);
    assert_eq!(
        full.origin.y,
        op_editor_ui::widgets::TOP_BAR_HEIGHT,
        "a document without a tab row keeps the whole rail"
    );
}

#[test]
fn hovering_a_row_washes_it_and_leaving_the_rail_clears_it() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    let point = row_centre(&mut host, 1);
    host.apply_cursor_move(point.x, point.y);
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.hover,
        Some(SlidesPanelTarget::Slide(1))
    );
    host.apply_cursor_move(VW / 2.0, VH / 2.0);
    assert_eq!(host.editor_state.editor_ui.slides_panel.hover, None);
}

#[test]
fn the_wheel_over_the_rail_scrolls_the_list_and_not_the_canvas() {
    let _guard = test_lock();
    let mut host = host_with(Some(TemplateScene::Slides));
    // A three-slide deck fits, so nothing scrolls but the event is still
    // the panel's — the canvas must not zoom under the rail.
    let point = row_centre(&mut host, 0);
    let zoom_before = host.editor_state.viewport.zoom;
    host.apply_wheel(point.x, point.y, -400.0, VW, VH);
    assert_eq!(
        host.editor_state.viewport.zoom, zoom_before,
        "the rail swallowed the wheel"
    );
}
