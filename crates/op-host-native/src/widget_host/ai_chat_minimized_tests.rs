//! The minimized AI chat bar: placement inside the canvas region and
//! the single click that brings the panel back.

use super::WidgetHostNative;
use op_editor_ui::widgets::{AI_CHAT_MINIMIZED_HEIGHT, AI_CHAT_MINIMIZED_WIDTH};

const VIEWPORT: (f32, f32) = (1200.0, 800.0);

fn minimized_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().chat.minimize();
    host
}

#[test]
fn the_bar_docks_to_the_canvas_bottom_left_at_its_own_size() {
    let host = minimized_host();
    let (cx0, cy0, _cw, ch) = host.canvas_region(VIEWPORT.0, VIEWPORT.1);

    let bar = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("bar placed");

    assert_eq!(bar.size.x, AI_CHAT_MINIMIZED_WIDTH);
    assert_eq!(bar.size.y, AI_CHAT_MINIMIZED_HEIGHT);
    assert_eq!(bar.origin.x, cx0 + 12.0);
    assert_eq!(bar.origin.y, cy0 + ch - AI_CHAT_MINIMIZED_HEIGHT - 12.0);
}

#[test]
fn the_bar_ignores_a_dragged_panel_position_and_the_top_half_of_the_anchor() {
    // Minimized is a dock, not a floating window: a position left over
    // from dragging the expanded panel must not lift the bar off the
    // canvas floor.
    let mut host = minimized_host();
    {
        let state = host.editor_state_mut();
        state.chat.panel_position = Some((600.0, 100.0));
        state.chat.anchor = op_editor_core::ChatAnchor::TopRight;
    }
    let (cx0, cy0, cw, ch) = host.canvas_region(VIEWPORT.0, VIEWPORT.1);

    let bar = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("bar placed");

    assert_eq!(bar.origin.y, cy0 + ch - AI_CHAT_MINIMIZED_HEIGHT - 12.0);
    // The anchor still picks the side, so a right-parked panel stays right.
    assert_eq!(bar.origin.x, cx0 + cw - AI_CHAT_MINIMIZED_WIDTH - 12.0);
}

#[test]
fn expanding_restores_the_full_panel_and_focuses_the_input() {
    let mut host = minimized_host();
    let bar = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("bar placed");

    // Anywhere on the bar — including over the model label, which is not
    // a control in this form.
    assert!(host.apply_click(
        bar.origin.x + bar.size.x - 90.0,
        bar.origin.y + bar.size.y / 2.0,
        VIEWPORT.0,
        VIEWPORT.1
    ));

    assert!(!host.editor_state().chat.is_minimized());
    assert!(
        host.editor_state().chat.focused,
        "the bar reads as an input, so expanding hands over the caret"
    );
    let panel = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("panel placed");
    assert!(panel.size.y > AI_CHAT_MINIMIZED_HEIGHT);
}

#[test]
fn a_press_over_the_vacated_panel_area_reaches_the_canvas() {
    // The minimized form must not keep claiming the expanded panel's
    // footprint — the panel used to reach this far up the canvas, and
    // everything outside the bar now belongs to the canvas again.
    let mut host = minimized_host();
    let bar = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("bar placed");
    let vacated_y = bar.origin.y - 120.0;

    assert!(host.apply_press(bar.origin.x + 40.0, vacated_y, VIEWPORT.0, VIEWPORT.1));

    assert!(
        host.editor_state().chat.is_minimized(),
        "a press over the vacated panel area must not expand the chat"
    );
}
