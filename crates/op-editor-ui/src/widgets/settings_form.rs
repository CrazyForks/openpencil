//! Shared form-row helpers for the Agent settings sections.
//!
//! `agent_settings_acp.rs` and `agent_settings_builtin.rs` paint the
//! same card-form chrome (section subtitle / empty hint, field label +
//! input box, hover action button, card hit walk); the shared bodies
//! live here once and both sections delegate.

use crate::theme::Theme;
use crate::widgets::agent_settings_caret::paint_settings_input_view;
use crate::widgets::button::paint_ghost_button_feedback;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::editor_ui_state::EditorUiState;

/// Section subtitle / empty-hint row advances — must stay in sync with
/// the per-section layout constants (both sections use 28 / 64 today).
const SUBTITLE_H: f32 = 28.0;
const EMPTY_H: f32 = 64.0;

/// Single-run "system-ui" text draw at an absolute baseline position.
pub(crate) fn draw_text(cx: &mut PaintCx<'_>, text: &str, size: f32, color: Color, x: f32, y: f32) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        size,
        (color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y));
}

/// Trim `value` with a `...` suffix until it measures within `max_w`.
pub(crate) fn ellipsize(cx: &mut PaintCx<'_>, value: &str, max_w: f32, size: f32) -> String {
    if cx.backend.measure_text(value, size) <= max_w {
        return value.to_string();
    }
    let mut out = value.to_string();
    while !out.is_empty() && cx.backend.measure_text(&format!("{out}..."), size) > max_w {
        out.pop();
    }
    format!("{out}...")
}

/// 12px muted section subtitle; returns the y advanced past the row.
pub(crate) fn paint_subtitle(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    text: &str,
    x: f32,
    y: f32,
    max_w: f32,
) -> f32 {
    let shown = ellipsize(cx, text, max_w, 12.0);
    draw_text(cx, &shown, 12.0, theme.muted_foreground, x, y + 16.0);
    y + SUBTITLE_H
}

/// Centered 13px muted empty-state hint; returns the advanced y.
pub(crate) fn paint_empty(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    text: &str,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let shown = ellipsize(cx, text, w, 13.0);
    let text_w = cx.backend.measure_text(&shown, 13.0);
    draw_text(
        cx,
        &shown,
        13.0,
        theme.muted_foreground,
        x + (w - text_w) / 2.0,
        y + 44.0,
    );
    y + EMPTY_H
}

/// Hover-revealed 24px icon action (edit / remove) with the shared
/// ghost-button hover/press wash behind a 12px glyph.
pub(crate) fn paint_action(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    icon: Icon,
    color: Color,
    pressed: bool,
) {
    paint_ghost_button_feedback(cx.backend, theme, rect, true, pressed);
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(rect.origin.x + 6.0, rect.origin.y + 6.0),
        12.0,
        color,
        1.4,
    );
}

/// Shared form-field chrome: the 11px muted label, the focus-aware
/// input fill/stroke, and — when focused — the live settings input
/// view. Returns `true` when the focused view was painted so the
/// caller can skip its static value text; the unfocused value paint
/// stays section-specific (placeholder/masking rules differ).
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_field_frame(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    label: &str,
    label_x: f32,
    label_y: f32,
    input: Rect,
    focused: bool,
    placeholder: &str,
    now_ms: u64,
) -> bool {
    draw_text(cx, label, 11.0, theme.muted_foreground, label_x, label_y);
    cx.backend.fill_round_rect(
        input,
        6.0,
        if focused {
            theme.background
        } else {
            theme.card
        },
    );
    cx.backend.stroke_round_rect(
        input,
        6.0,
        if focused { theme.primary } else { theme.border },
        1.0,
    );
    if focused {
        paint_settings_input_view(
            cx,
            theme,
            ui,
            input,
            11.0,
            6.0,
            input.origin.y + 16.0,
            now_ms,
            placeholder,
        );
        return true;
    }
    false
}

/// Walk a vertical card stack (`heights`, separated by `gap`) starting
/// at `start_y` and return the index of the card containing `point`.
pub(crate) fn card_index_at(
    x: f32,
    w: f32,
    start_y: f32,
    gap: f32,
    heights: impl Iterator<Item = f32>,
    point: Point2D,
) -> Option<usize> {
    let mut card_y = start_y;
    for (index, h) in heights.enumerate() {
        let card = Rect {
            origin: Point2D::new(x, card_y),
            size: Point2D::new(w, h),
        };
        if (card).contains(point) {
            return Some(index);
        }
        card_y += h + gap;
    }
    None
}
