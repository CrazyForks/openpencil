//! "Style: Dimension ✕" — what the next generation will actually use.
//!
//! The Asset Center already shows a pin: the card is outlined, the chip says
//! 已钉住. But the place a person presses Generate is here, and this is where
//! the pin was invisible. Two separate failures shipped looking identical from
//! the outside — a pin that never reached the plan, and a pin that reached it
//! carrying no readable values — and in both the user saw only that the colours
//! were wrong. Neither was diagnosable from the editor.
//!
//! So the row answers two questions at a glance, and the colour band is the
//! second one. The name says *which* style is in force; the band says whether
//! anything could be read out of it. An empty band on a named style is the
//! visible form of "this file parsed, but its values did not" — the condition
//! that previously required running a probe against the file to detect.
//!
//! It says nothing when there is nothing true to say: a pin naming a guide
//! that no longer exists shows no row, because generation has already fallen
//! back to choosing its own style and naming the dead guide would replace one
//! silent failure with a loud false one.

use op_ai_skills::style_guide::StyleGuideSummary;
use op_editor_core::EditorState;
use op_util::hex_color;

use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

/// Height of the receipt row inside the input block.
pub(crate) const STYLE_RECEIPT_ROW_HEIGHT: f32 = 24.0;
const CHIP_HEIGHT: f32 = 20.0;
const PAD_X: f32 = 8.0;
const FONT: f32 = 11.0;
const CLEAR_W: f32 = 18.0;
const SWATCH_W: f32 = 9.0;
const SWATCH_H: f32 = 10.0;
const SWATCH_GAP: f32 = 2.0;

/// What the row shows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StyleReceipt {
    /// The name a person reads.
    pub name: String,
    /// The band, already parsed. Empty means the guide stated no values this
    /// build could read — which the row shows by drawing no band.
    pub swatches: Vec<Color>,
    /// Whether pressing ✕ can clear what this row describes.
    ///
    /// False for the design.md row: design.md is bound and unbound from its
    /// own panel, and a ✕ here would either do nothing or silently clear a
    /// pin that is not the thing being reported.
    pub clearable: bool,
}

impl StyleReceipt {
    /// The receipt for the current editor state, or `None` when the row
    /// should not paint at all.
    ///
    /// Precedence mirrors the generation pipeline exactly, because a receipt
    /// that disagreed with the pipeline would be worse than none: design.md
    /// outranks a pin there, so it outranks it here.
    pub(crate) fn for_state(state: &EditorState) -> Option<Self> {
        if state.doc.design_md.is_some() {
            // Named rather than hidden. A user who pinned a style and sees
            // "design.md" here learns why their pin is not showing up, which
            // is the one question silence cannot answer.
            return Some(Self {
                name: "design.md".to_string(),
                swatches: Vec::new(),
                clearable: false,
            });
        }
        let pinned = state.editor_ui.pinned_style_guide.as_deref()?;
        let summary = op_ai_skills::style_guide::style_guide_summary(pinned)?;
        Some(Self::from_summary(&summary))
    }

    fn from_summary(summary: &StyleGuideSummary) -> Self {
        Self {
            name: summary.name.clone(),
            swatches: summary
                .swatches
                .iter()
                .filter_map(|hex| parse(hex))
                .collect(),
            clearable: true,
        }
    }
}

fn parse(raw: &str) -> Option<Color> {
    let [r, g, b, a] = hex_color::parse_hex_rgba8(raw, hex_color::HexOptions::LENIENT)?;
    Some(Color::rgba_u8(r, g, b, a as f32 / 255.0))
}

/// Width the chip needs for `label` plus its band and clear button.
pub(crate) fn chip_width(label: &str, swatches: usize, clearable: bool) -> f32 {
    let band = if swatches == 0 {
        0.0
    } else {
        swatches as f32 * SWATCH_W + (swatches.saturating_sub(1)) as f32 * SWATCH_GAP + 8.0
    };
    let clear = if clearable { CLEAR_W } else { 6.0 };
    PAD_X + crate::widgets::ai_chat_panel::footer_label_width(label, FONT) + band + clear
}

/// The chip's rect at the top of the input block.
pub(crate) fn chip_rect(receipt: &StyleReceipt, label: &str, input_rect: Rect) -> Rect {
    let width = chip_width(label, receipt.swatches.len(), receipt.clearable)
        .min(input_rect.size.x)
        .max(56.0);
    Rect {
        origin: Point2D::new(
            input_rect.origin.x,
            input_rect.origin.y + (STYLE_RECEIPT_ROW_HEIGHT - CHIP_HEIGHT) / 2.0,
        ),
        size: Point2D::new(width, CHIP_HEIGHT),
    }
}

/// The ✕ target, or `None` when this row has nothing to clear.
pub(crate) fn clear_rect(receipt: &StyleReceipt, label: &str, input_rect: Rect) -> Option<Rect> {
    if !receipt.clearable {
        return None;
    }
    let chip = chip_rect(receipt, label, input_rect);
    Some(Rect {
        origin: Point2D::new(chip.origin.x + chip.size.x - CLEAR_W, chip.origin.y),
        size: Point2D::new(CLEAR_W, chip.size.y),
    })
}

/// Paint the row.
pub(crate) fn paint(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    receipt: &StyleReceipt,
    label: &str,
    input_rect: Rect,
) {
    let chip = chip_rect(receipt, label, input_rect);
    cx.backend.fill_round_rect(chip, 6.0, theme.muted);

    let baseline_y = chip.origin.y + chip.size.y / 2.0 + FONT * 0.35;
    let text = TextLayout::single_run(
        label,
        "system-ui",
        FONT,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&text, Point2D::new(chip.origin.x + PAD_X, baseline_y));

    let label_end =
        chip.origin.x + PAD_X + crate::widgets::ai_chat_panel::footer_label_width(label, FONT);
    paint_band(cx, receipt, chip, label_end);

    if receipt.clearable {
        let clear = TextLayout::single_run(
            "×",
            "system-ui",
            FONT,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &clear,
            Point2D::new(
                chip.origin.x + chip.size.x - CLEAR_W / 2.0 - 3.0,
                baseline_y,
            ),
        );
    }
}

/// The colour band, clipped to whatever room is left between the name and the
/// clear button. Swatches that would overrun are dropped rather than squeezed:
/// a band is read as "these are the colours", and a half-drawn one reads as a
/// different palette.
fn paint_band(cx: &mut PaintCx<'_>, receipt: &StyleReceipt, chip: Rect, label_end: f32) {
    if receipt.swatches.is_empty() {
        return;
    }
    let clear_w = if receipt.clearable { CLEAR_W } else { 6.0 };
    let available = chip.origin.x + chip.size.x - clear_w - (label_end + 8.0);
    let top = chip.origin.y + (chip.size.y - SWATCH_H) / 2.0;
    for (index, color) in receipt.swatches.iter().enumerate() {
        let x = label_end + 8.0 + index as f32 * (SWATCH_W + SWATCH_GAP);
        if x + SWATCH_W > label_end + 8.0 + available {
            break;
        }
        cx.backend.fill_round_rect(
            Rect {
                origin: Point2D::new(x, top),
                size: Point2D::new(SWATCH_W, SWATCH_H),
            },
            2.0,
            *color,
        );
    }
}

#[cfg(test)]
#[path = "ai_chat_style_receipt_tests.rs"]
mod ai_chat_style_receipt_tests;
