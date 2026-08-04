//! Deck filmstrip — the page navigator a slide deck floats at the bottom
//! of the canvas.
//!
//! One chip per slide, in page order, with the slide the camera is looking
//! at highlighted. Clicking a chip frames that board; dragging a chip
//! reorders the deck. That second half is the point of the strip existing
//! at all: the presentation / export order IS the document's top-level
//! child order, and before this there was no way to see that order, let
//! alone change it, without dragging rows in the layer panel.
//!
//! Chips carry a page number and a name, NOT a thumbnail. A live
//! thumbnail means a second render of every board on every frame, keyed by
//! a cache whose invalidation has burned us before; a numbered chip is
//! navigable today and can grow a picture later without moving any of the
//! geometry below.
//!
//! Every rect here is derived WITHOUT measuring text — chips are a fixed
//! width — so the host's hit-test and the paint pass compute the same
//! layout from the same three inputs (canvas rect, chip count, active
//! index). Measurement only decides where a name is cut, never where a
//! chip is.

use op_editor_core::FilmstripDrag;

use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout, Theme};

/// Fixed chip size. Uniform tiles keep the strip readable as a sequence
/// (the eye counts positions, not widths) and make the drag arithmetic a
/// division instead of a scan.
pub const CHIP_W: f32 = 104.0;
pub const CHIP_H: f32 = 30.0;
const CHIP_GAP: f32 = 6.0;
const CHIP_RADIUS: f32 = 6.0;
/// Padding between the pill's edge and the chip band.
const STRIP_PAD: f32 = 7.0;
/// Outer height of the pill.
pub const FILMSTRIP_HEIGHT: f32 = CHIP_H + STRIP_PAD * 2.0;
const STRIP_RADIUS: f32 = FILMSTRIP_HEIGHT / 2.0;
/// Gap between the pill and the bottom edge of the canvas region. Matches
/// the presenting toolbar so the two never read as different furniture.
const BOTTOM_MARGIN: f32 = 20.0;
/// Minimum clearance from the canvas's left / right edges.
const SIDE_MARGIN: f32 = 24.0;
const FONT_SIZE: f32 = 11.5;
const NUMBER_GAP: f32 = 6.0;
const CHIP_PAD_X: f32 = 8.0;
/// The canvas shows through the pill, like the presenting toolbar's.
const PILL_ALPHA: f32 = 0.94;
/// Wash strength for the chip of the slide the camera is on.
const ACTIVE_FILL_ALPHA: f32 = 0.20;
/// The chip being dragged stays visible at its origin as a ghost, so the
/// strip still reads as N slides while one of them is in the air.
const GHOST_ALPHA: f32 = 0.30;
/// Width of the bar showing where a dropped chip would land.
const DROP_BAR_W: f32 = 2.0;

/// One slide's entry in the strip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilmstripChip {
    /// Node id of the board, used to frame it on click.
    pub id: String,
    /// The board's authored name. Empty is fine — the page number alone
    /// still identifies the slide.
    pub name: String,
}

/// Where the strip and its chips sit for a given canvas.
///
/// Built once per event / per paint and shared by both, which is what
/// keeps a chip's painted rect and its click target identical.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilmstripLayout {
    /// Outer pill.
    pub strip: Rect,
    /// The band chips are laid out in and clipped to.
    pub inner: Rect,
    /// How far the chip row is scrolled left, for a deck too long to fit.
    pub offset: f32,
    pub count: usize,
}

impl FilmstripLayout {
    /// Lay the strip out at the bottom centre of `canvas`.
    ///
    /// `None` when there is nothing to show — no slides, or a canvas too
    /// narrow to hold even one chip. A strip that cannot show a chip is
    /// worse than no strip: it is a bar that eats clicks and explains
    /// nothing.
    pub fn new(canvas: Rect, count: usize, active: Option<usize>) -> Option<Self> {
        if count == 0 {
            return None;
        }
        let available = canvas.size.x - SIDE_MARGIN * 2.0;
        let min_width = CHIP_W + STRIP_PAD * 2.0;
        if available < min_width || canvas.size.y < FILMSTRIP_HEIGHT + BOTTOM_MARGIN {
            return None;
        }
        let width = (content_width(count) + STRIP_PAD * 2.0).min(available);
        let strip = Rect {
            origin: Point2D::new(
                canvas.origin.x + (canvas.size.x - width) / 2.0,
                canvas.origin.y + canvas.size.y - FILMSTRIP_HEIGHT - BOTTOM_MARGIN,
            ),
            size: Point2D::new(width, FILMSTRIP_HEIGHT),
        };
        let inner = Rect {
            origin: Point2D::new(strip.origin.x + STRIP_PAD, strip.origin.y + STRIP_PAD),
            size: Point2D::new(width - STRIP_PAD * 2.0, CHIP_H),
        };
        Some(Self {
            strip,
            inner,
            offset: scroll_offset(count, active, inner.size.x),
            count,
        })
    }

    /// Rect of chip `index`, in screen space. Not clamped to the strip: a
    /// chip scrolled out of view still has a position, which is what lets
    /// the drag arithmetic work on chips the user cannot see.
    pub fn chip_rect(&self, index: usize) -> Rect {
        Rect {
            origin: Point2D::new(
                self.inner.origin.x - self.offset + index as f32 * (CHIP_W + CHIP_GAP),
                self.inner.origin.y,
            ),
            size: Point2D::new(CHIP_W, CHIP_H),
        }
    }

    /// The chips with any part inside the band, paired with their rects.
    pub fn visible_chips(&self) -> Vec<(usize, Rect)> {
        (0..self.count)
            .map(|index| (index, self.chip_rect(index)))
            .filter(|(_, rect)| {
                rect.origin.x + rect.size.x > self.inner.origin.x
                    && rect.origin.x < self.inner.origin.x + self.inner.size.x
            })
            .collect()
    }

    /// Which chip `point` lands on. Only the part of a chip inside the
    /// band counts — a half-scrolled chip must not be clickable where it
    /// is not painted.
    pub fn chip_at(&self, point: Point2D) -> Option<usize> {
        if !contains(self.inner, point) {
            return None;
        }
        self.visible_chips()
            .into_iter()
            .find_map(|(index, rect)| contains(rect, point).then_some(index))
    }

    /// Whether `point` is anywhere on the pill, including its padding and
    /// the gaps between chips. The press path uses this to decide the
    /// press is the strip's business and must not reach the canvas.
    pub fn contains_point(&self, point: Point2D) -> bool {
        contains(self.strip, point)
    }

    /// The slot a chip dropped at `pointer_x` would be inserted before,
    /// in the range `0..=count`. Counted from chip CENTRES, so the drop
    /// flips as the dragged chip passes the middle of its neighbour.
    pub fn insertion_slot(&self, pointer_x: f32) -> usize {
        let content_x = pointer_x - (self.inner.origin.x - self.offset);
        (0..self.count)
            .filter(|index| {
                let centre = *index as f32 * (CHIP_W + CHIP_GAP) + CHIP_W / 2.0;
                content_x > centre
            })
            .count()
    }

    /// Screen x of the bar marking `slot`, clamped into the band so it
    /// stays visible at either end of a scrolled strip.
    fn drop_bar_x(&self, slot: usize) -> f32 {
        let content_x = if slot == 0 {
            -CHIP_GAP / 2.0
        } else {
            (slot - 1) as f32 * (CHIP_W + CHIP_GAP) + CHIP_W + CHIP_GAP / 2.0
        };
        (self.inner.origin.x - self.offset + content_x).clamp(
            self.inner.origin.x,
            self.inner.origin.x + self.inner.size.x - DROP_BAR_W,
        )
    }
}

/// Total width of `count` chips laid end to end.
pub fn content_width(count: usize) -> f32 {
    if count == 0 {
        return 0.0;
    }
    count as f32 * CHIP_W + (count - 1) as f32 * CHIP_GAP
}

/// How far to scroll the chip row so the active chip is on screen.
///
/// Centres the active chip when the row overflows, clamped to the ends so
/// the strip never shows blank space beside its first or last chip.
fn scroll_offset(count: usize, active: Option<usize>, band_width: f32) -> f32 {
    let content = content_width(count);
    if content <= band_width {
        return 0.0;
    }
    let active = active.unwrap_or(0).min(count.saturating_sub(1));
    let centre = active as f32 * (CHIP_W + CHIP_GAP) + CHIP_W / 2.0;
    (centre - band_width / 2.0).clamp(0.0, content - band_width)
}

/// Where a chip dragged from `from` and dropped before `slot` ends up in
/// the child array, or `None` when the drop changes nothing.
///
/// `slot` counts gaps in the CURRENT order, so dropping either side of
/// the dragged chip is a no-op; every other slot converts to a
/// post-removal index, which is what `EditorCommand::MoveNode` takes.
pub fn reorder_target_index(from: usize, slot: usize) -> Option<usize> {
    if slot == from || slot == from + 1 {
        return None;
    }
    Some(if slot > from { slot - 1 } else { slot })
}

pub struct DeckFilmstrip<'a> {
    pub chips: &'a [FilmstripChip],
    /// The slide the camera is looking at, if any board is near enough.
    pub active: Option<usize>,
    pub hover: Option<usize>,
    pub drag: Option<FilmstripDrag>,
}

impl DeckFilmstrip<'_> {
    pub fn paint(&self, cx: &mut PaintCx<'_>, layout: &FilmstripLayout, theme: &Theme) {
        cx.backend.fill_round_rect(
            layout.strip,
            STRIP_RADIUS,
            Color {
                a: PILL_ALPHA,
                ..theme.popover
            },
        );
        cx.backend
            .stroke_round_rect(layout.strip, STRIP_RADIUS, theme.border, 1.0);

        // Chips are clipped to the band so a scrolled deck's overflow ends
        // at the pill's edge instead of painting across the canvas.
        cx.backend.save();
        cx.backend.clip_rect(layout.inner);
        let dragging = self.drag.filter(drag_is_live);
        for (index, rect) in layout.visible_chips() {
            let ghosted = dragging.is_some_and(|drag| drag.from == index);
            self.paint_chip(cx, theme, index, rect, ghosted);
        }
        if let Some(drag) = dragging {
            let slot = layout.insertion_slot(drag.pointer_x);
            cx.backend.fill_rect(
                Rect {
                    origin: Point2D::new(layout.drop_bar_x(slot), layout.inner.origin.y),
                    size: Point2D::new(DROP_BAR_W, CHIP_H),
                },
                theme.primary,
            );
            // The chip in flight rides the cursor, painted last so it sits
            // over its neighbours.
            let carried = Rect {
                origin: Point2D::new(drag.pointer_x - CHIP_W / 2.0, layout.inner.origin.y - 2.0),
                size: Point2D::new(CHIP_W, CHIP_H),
            };
            self.paint_chip(cx, theme, drag.from, carried, false);
        }
        cx.backend.restore();
    }

    fn paint_chip(
        &self,
        cx: &mut PaintCx<'_>,
        theme: &Theme,
        index: usize,
        rect: Rect,
        ghosted: bool,
    ) {
        let active = self.active == Some(index);
        let alpha = if ghosted { GHOST_ALPHA } else { 1.0 };
        let fill = if active {
            Color {
                a: ACTIVE_FILL_ALPHA * alpha,
                ..theme.primary
            }
        } else if self.hover == Some(index) {
            fade(theme.button_hover, alpha)
        } else {
            fade(theme.muted, alpha)
        };
        cx.backend.fill_round_rect(rect, CHIP_RADIUS, fill);
        if active {
            cx.backend
                .stroke_round_rect(rect, CHIP_RADIUS, fade(theme.primary, alpha), 1.0);
        }

        let baseline = rect.origin.y + CHIP_H / 2.0 + FONT_SIZE / 2.0 - 1.5;
        let number = format!("{}", index + 1);
        let number_color = if active {
            fade(theme.primary, alpha)
        } else {
            fade(theme.muted_foreground, alpha)
        };
        cx.backend.draw_text(
            &TextLayout::single_run(
                &number,
                "system-ui",
                FONT_SIZE,
                number_color.to_jian(),
                Point2D::ZERO,
            )
            .with_font_weight(600),
            Point2D::new(rect.origin.x + CHIP_PAD_X, baseline),
        );

        let Some(chip) = self.chips.get(index) else {
            return;
        };
        if chip.name.is_empty() {
            return;
        }
        let number_w = cx.backend.measure_text(&number, FONT_SIZE);
        let name_x = rect.origin.x + CHIP_PAD_X + number_w + NUMBER_GAP;
        let budget = rect.origin.x + rect.size.x - CHIP_PAD_X - name_x;
        let name = crate::widgets::file_menu::truncate_to_width(cx, &chip.name, FONT_SIZE, budget);
        if name.is_empty() {
            return;
        }
        cx.backend.draw_text(
            &TextLayout::single_run(
                &name,
                "system-ui",
                FONT_SIZE,
                fade(theme.foreground, alpha).to_jian(),
                Point2D::ZERO,
            ),
            Point2D::new(name_x, baseline),
        );
    }
}

/// Whether a drag has travelled far enough to be a reorder rather than a
/// click that has not been released yet.
pub fn drag_is_live(drag: &FilmstripDrag) -> bool {
    (drag.pointer_x - drag.press_x).abs() > DRAG_THRESHOLD_PX
}

/// How far a press has to travel before it stops being a click. Matches
/// the canvas node-drag threshold so the two gestures feel the same.
pub const DRAG_THRESHOLD_PX: f32 = 3.0;

fn fade(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}

fn contains(rect: Rect, point: Point2D) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.x
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.y
}

#[cfg(test)]
#[path = "deck_filmstrip_tests.rs"]
mod deck_filmstrip_tests;
