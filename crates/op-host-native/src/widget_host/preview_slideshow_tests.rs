//! Deck slideshow host lifecycle: entering, arrow keys, and Escape.
//!
//! Windows-gated for the same reason the other preview host tests are:
//! entering preview solves layout through `jian_skia::SkiaMeasure`, which
//! aborts the process under Windows CI's DirectWrite.

#![cfg(all(test, not(target_os = "windows")))]

use super::WidgetHostNative;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::{EditorState, PreviewDeviceKind};
use std::sync::{LazyLock, Mutex, MutexGuard};

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn test_lock() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

/// Three 16:9 boards side by side — the shape a generated deck has.
const THREE_BOARD_DECK: &str = r##"{
    "version": "1.0.0",
    "children": [
        { "type": "frame", "id": "slide-1", "x": 0, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#ffffff"}], "children": [] },
        { "type": "frame", "id": "slide-2", "x": 2100, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#eeeeee"}], "children": [] },
        { "type": "frame", "id": "slide-3", "x": 4200, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#dddddd"}], "children": [] }
    ]
}"##;

fn host_with(source: &str, scenario: Option<TemplateScene>) -> WidgetHostNative {
    let document = jian_ops_schema::load_str(source)
        .expect("parse slideshow fixture")
        .value;
    let mut host = WidgetHostNative::new();
    let mut state = EditorState::from_document(document);
    state.editor_ui.scenario = scenario;
    host.install_imported_state(state);
    host
}

fn board_on_screen(host: &WidgetHostNative) -> Option<String> {
    host.editor_state
        .preview_slideshow()
        .and_then(|slideshow| slideshow.current_board())
        .map(str::to_string)
}

#[test]
fn a_deck_presents_from_board_zero_and_arrow_keys_move_through_it() {
    let _guard = test_lock();
    let mut host = host_with(THREE_BOARD_DECK, Some(TemplateScene::Slides));

    assert!(host.enter_preview((1200.0, 800.0)));
    assert!(host.preview_slideshow_active(), "a deck presents");
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-1"));
    assert_eq!(
        host.editor_state.editor_ui.preview.device,
        Some(PreviewDeviceKind::Canvas),
        "a slide has no phone or desktop silhouette"
    );

    assert!(host.preview_dispatch_key("ArrowRight", false));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-2"));
    assert!(host.preview_dispatch_key("ArrowRight", false));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-3"));

    // The end of the deck holds, it does not wrap to the title slide.
    assert!(host.preview_dispatch_key("ArrowRight", false));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-3"));

    assert!(host.preview_dispatch_key("ArrowLeft", false));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-2"));

    // Escape leaves preview through the existing ladder, ending the
    // presentation with it.
    assert!(host.apply_escape());
    assert!(!host.preview_slideshow_active());
    assert!(!host.editor_state.editor_ui.preview.mode);
    assert!(host.editor_state.preview_slideshow().is_none());
}

#[test]
fn the_presented_board_is_framed_by_the_viewport_fit() {
    let _guard = test_lock();
    let mut host = host_with(THREE_BOARD_DECK, Some(TemplateScene::Slides));

    host.enter_preview((1920.0, 1080.0));
    let first_zoom = host.editor_state.viewport.zoom;
    let first_pan_x = host.editor_state.viewport.pan_x;
    // A 1920x1080 board in a 1920x1080 canvas fits at 1:1, centred.
    assert!((first_zoom - 1.0).abs() < 1e-3, "zoom {first_zoom}");
    assert!(first_pan_x.abs() < 1e-3, "pan_x {first_pan_x}");

    // Advancing re-frames onto the next board, which sits 2100px away.
    // Paint owns the canvas size in production; the test drives the same
    // framing call paint makes.
    assert!(host.preview_slideshow_step(1));
    assert!(host.frame_slideshow_board((1920.0, 1080.0)));
    assert!(
        (host.editor_state.viewport.pan_x + 2100.0).abs() < 1e-3,
        "pan_x {}",
        host.editor_state.viewport.pan_x
    );
    assert!((host.editor_state.viewport.zoom - first_zoom).abs() < 1e-3);
}

#[test]
fn an_untagged_document_previews_interactively_as_before() {
    let _guard = test_lock();
    let mut host = host_with(THREE_BOARD_DECK, None);

    assert!(host.enter_preview((1200.0, 800.0)));

    assert!(!host.preview_slideshow_active());
    assert!(
        !host.preview_slideshow_step(1),
        "slideshow keys do nothing outside a presentation"
    );
}

/// The real shipped deck, not just a hand-written fixture: the template a
/// user actually opens has to present, and its board count has to match the
/// slides it was authored with.
#[test]
fn the_shipped_slide_deck_template_presents_every_board() {
    let _guard = test_lock();
    let source = op_editor_core::scene_template_catalog::scene_template_document("slide-deck")
        .expect("the deck template ships");
    let mut host = host_with(source, Some(TemplateScene::Slides));
    let authored_boards = host.editor_state.active_children().len();
    assert!(authored_boards > 1, "the deck has several slides");

    assert!(host.enter_preview((1200.0, 800.0)));

    let slideshow = host
        .editor_state
        .preview_slideshow()
        .expect("the deck presents");
    assert_eq!(slideshow.len(), authored_boards);
    assert_eq!(slideshow.counter_label(), format!("1 / {authored_boards}"));

    // Walking to the end lands on the last board and holds there.
    for _ in 0..authored_boards {
        host.preview_dispatch_key("ArrowRight", false);
    }
    let slideshow = host
        .editor_state
        .preview_slideshow()
        .expect("still present");
    assert_eq!(slideshow.index(), authored_boards - 1);
}

/// A deck tag on a page with no boards must not panic or trap the user in
/// an empty presentation — preview behaves exactly as it did before.
#[test]
fn a_deck_with_no_boards_falls_back_to_ordinary_preview() {
    let _guard = test_lock();
    let mut host = host_with(
        r#"{"version":"1.0.0","children":[]}"#,
        Some(TemplateScene::Slides),
    );

    assert!(host.enter_preview((1200.0, 800.0)));

    assert!(!host.preview_slideshow_active());
    assert!(!host.preview_dispatch_key("ArrowRight", false));
    assert!(host.apply_escape());
}
