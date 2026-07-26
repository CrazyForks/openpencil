//! Modal chrome: the close button and the sidebar navigation rows.
//!
//! Split out of `agent_settings_panel_tests.rs` to keep that file under the
//! 800-line cap.

use super::*;

#[test]
fn close_button_paints_after_scrollable_content() {
    let state = EditorState::default();
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let close_origin = Point2D::new(rect.origin.x + rect.size.x - 32.0, rect.origin.y + 16.0);
    let close_idx = backend
        .icon_strokes
        .iter()
        .find_map(|(at, size, idx)| {
            ((at.x - close_origin.x).abs() < 0.01
                && (at.y - close_origin.y).abs() < 0.01
                && (*size - 16.0).abs() < 0.01)
                .then_some(*idx)
        })
        .expect("close icon should paint");
    let restore_idx = backend
        .ops
        .iter()
        .rposition(|op| *op == "restore")
        .expect("content clip should restore");

    assert!(
        close_idx > restore_idx,
        "close button must paint above clipped, scrollable content"
    );
}

#[test]
fn close_button_hover_paints_visible_wash() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.hover_agent_settings_close = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let close = Rect {
        origin: Point2D::new(rect.origin.x + rect.size.x - 32.0, rect.origin.y + 16.0),
        size: Point2D::new(16.0, 16.0),
    };
    assert!(
        backend
            .round_fills
            .iter()
            .any(|(fill, color)| *fill == close && color_eq(*color, panel.theme.button_hover)),
        "hovered close button should paint a visible hover wash"
    );
}

#[test]
fn pressed_close_button_uses_shared_button_feedback() {
    let mut state = EditorState::default();
    state.editor_ui.pressed_button =
        Some(ButtonPressTarget::AgentSettings(AgentSettingsButton::Close));
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let close = Rect {
        origin: Point2D::new(rect.origin.x + rect.size.x - 32.0, rect.origin.y + 16.0),
        size: Point2D::new(16.0, 16.0),
    };
    let expected = panel
        .theme
        .button_hover
        .with_alpha(panel.theme.button_hover.a * 1.8);
    assert!(
        backend
            .round_fills
            .iter()
            .any(|(fill, color)| *fill == close && color_eq(*color, expected)),
        "pressed close button should paint the shared pressed feedback token"
    );
}

#[test]
fn agents_nav_icon_uses_ts_pen_glyph_not_pencil() {
    const PEN_PATH: &str =
        "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z";
    let state = EditorState::default();
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let agents_nav_icon = Point2D::new(rect.origin.x + 20.0, rect.origin.y + 63.0);
    let strokes: Vec<_> = backend
        .svg_strokes
        .iter()
        .filter(|(_, at, size)| {
            (at.x - agents_nav_icon.x).abs() < 0.01
                && (at.y - agents_nav_icon.y).abs() < 0.01
                && (*size - 14.0).abs() < 0.01
        })
        .collect();

    assert_eq!(strokes.len(), 1, "TS settings sidebar uses lucide Pen");
    assert_eq!(strokes[0].0, PEN_PATH);
}

#[test]
fn sidebar_nav_uses_ts_compact_rows() {
    let state = EditorState::default();
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let x = rect.origin.x + 100.0;

    assert_eq!(
        panel.hit_test(rect, crate::Point2D::new(x, rect.origin.y + 70.0)),
        AgentSettingsHit::SelectTab(AgentSettingsTab::Agents)
    );
    assert_eq!(
        panel.hit_test(rect, crate::Point2D::new(x, rect.origin.y + 100.0)),
        AgentSettingsHit::SelectTab(AgentSettingsTab::Mcp)
    );
}
