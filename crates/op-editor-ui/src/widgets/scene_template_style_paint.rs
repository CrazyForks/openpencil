//! Asset Center → Styles tab painting.
//!
//! Split from `scene_template_panel_paint.rs` so neither file has to carry
//! both card shapes. A template card is a picture with a caption; a style
//! card is a palette with a name, and until previews are baked (M2) the
//! colour band is the whole picture — it is what makes two guides
//! distinguishable at a glance.

use super::asset_center_style_cards::StyleGuideCard;
use super::scene_template_panel::{SceneTemplatePanel, STYLE_CARD_H, STYLE_SWATCH_H};
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::prompt_center_panel::estimated_text_width;
use crate::widgets::scene_template_panel_paint::truncate_to_width;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};

const CARD_RADIUS: f32 = 9.0;
const NAME_SIZE: f32 = 12.5;
const SUMMARY_SIZE: f32 = 10.5;
const PIN_SIZE: f32 = 10.0;
const INSET: f32 = 10.0;

impl SceneTemplatePanel<'_> {
    pub(super) fn paint_style_cards(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let viewport = self.cards_viewport(panel);
        let cards = self.style_cards();
        if cards.is_empty() {
            self.paint_text(
                cx,
                self.t("assetCenter.style.empty", "没有匹配的风格"),
                Point2D::new(viewport.origin.x + 4.0, viewport.origin.y + 28.0),
                12.0,
                self.theme.muted_foreground,
            );
            return;
        }

        cx.backend.save();
        cx.backend.clip_rect(viewport);
        let pinned = self.pinned_style_guide();
        for (index, rect) in self.card_rects_for_count(panel, cards.len()) {
            // Same cheap reject the template grid uses: rects for scrolled-out
            // rows are still computed so hover and paint agree on indices.
            if rect.origin.y > viewport.origin.y + viewport.size.y
                || rect.origin.y + rect.size.y < viewport.origin.y
            {
                continue;
            }
            self.paint_style_card(cx, rect, &cards[index], index, pinned);
        }
        cx.backend.restore();
    }

    fn paint_style_card(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        card: &StyleGuideCard,
        index: usize,
        pinned: Option<&str>,
    ) {
        let is_pinned = card.is_pinned(pinned);
        cx.backend
            .fill_round_rect(rect, CARD_RADIUS, self.theme.card);
        // The pinned card is outlined in the primary colour at double width.
        // A pin survives the panel closing and steers every later generation,
        // so it has to read as state, not as hover.
        let (border, border_w) = if is_pinned {
            (self.theme.primary, 2.0)
        } else {
            (self.theme.border, 1.0)
        };
        cx.backend
            .stroke_round_rect(rect, CARD_RADIUS, border, border_w);
        paint_button_feedback_wash(
            cx.backend,
            &self.theme,
            rect,
            CARD_RADIUS,
            self.state.editor_ui.scene_template_center.hover == Some(index),
            self.is_pressed(index),
        );

        let text_x = rect.origin.x + INSET;
        let text_w = rect.size.x - INSET * 2.0;

        let pin_label = self.t("assetCenter.style.pinned", "已钉住");
        let pin_w = if is_pinned {
            estimated_text_width(pin_label, PIN_SIZE) + 8.0
        } else {
            0.0
        };
        self.paint_text(
            cx,
            &truncate_to_width(card.name, (text_w - pin_w).max(0.0), NAME_SIZE),
            Point2D::new(text_x, rect.origin.y + 24.0),
            NAME_SIZE,
            self.theme.foreground,
        );
        if is_pinned {
            self.paint_text(
                cx,
                pin_label,
                Point2D::new(
                    rect.origin.x + rect.size.x - INSET - pin_w + 8.0,
                    rect.origin.y + 23.0,
                ),
                PIN_SIZE,
                self.theme.primary,
            );
        }

        self.paint_swatch_band(cx, Self::swatch_band_rect(rect), card);

        self.paint_text(
            cx,
            &truncate_to_width(&card.summary, text_w, SUMMARY_SIZE),
            Point2D::new(text_x, rect.origin.y + STYLE_CARD_H - 12.0),
            SUMMARY_SIZE,
            self.theme.muted_foreground,
        );
    }

    /// The colour band's rect. Shared with the tests so a band that stops
    /// sitting inside its card fails loudly instead of painting off-card.
    pub(super) fn swatch_band_rect(card: Rect) -> Rect {
        Rect::xywh(
            card.origin.x + INSET,
            card.origin.y + 34.0,
            card.size.x - INSET * 2.0,
            STYLE_SWATCH_H,
        )
    }

    fn paint_swatch_band(&self, cx: &mut PaintCx<'_>, band: Rect, card: &StyleGuideCard) {
        if card.swatches.is_empty() {
            return;
        }
        let count = card.swatches.len() as f32;
        let width = band.size.x / count;
        for (index, color) in card.swatches.iter().enumerate() {
            let swatch = Rect::xywh(
                band.origin.x + index as f32 * width,
                band.origin.y,
                width,
                band.size.y,
            );
            cx.backend.fill_rect(swatch, *color);
        }
        // One hairline around the whole band rather than per swatch: a light
        // guide's near-white background is otherwise invisible on the card.
        cx.backend
            .stroke_round_rect(band, 3.0, self.theme.border, 1.0);
    }
}
