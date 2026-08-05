//! Shared layout language for the settings modal: the per-tab hero block
//! and the borderless list row.
//!
//! Every tab opens with a hero (large title + one or more muted lines),
//! then lists its settings as full-width rows separated by hairlines —
//! not tinted cards. The row carries its label on the left and its
//! control (switch / segmented / button) right-aligned, so a tab's
//! vertical rhythm is a single constant rather than per-section maths.

use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

pub(super) const HERO_TITLE_FONT: f32 = 27.0;
pub(super) const HERO_SUB_FONT: f32 = 13.0;
/// Baseline of the hero title, measured from the content viewport top.
const HERO_TITLE_BASELINE: f32 = 30.0;
/// Baseline of the first subtitle line, and the step between lines.
const HERO_FIRST_LINE_BASELINE: f32 = 60.0;
const HERO_LINE_STEP: f32 = 22.0;
/// Clear space between the last hero line and the first row below it.
const HERO_BOTTOM_GAP: f32 = 30.0;

pub(super) const ROW_HEIGHT: f32 = 54.0;
pub(super) const ROW_LABEL_FONT: f32 = 15.0;
pub(super) const ROW_DESC_FONT: f32 = 12.0;
pub(super) const SECTION_TITLE_FONT: f32 = 15.0;
/// A section header (icon + title, optional trailing action) occupies this
/// much vertical space before its first row.
pub(super) const SECTION_HEADER_H: f32 = 40.0;
pub(super) const SECTION_GAP: f32 = 32.0;
/// Footnote line under a row list (the `*` caveat under the CLI toggles).
pub(super) const FOOTNOTE_H: f32 = 32.0;
pub(super) const FOOTNOTE_FONT: f32 = 12.0;
/// Diameter of the leading dot on a row's status line.
const STATUS_DOT: f32 = 7.0;

/// Height of a hero block carrying `lines` subtitle lines. One line is
/// the floor; the Agents tab uses two.
pub(super) fn tab_hero_height(lines: usize) -> f32 {
    let lines = lines.max(1) as f32;
    HERO_FIRST_LINE_BASELINE + (lines - 1.0) * HERO_LINE_STEP + HERO_BOTTOM_GAP
}

/// Paint a tab's hero block at the top of `content`. Lines are already
/// localized; each is ellipsized to the content width so a long
/// translation can't run past the modal edge.
pub(super) fn paint_tab_hero(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    content: Rect,
    title: &str,
    lines: &[&str],
) {
    let title_text = crate::util::ellipsize_to_width(title, content.size.x, |s| {
        cx.backend.measure_text(s, HERO_TITLE_FONT)
    });
    let title_layout = TextLayout::single_run(
        &title_text,
        "system-ui",
        HERO_TITLE_FONT,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title_layout,
        Point2D::new(content.origin.x, content.origin.y + HERO_TITLE_BASELINE),
    );
    for (i, line) in lines.iter().enumerate() {
        let text = crate::util::ellipsize_to_width(line, content.size.x, |s| {
            cx.backend.measure_text(s, HERO_SUB_FONT)
        });
        let layout = TextLayout::single_run(
            &text,
            "system-ui",
            HERO_SUB_FONT,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &layout,
            Point2D::new(
                content.origin.x,
                content.origin.y + HERO_FIRST_LINE_BASELINE + i as f32 * HERO_LINE_STEP,
            ),
        );
    }
}

/// Full-width row `index` in a list starting at `top`.
pub(super) fn row_rect(content: Rect, top: f32, index: usize) -> Rect {
    Rect {
        origin: Point2D::new(content.origin.x, top + index as f32 * ROW_HEIGHT),
        size: Point2D::new(content.size.x, ROW_HEIGHT),
    }
}

/// Right-aligned, vertically centred control slot inside `row`.
pub(super) fn row_control_rect(row: Rect, w: f32, h: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            row.origin.x + row.size.x - w,
            row.origin.y + (row.size.y - h) / 2.0,
        ),
        size: Point2D::new(w, h),
    }
}

/// Hairline along a row's bottom edge — the separator that replaces the
/// old per-setting card fill. Callers skip it on the last row of a list.
pub(super) fn paint_row_hairline(cx: &mut PaintCx<'_>, theme: &Theme, row: Rect) {
    let y = row.origin.y + row.size.y;
    cx.backend.stroke_line(
        Point2D::new(row.origin.x, y),
        Point2D::new(row.origin.x + row.size.x, y),
        theme.border,
        1.0,
    );
}

/// Row label, optionally with a muted description under it. `reserved` is
/// the width the row's control occupies on the right, so the text
/// ellipsizes before it collides.
pub(super) fn paint_row_label(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    row: Rect,
    label: &str,
    desc: Option<&str>,
    reserved: f32,
) {
    let budget = (row.size.x - reserved - 16.0).max(0.0);
    let (label_baseline, desc_baseline) = match desc {
        Some(_) => (row.origin.y + 23.0, row.origin.y + 41.0),
        None => (row.origin.y + row.size.y / 2.0 + ROW_LABEL_FONT * 0.36, 0.0),
    };
    let label_text = crate::util::ellipsize_to_width(label, budget, |s| {
        cx.backend.measure_text(s, ROW_LABEL_FONT)
    });
    let label_layout = TextLayout::single_run(
        &label_text,
        "system-ui",
        ROW_LABEL_FONT,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&label_layout, Point2D::new(row.origin.x, label_baseline));
    if let Some(desc) = desc {
        let desc_text = crate::util::ellipsize_to_width(desc, budget, |s| {
            cx.backend.measure_text(s, ROW_DESC_FONT)
        });
        let desc_layout = TextLayout::single_run(
            &desc_text,
            "system-ui",
            ROW_DESC_FONT,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&desc_layout, Point2D::new(row.origin.x, desc_baseline));
    }
}

/// Leading-dot status line in a row's description slot. Used wherever a
/// row answers "is this thing on" — the MCP server row and the System
/// auto-update row — so both read identically.
pub(super) fn paint_row_status_line(
    cx: &mut PaintCx<'_>,
    row: Rect,
    text: &str,
    color: crate::Color,
) {
    cx.backend.fill_oval(
        Rect {
            origin: Point2D::new(row.origin.x, row.origin.y + 36.0),
            size: Point2D::new(STATUS_DOT, STATUS_DOT),
        },
        color,
    );
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        ROW_DESC_FONT,
        color.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(row.origin.x + STATUS_DOT + 7.0, row.origin.y + 43.0),
    );
}

/// Muted footnote under a row list.
pub(super) fn paint_footnote(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    content: Rect,
    y: f32,
    text: &str,
) {
    let text = crate::util::ellipsize_to_width(text, content.size.x, |s| {
        cx.backend.measure_text(s, FOOTNOTE_FONT)
    });
    let layout = TextLayout::single_run(
        &text,
        "system-ui",
        FOOTNOTE_FONT,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&layout, Point2D::new(content.origin.x, y + 18.0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_hero_constant_matches_the_shared_two_line_hero() {
        // The Agents tab publishes its hero height as a constant so host
        // tests can anchor to it; every other tab derives one from
        // `tab_hero_height`. They must be the same block.
        assert_eq!(
            crate::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT,
            tab_hero_height(2)
        );
    }

    #[test]
    fn hero_height_grows_one_line_step_at_a_time() {
        assert_eq!(tab_hero_height(2) - tab_hero_height(1), HERO_LINE_STEP);
        // Zero lines is meaningless — the floor is a single line.
        assert_eq!(tab_hero_height(0), tab_hero_height(1));
    }
}
