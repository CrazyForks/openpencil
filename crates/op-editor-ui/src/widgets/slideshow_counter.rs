//! Slide counter shown while Preview presents a deck.
//!
//! A presenter needs to know where they are in the deck, so a small pill in
//! the bottom-right corner carries the position. The label is bare numerals
//! and a slash (`3 / 6`) — it reads identically in every locale, so the
//! overlay needs no translation and can never surface an untranslated key.

use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout, Theme};

const FONT_SIZE: f32 = 13.0;
const PAD_X: f32 = 12.0;
const HEIGHT: f32 = 28.0;
const RADIUS: f32 = 14.0;
/// Clear of the StatusBar's own bottom-right cluster, which the editor
/// hides in preview but whose corner this still borrows.
const MARGIN: f32 = 16.0;

pub struct SlideshowCounter<'a> {
    /// Already-formatted position, e.g. `"3 / 6"` — see
    /// `op_editor_core::preview_slideshow::SlideshowState::counter_label`.
    pub label: &'a str,
}

impl SlideshowCounter<'_> {
    /// Pill rect in the bottom-right corner of `canvas`.
    ///
    /// The width follows the measured label so a two-digit deck (`10 / 12`)
    /// does not overflow a fixed pill.
    pub fn pill_rect(canvas: Rect, text_width: f32) -> Rect {
        let width = text_width + PAD_X * 2.0;
        Rect {
            origin: Point2D::new(
                canvas.origin.x + canvas.size.x - width - MARGIN,
                canvas.origin.y + canvas.size.y - HEIGHT - MARGIN,
            ),
            size: Point2D::new(width, HEIGHT),
        }
    }

    pub fn paint(&self, cx: &mut PaintCx<'_>, canvas: Rect, theme: &Theme) {
        let text_width = cx.backend.measure_text(self.label, FONT_SIZE);
        let pill = Self::pill_rect(canvas, text_width);
        cx.backend.fill_round_rect(pill, RADIUS, theme.card);
        cx.backend
            .stroke_round_rect(pill, RADIUS, theme.border, 1.0);
        let label = TextLayout::single_run(
            self.label,
            "system-ui",
            FONT_SIZE,
            theme.foreground.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &label,
            Point2D::new(
                pill.origin.x + (pill.size.x - text_width) / 2.0,
                pill.origin.y + pill.size.y / 2.0 + FONT_SIZE / 2.0 - 1.5,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas() -> Rect {
        Rect {
            origin: Point2D::new(100.0, 40.0),
            size: Point2D::new(1000.0, 700.0),
        }
    }

    #[test]
    fn the_pill_sits_in_the_bottom_right_corner_and_follows_the_label_width() {
        let narrow = SlideshowCounter::pill_rect(canvas(), 30.0);
        assert_eq!(narrow.size.x, 30.0 + 24.0);
        assert_eq!(narrow.size.y, 28.0);
        assert!((narrow.origin.x - (1100.0 - 54.0 - 16.0)).abs() < 0.5);
        assert!((narrow.origin.y - (740.0 - 28.0 - 16.0)).abs() < 0.5);

        // A wider label grows leftwards, keeping the right margin fixed.
        let wide = SlideshowCounter::pill_rect(canvas(), 60.0);
        let right_edge = |rect: Rect| rect.origin.x + rect.size.x;
        assert!((right_edge(wide) - right_edge(narrow)).abs() < 0.5);
        assert!(wide.origin.x < narrow.origin.x);
    }
}
