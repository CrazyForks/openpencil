//! Shared layout language for the settings modal: the per-tab opener and
//! the borderless list row.
//!
//! A tab opens in one of two registers. Introductory tabs (Agents, MCP)
//! get a **hero** — a 27 pt headline over a line or two of muted copy —
//! because their job is to explain a thing before you configure it.
//! Inventory tabs (System, Images, Fonts, Account) get a **heading** —
//! a glyph plus a section-sized title — because their job is to list
//! settings, and a hero there just pushes rows off the fold. Below
//! either, settings are full-width rows separated by hairlines, not
//! tinted cards: label left, control right, one row-height constant.
//!
//! Everything here measures through [`fit_text`] /
//! [`measure_settings_text`], the modal-shaped names for
//! [`crate::widgets::text_metrics`] — never `RenderBackend::measure_text`,
//! which is family-blind. See that module for why.

use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

/// The font family every string in this modal is DRAWN with. Measurement
/// has to name it too — see [`fit_text`]. Aliases the workspace-wide chrome
/// family so the modal cannot drift from the rest of the editor.
pub(super) const SETTINGS_FONT_FAMILY: &str = crate::widgets::text_metrics::CHROME_FONT_FAMILY;

pub(super) const HERO_TITLE_FONT: f32 = 27.0;
pub(super) const HERO_SUB_FONT: f32 = 13.0;
/// Baseline of the hero title, measured from the content viewport top.
const HERO_TITLE_BASELINE: f32 = 28.0;
/// Baseline of the first subtitle line, and the step between lines.
const HERO_FIRST_LINE_BASELINE: f32 = 52.0;
const HERO_LINE_STEP: f32 = 20.0;
/// Clear space between the last hero line and the first row below it.
const HERO_BOTTOM_GAP: f32 = 24.0;

/// Compact heading for the list-shaped tabs (System / Images / Fonts /
/// Account). Those pages are a settings inventory, not an introduction —
/// a 27 pt hero on them pushes the first row off the fold for no gain, so
/// they get an icon plus a section-sized title instead.
pub(super) const HEADING_TITLE_FONT: f32 = 16.0;
pub(super) const HEADING_ICON: f32 = 16.0;
const HEADING_TITLE_BASELINE: f32 = 22.0;
const HEADING_DESC_BASELINE: f32 = 42.0;
const HEADING_BOTTOM_GAP: f32 = 20.0;

pub(super) const ROW_LABEL_FONT: f32 = 15.0;
pub(super) const ROW_DESC_FONT: f32 = 12.0;
pub(super) const SECTION_TITLE_FONT: f32 = 15.0;

/// Nominal vertical metrics as a fraction of font size. Row geometry has
/// to be computable without a backend (hit-test paths have no painter),
/// so the box maths uses these instead of querying real font metrics.
/// They run slightly generous, and `ROW_VPAD` carries the slack.
pub(super) const ASCENT_RATIO: f32 = 0.78;
pub(super) const DESCENT_RATIO: f32 = 0.22;
/// Clear space above the first line's ascender and below the last line's
/// descender.
const ROW_VPAD: f32 = 15.5;
/// Space between the label's descender and the second line's ascender.
const ROW_LINE_GAP: f32 = 4.0;

const LABEL_ASCENT: f32 = ROW_LABEL_FONT * ASCENT_RATIO;
const LABEL_DESCENT: f32 = ROW_LABEL_FONT * DESCENT_RATIO;
const DESC_ASCENT: f32 = ROW_DESC_FONT * ASCENT_RATIO;
const DESC_DESCENT: f32 = ROW_DESC_FONT * DESCENT_RATIO;

/// How many lines of text a row carries. The row's BOX HEIGHT follows
/// from this — one shared `ROW_HEIGHT` for both shapes is what let a
/// two-line row's second line escape its box and collide with the row
/// under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowLines {
    /// Label only.
    One,
    /// Label plus a description or status line.
    Two,
}

/// Baseline of a row's label, measured from the row's top. Identical for
/// both shapes: a one-line row and a two-line row start their label on
/// the same line, only the box below it differs.
pub(super) const ROW_LABEL_BASELINE: f32 = ROW_VPAD + LABEL_ASCENT;
/// Baseline of a two-line row's second line.
pub(super) const ROW_SECOND_LINE_BASELINE: f32 =
    ROW_LABEL_BASELINE + LABEL_DESCENT + ROW_LINE_GAP + DESC_ASCENT;

/// Box height for a row carrying `lines` lines — ascender clearance,
/// the text itself, descender clearance.
pub(super) const fn row_height(lines: RowLines) -> f32 {
    match lines {
        RowLines::One => ROW_LABEL_BASELINE + LABEL_DESCENT + ROW_VPAD,
        RowLines::Two => ROW_SECOND_LINE_BASELINE + DESC_DESCENT + ROW_VPAD,
    }
}

/// Legacy uniform stride, kept for the lists whose rows are all
/// single-line (the MCP CLI toggles). Mixed lists must walk
/// [`row_rect_in`] instead.
pub(super) const ROW_HEIGHT: f32 = row_height(RowLines::One);
/// A section header (icon + title, optional trailing action) occupies this
/// much vertical space before its first row.
pub(super) const SECTION_HEADER_H: f32 = 32.0;
pub(super) const SECTION_GAP: f32 = 24.0;
/// Footnote line under a row list (the `*` caveat under the CLI toggles).
pub(super) const FOOTNOTE_H: f32 = 28.0;
pub(super) const FOOTNOTE_FONT: f32 = 12.0;
/// Diameter of the leading dot on a row's status line.
const STATUS_DOT: f32 = 7.0;

/// Ellipsize `text` to `max_w` measured in the family the modal actually
/// paints with. The modal-shaped name for
/// [`crate::widgets::text_metrics::fit_chrome`], which carries the full
/// account of why the family-blind call is a trap.
pub(super) fn fit_text(cx: &mut PaintCx<'_>, text: &str, max_w: f32, font_size: f32) -> String {
    crate::widgets::text_metrics::fit_chrome(cx.backend, text, max_w, font_size)
}

/// Width of `text` in the family the modal paints with — the centring
/// twin of [`fit_text`]. Centring a label on a family-blind width leaves
/// it visibly off-centre wherever the drawn family is wider.
pub(super) fn measure_settings_text(cx: &mut PaintCx<'_>, text: &str, font_size: f32) -> f32 {
    crate::widgets::text_metrics::measure_chrome(cx.backend, text, font_size)
}

/// Height of a hero block carrying `lines` subtitle lines. One line is
/// the floor; the Agents tab uses two.
pub(super) fn tab_hero_height(lines: usize) -> f32 {
    let lines = lines.max(1) as f32;
    HERO_FIRST_LINE_BASELINE + (lines - 1.0) * HERO_LINE_STEP + HERO_BOTTOM_GAP
}

/// Height of the compact heading used by the list-shaped tabs.
pub(super) fn tab_heading_height(has_desc: bool) -> f32 {
    if has_desc {
        HEADING_DESC_BASELINE + HEADING_BOTTOM_GAP
    } else {
        HEADING_TITLE_BASELINE + HEADING_BOTTOM_GAP
    }
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
    let title_text = fit_text(cx, title, content.size.x, HERO_TITLE_FONT);
    let title_layout = TextLayout::single_run(
        &title_text,
        SETTINGS_FONT_FAMILY,
        HERO_TITLE_FONT,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title_layout,
        Point2D::new(content.origin.x, content.origin.y + HERO_TITLE_BASELINE),
    );
    for (i, line) in lines.iter().enumerate() {
        let text = fit_text(cx, line, content.size.x, HERO_SUB_FONT);
        let layout = TextLayout::single_run(
            &text,
            SETTINGS_FONT_FAMILY,
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

/// Full-width row `index` in a list of uniform single-line rows.
pub(super) fn row_rect(content: Rect, top: f32, index: usize) -> Rect {
    Rect {
        origin: Point2D::new(content.origin.x, top + index as f32 * ROW_HEIGHT),
        size: Point2D::new(content.size.x, ROW_HEIGHT),
    }
}

/// Row `index` in a list whose rows differ in height. Sums the boxes
/// ahead of it, so paint and hit-test walk the same ladder and a row can
/// never be laid out at a height other than the one its content needs.
pub(super) fn row_rect_in(content: Rect, top: f32, kinds: &[RowLines], index: usize) -> Rect {
    let y = top
        + kinds
            .iter()
            .take(index)
            .map(|kind| row_height(*kind))
            .sum::<f32>();
    let height = kinds.get(index).copied().unwrap_or(RowLines::One);
    Rect {
        origin: Point2D::new(content.origin.x, y),
        size: Point2D::new(content.size.x, row_height(height)),
    }
}

/// Total height of a mixed-height row list.
pub(super) fn rows_block_height(kinds: &[RowLines]) -> f32 {
    kinds.iter().map(|kind| row_height(*kind)).sum()
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
    // Both shapes put the label on the same baseline; only a two-line row
    // adds the second one. Centring the single-line label separately is
    // exactly what made a self-drawn status line collide with it.
    let (label_baseline, desc_baseline) = (
        row.origin.y + ROW_LABEL_BASELINE,
        row.origin.y + ROW_SECOND_LINE_BASELINE,
    );
    let label_text = fit_text(cx, label, budget, ROW_LABEL_FONT);
    let label_layout = TextLayout::single_run(
        &label_text,
        SETTINGS_FONT_FAMILY,
        ROW_LABEL_FONT,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&label_layout, Point2D::new(row.origin.x, label_baseline));
    if let Some(desc) = desc {
        let desc_text = fit_text(cx, desc, budget, ROW_DESC_FONT);
        let desc_layout = TextLayout::single_run(
            &desc_text,
            SETTINGS_FONT_FAMILY,
            ROW_DESC_FONT,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&desc_layout, Point2D::new(row.origin.x, desc_baseline));
    }
}

/// Label for a row whose second line the caller paints itself (a status
/// readout, which needs its own colour and leading dot). Pins the label
/// to the two-line baseline so it does not sit centred as if it were the
/// row's only line — which is what made the MCP server row's title and
/// its "Running" line collide.
pub(super) fn paint_row_label_above_status(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    row: Rect,
    label: &str,
    reserved: f32,
) {
    let budget = (row.size.x - reserved - 16.0).max(0.0);
    let text = fit_text(cx, label, budget, ROW_LABEL_FONT);
    let layout = TextLayout::single_run(
        &text,
        SETTINGS_FONT_FAMILY,
        ROW_LABEL_FONT,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(row.origin.x, row.origin.y + ROW_LABEL_BASELINE),
    );
}

/// Compact heading for the list-shaped tabs: glyph + a section-sized
/// title, with an optional single muted line under it.
pub(super) fn paint_tab_heading(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    content: Rect,
    icon: crate::widgets::icons::Icon,
    title: &str,
    desc: Option<&str>,
) {
    crate::widgets::icons::draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            content.origin.x,
            content.origin.y + HEADING_TITLE_BASELINE - HEADING_ICON + 3.0,
        ),
        HEADING_ICON,
        theme.foreground,
        1.7,
    );
    let title_x = content.origin.x + HEADING_ICON + 10.0;
    let title_text = fit_text(
        cx,
        title,
        (content.origin.x + content.size.x - title_x).max(0.0),
        HEADING_TITLE_FONT,
    );
    let title_layout = TextLayout::single_run(
        &title_text,
        SETTINGS_FONT_FAMILY,
        HEADING_TITLE_FONT,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title_layout,
        Point2D::new(title_x, content.origin.y + HEADING_TITLE_BASELINE),
    );
    if let Some(desc) = desc {
        let desc_text = fit_text(cx, desc, content.size.x, ROW_DESC_FONT);
        let desc_layout = TextLayout::single_run(
            &desc_text,
            SETTINGS_FONT_FAMILY,
            ROW_DESC_FONT,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &desc_layout,
            Point2D::new(content.origin.x, content.origin.y + HEADING_DESC_BASELINE),
        );
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
    // Dot sits on the second line's optical centre; the text shares the
    // description baseline so a status row and a described row line up.
    cx.backend.fill_oval(
        Rect {
            origin: Point2D::new(
                row.origin.x,
                row.origin.y + ROW_SECOND_LINE_BASELINE - STATUS_DOT + 1.0,
            ),
            size: Point2D::new(STATUS_DOT, STATUS_DOT),
        },
        color,
    );
    let layout = TextLayout::single_run(
        text,
        SETTINGS_FONT_FAMILY,
        ROW_DESC_FONT,
        color.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(
            row.origin.x + STATUS_DOT + 7.0,
            row.origin.y + ROW_SECOND_LINE_BASELINE,
        ),
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
    let text = fit_text(cx, text, content.size.x, FOOTNOTE_FONT);
    let layout = TextLayout::single_run(
        &text,
        SETTINGS_FONT_FAMILY,
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

#[cfg(test)]
mod fit_tests {
    use super::*;
    use crate::widgets::agent_settings_panel::AgentSettingsPanel;
    use crate::widgets::{PaintCx, Widget};
    use crate::{Point2D, Rect, RenderBackend};
    use op_editor_core::EditorState;

    use crate::widgets::test_family_gap_backend::FamilyGapBackend;

    #[test]
    fn fit_text_measures_in_the_family_it_will_be_drawn_in() {
        let mut backend = FamilyGapBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let long = "A".repeat(200);
        let fitted = fit_text(&mut cx, &long, 100.0, 13.0);

        assert!(fitted.ends_with('…'), "an over-wide line must ellipsize");
        let painted = cx
            .backend
            .measure_text_family(&fitted, 13.0, SETTINGS_FONT_FAMILY);
        assert!(
            painted <= 100.0,
            "fitted width {painted} must fit the 100px column in the PAINTED family"
        );
    }

    #[test]
    fn agents_hero_lines_fit_the_content_column() {
        // The provider roll is the longest string the modal generates and
        // the one that shipped sheared in half; assert every hero line
        // lands inside the content column measured the way it is painted.
        let mut state = EditorState::default();
        state.editor_ui.locale = op_i18n::Locale::EnUs;
        let panel = AgentSettingsPanel::for_editor(&state);
        let rect = panel.rect(1200.0, 800.0);
        let content = crate::widgets::agent_settings_panel::content_viewport(rect);
        let mut backend = FamilyGapBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        let hero: Vec<(String, f32)> = backend
            .runs
            .iter()
            .filter(|run| {
                (run.font_size - HERO_SUB_FONT).abs() < 0.01
                    && (run.origin.x - content.origin.x).abs() < 0.01
                    && run.origin.y <= content.origin.y + tab_hero_height(2)
            })
            .map(|run| (run.text.clone(), run.font_size))
            .collect();
        assert!(!hero.is_empty(), "the Agents hero should paint muted lines");
        for (text, size) in hero {
            let w = backend.measure_text_family(&text, size, SETTINGS_FONT_FAMILY);
            assert!(
                w <= content.size.x,
                "hero line {text:?} is {w}px wide in a {}px column",
                content.size.x
            );
        }
    }

    #[test]
    fn a_row_with_a_status_line_uses_the_two_line_baselines() {
        // The MCP server row draws its status line itself. If its label
        // took the single-line centred baseline the two lines collided —
        // that is the squeeze the shipped build showed.
        let row = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(400.0, row_height(RowLines::Two)),
        };
        let mut backend = FamilyGapBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let theme = crate::theme::Theme::dark();

        paint_row_label_above_status(&mut cx, &theme, row, "MCP Server", 0.0);
        paint_row_status_line(&mut cx, row, "Running", theme.status_success);

        let label_y = backend.runs[0].origin.y;
        let status_y = backend.runs[1].origin.y;
        let leading = status_y - label_y;
        assert!(
            leading >= ROW_LABEL_FONT,
            "label and status baselines are {leading}px apart — under one label line-height they read as one blob"
        );
        // Both lines have to live inside the row box, ascender to descender.
        assert!(label_y - ROW_LABEL_FONT >= row.origin.y);
        assert!(status_y + ROW_DESC_FONT * 0.3 <= row.origin.y + row.size.y);
    }

    #[test]
    fn described_rows_and_status_rows_share_one_rhythm() {
        let row = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(400.0, row_height(RowLines::Two)),
        };
        let theme = crate::theme::Theme::dark();

        let mut a = FamilyGapBackend::default();
        let mut cx = PaintCx { backend: &mut a };
        paint_row_label(&mut cx, &theme, row, "Label", Some("Description"), 0.0);

        let mut b = FamilyGapBackend::default();
        let mut cx = PaintCx { backend: &mut b };
        paint_row_label_above_status(&mut cx, &theme, row, "Label", 0.0);
        paint_row_status_line(&mut cx, row, "Running", theme.status_success);

        assert_eq!(
            a.runs[0].origin.y, b.runs[0].origin.y,
            "label baselines must agree"
        );
        assert_eq!(
            a.runs[1].origin.y, b.runs[1].origin.y,
            "second-line baselines must agree"
        );
    }
}
