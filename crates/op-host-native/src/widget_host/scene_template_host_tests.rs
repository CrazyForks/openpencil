//! Native-host routing for the Scene Template Center.

use super::WidgetHostNative;
use op_editor_ui::widgets::SceneTemplatePanel;
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1_200.0;
const VIEWPORT_H: f32 = 800.0;

fn open_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    host.editor_state_mut()
        .editor_ui
        .open_scene_template_center(1);
    host
}

/// Wheel AND trackpad must both scroll the grid, never the canvas.
///
/// Wheel deltas route through `apply_wheel_inner` while trackpad pans route
/// through `apply_pan_gesture` — two separate ladders. The panel was wired
/// into only the first, so a two-finger scroll over the card grid moved the
/// canvas underneath while the grid sat still (reported 2026-08-02). A panel
/// present in one ladder and absent from the other looks correct in every
/// mouse test.
#[test]
fn scene_template_wheel_and_trackpad_scroll_without_moving_viewport() {
    let mut host = open_host();
    let panel_rect = host
        .scene_template_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("scene template rect");
    let point = Point2D::new(
        panel_rect.origin.x + panel_rect.size.x / 2.0,
        panel_rect.origin.y + panel_rect.size.y / 2.0,
    );
    assert!(
        SceneTemplatePanel::for_editor(host.editor_state())
            .expect("open scene template centre")
            .max_scroll(panel_rect)
            > 0.0,
        "the shipped catalogue must overflow the two-column grid"
    );
    let viewport = host.editor_state().viewport;

    assert!(host.apply_wheel(point.x, point.y, -120.0, VIEWPORT_W, VIEWPORT_H));
    let wheel_offset = host
        .editor_state()
        .editor_ui
        .scene_template_center
        .scroll
        .offset;
    assert!(wheel_offset > 0.0, "wheel must scroll the grid");
    assert_eq!(
        host.editor_state().viewport,
        viewport,
        "the canvas must not move under a wheel over the panel"
    );

    assert!(host.apply_pan_gesture(point.x, point.y, 35.0, -80.0, VIEWPORT_W, VIEWPORT_H));
    assert!(
        host.editor_state()
            .editor_ui
            .scene_template_center
            .scroll
            .offset
            > wheel_offset,
        "a trackpad vertical delta must keep scrolling the grid"
    );
    assert_eq!(
        host.editor_state().viewport,
        viewport,
        "the canvas must not move under a trackpad pan over the panel"
    );
}

/// Outside the panel both ladders fall through to the canvas as usual.
#[test]
fn scrolling_outside_the_panel_still_reaches_the_canvas() {
    let mut host = open_host();
    let panel_rect = host
        .scene_template_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("scene template rect");
    let outside = Point2D::new(panel_rect.origin.x - 20.0, panel_rect.origin.y - 20.0);
    let before = host.editor_state().viewport;

    host.apply_pan_gesture(outside.x, outside.y, 0.0, -60.0, VIEWPORT_W, VIEWPORT_H);
    assert_ne!(
        host.editor_state().viewport,
        before,
        "a pan outside the panel belongs to the canvas"
    );
    assert_eq!(
        host.editor_state()
            .editor_ui
            .scene_template_center
            .scroll
            .offset,
        0.0,
        "the grid must not scroll from a pointer outside it"
    );
}
