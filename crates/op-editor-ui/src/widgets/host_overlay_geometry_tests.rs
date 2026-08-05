//! Placement of the overlays whose rect is not simply "a box in the
//! middle of the canvas".

use super::*;

const VIEWPORT_W: f32 = 1_600.0;
const VIEWPORT_H: f32 = 900.0;

fn gallery_state(layer_panel_width: f32) -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.sidebar_open = true;
    state.editor_ui.layer_panel_width = layer_panel_width;
    state.editor_ui.scene_template_center.open = true;
    state
}

/// The gallery is centred on the WINDOW, not on the canvas region. It
/// used to inset the canvas region, which took the left rail's width off
/// one side only and left the whole panel sitting visibly right of
/// centre whenever the rail was open.
#[test]
fn the_asset_center_is_centred_on_the_whole_viewport() {
    for width in [180.0_f32, 240.0, 480.0] {
        let state = gallery_state(width);
        let rect =
            scene_template_panel_rect(&state, VIEWPORT_W, VIEWPORT_H).expect("the gallery is open");
        let left = rect.origin.x;
        let right = VIEWPORT_W - (rect.origin.x + rect.size.x);
        assert!(
            (left - right).abs() < 0.01,
            "a {width}px rail pushed the gallery off centre: {left} left, {right} right"
        );
    }
}

/// It spans the same width as its own scrim, less the margin — the two
/// have to read as one surface.
#[test]
fn the_asset_center_spans_its_scrim_and_clears_the_top_bar() {
    let state = gallery_state(240.0);
    let rect =
        scene_template_panel_rect(&state, VIEWPORT_W, VIEWPORT_H).expect("the gallery is open");
    let scrim =
        scene_template_scrim_rect(&state, VIEWPORT_W, VIEWPORT_H).expect("the scrim is painted");
    assert_eq!(scrim.origin.x, 0.0);
    assert_eq!(scrim.size.x, VIEWPORT_W);
    assert!(
        rect.origin.x > scrim.origin.x && rect.origin.x < scrim.origin.x + 40.0,
        "the gallery is inset from the scrim's edge, not from the canvas region's"
    );
    // It overlaps the left rail on purpose; only the top bar stays clear.
    assert!(rect.origin.x < state.editor_ui.layer_panel_width);
    assert!(rect.origin.y > TOP_BAR_HEIGHT);
    assert_eq!(
        rect.origin.y - TOP_BAR_HEIGHT,
        VIEWPORT_H - (rect.origin.y + rect.size.y),
        "the top and bottom margins match"
    );
}

/// A window too small for the nominal margin gets a smaller one rather
/// than an inverted rect.
#[test]
fn a_tiny_viewport_shrinks_the_margin_instead_of_going_negative() {
    let state = gallery_state(240.0);
    let rect = scene_template_panel_rect(&state, 120.0, 90.0).expect("the gallery is open");
    assert!(rect.size.x > 0.0 && rect.size.y > 0.0);
    assert!(rect.origin.x >= 0.0);
    assert!(rect.origin.x + rect.size.x <= 120.0 + 0.01);
}

/// A closed gallery has no rect at all, so no host can paint or hit-test
/// one.
#[test]
fn a_closed_asset_center_has_neither_panel_nor_scrim() {
    let mut state = gallery_state(240.0);
    state.editor_ui.scene_template_center.open = false;
    assert!(scene_template_panel_rect(&state, VIEWPORT_W, VIEWPORT_H).is_none());
    assert!(scene_template_scrim_rect(&state, VIEWPORT_W, VIEWPORT_H).is_none());
}
