//! Slides panel — the left rail's page-navigator tab.
//!
//! One row per top-level board, in page order: a number, a real
//! rendered thumbnail of the board, and its name. Clicking a row frames
//! that board; dragging one reorders the deck; the footer starts the
//! presentation. It is the deck's only navigator — what a slide IS,
//! which one the camera is on and how a reorder commits all come from
//! [`crate::widgets::deck_boards`], so the rail can never disagree with
//! the presentation about the order.
//!
//! **This widget paints a thumbnail PLACEHOLDER, never a thumbnail.**
//! Rendering a board is platform work — a second skia surface per board
//! — so the host paints its cached rasters into [`SlidesPanelLayout::thumb_rect`]
//! after the widget has painted. A host without a local renderer (the
//! browser) simply paints nothing there and the placeholder stands, which
//! is why the placeholder carries the slide number rather than being a
//! blank hole.
//!
//! Every rect is derived WITHOUT measuring text — rows are a fixed
//! height for a given board aspect — so the host's hit-test and the
//! paint pass compute the same layout from the same four inputs (panel
//! rect, row count, board aspect, scroll offset). Measurement only
//! decides where a name is cut, never where a row is.

use op_editor_core::{SlidesDrag, SlidesPanelTarget};

use crate::widgets::deck_boards::BoardChip;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout, Theme};

/// Height of the tab row that heads the rail.
pub const SLIDES_TAB_ROW_HEIGHT: f32 = 36.0;
/// Short alias used inside this module's geometry.
const TAB_ROW_HEIGHT: f32 = SLIDES_TAB_ROW_HEIGHT;
/// Height of the footer holding the present button.
pub const FOOTER_HEIGHT: f32 = 48.0;
const TAB_INSET_X: f32 = 8.0;
const TAB_INSET_Y: f32 = 5.0;
const TAB_RADIUS: f32 = 6.0;
const TAB_FONT: f32 = 12.0;
/// Glyph size for a tab in icon mode.
const TAB_ICON_SIZE: f32 = 14.0;
/// Padding inside a tab, around whatever it holds.
const TAB_PAD_X: f32 = 8.0;
/// Gap between a tab's glyph and its label, when it keeps one.
const TAB_ICON_GAP: f32 = 5.0;

const ROW_PAD_X: f32 = 10.0;
/// Gutter holding the slide number, left of the thumbnail.
const INDEX_COL_W: f32 = 20.0;
const INDEX_GAP: f32 = 6.0;
/// Gap between the thumbnail and the name line under it.
const NAME_GAP: f32 = 4.0;
const NAME_H: f32 = 16.0;
/// Vertical gap between rows.
const ROW_GAP: f32 = 8.0;
const ROW_FONT: f32 = 11.5;
const THUMB_RADIUS: f32 = 4.0;
/// Fixed height of every row's thumbnail box.
///
/// The one number that makes a mixed document listable: rows keep this
/// height whether the page holds 16:9 decks, 3:4 cards or 9:19.5 phone
/// screens, and each board is fitted into the box rather than the box
/// being fitted to the board.
pub const THUMB_BOX_H: f32 = 132.0;
/// Fallback board aspect (16:9) for a deck whose boards have no
/// resolvable bounds yet — the scene may not have been built when the
/// first frame paints.
pub const DEFAULT_BOARD_ASPECT: f32 = 16.0 / 9.0;
/// How far a press has to travel before it stops being a click. Matches
/// the canvas node-drag threshold so the two gestures feel the same.
pub const DRAG_THRESHOLD_PX: f32 = 3.0;
const FOOTER_BUTTON_H: f32 = 30.0;
const DROP_BAR_H: f32 = 2.0;
const GHOST_ALPHA: f32 = 0.35;

#[path = "slides_panel_tabs.rs"]
mod tabs;

pub use tabs::{text_tabs_fit, SlidesPanelTabs};

/// Where the slides tab's rows, list viewport and footer sit.
///
/// Built once per event / per paint and shared by both, which is what
/// keeps a row's painted rect and its click target identical.
#[derive(Debug, Clone, PartialEq)]
pub struct SlidesPanelLayout {
    pub panel: Rect,
    pub tabs: SlidesPanelTabs,
    /// The clipped, scrolling band the rows live in.
    pub list: Rect,
    pub footer: Rect,
    /// The present button inside the footer.
    pub present: Rect,
    /// How far the row stack is scrolled up.
    pub offset: f32,
    pub count: usize,
    /// The thumbnail BOX every row gets — the same for all of them (see
    /// `new`). Individual boards are letterboxed inside it.
    pub thumb_box: Point2D,
    /// Each board's picture size inside the box, in board order.
    thumbs: Vec<Point2D>,
}

impl SlidesPanelLayout {
    /// Lay the slides tab out inside `panel`.
    ///
    /// `aspects` is one width / height per board, in page order.
    ///
    /// **Rows are a FIXED height and boards are letterboxed into them.**
    /// Sizing the row to the board instead would make a mixed document
    /// unusable: one 3:4 card among the 16:9 boards stretches every row
    /// in the list to the tallest shape, and a page of phone screens
    /// gives rows twice the height of the rail. A fixed box also keeps
    /// the list readable as a sequence — the eye counts positions, not
    /// shapes — and keeps the drag arithmetic a division instead of a
    /// scan.
    ///
    /// `tabs` is passed in rather than derived because the tab row's
    /// own geometry depends on the labels, and labels are i18n — which
    /// this module deliberately knows nothing about. The flow resolves
    /// both from one place so paint and hit-test cannot disagree.
    ///
    /// `None` when the rail is too small to show a row — a list that
    /// cannot show a slide is worse than no list: it is a strip that
    /// eats clicks and explains nothing.
    pub fn new(panel: Rect, tabs: SlidesPanelTabs, aspects: &[f32], offset: f32) -> Option<Self> {
        let box_w = panel.size.x - ROW_PAD_X * 2.0 - INDEX_COL_W - INDEX_GAP;
        if box_w <= 0.0 {
            return None;
        }
        let thumb_box = Point2D::new(box_w, THUMB_BOX_H);
        let thumbs = aspects.iter().map(|a| fit_into(thumb_box, *a)).collect();
        let count = aspects.len();
        let list_top = panel.origin.y + TAB_ROW_HEIGHT;
        let list_h = (panel.size.y - TAB_ROW_HEIGHT - FOOTER_HEIGHT).max(0.0);
        if list_h <= 0.0 {
            return None;
        }
        let list = Rect {
            origin: Point2D::new(panel.origin.x, list_top),
            size: Point2D::new(panel.size.x, list_h),
        };
        let footer = Rect {
            origin: Point2D::new(panel.origin.x, list_top + list_h),
            size: Point2D::new(panel.size.x, FOOTER_HEIGHT),
        };
        let present = Rect {
            origin: Point2D::new(
                footer.origin.x + ROW_PAD_X,
                footer.origin.y + (FOOTER_HEIGHT - FOOTER_BUTTON_H) / 2.0,
            ),
            size: Point2D::new((footer.size.x - ROW_PAD_X * 2.0).max(0.0), FOOTER_BUTTON_H),
        };
        let mut layout = Self {
            panel,
            tabs,
            list,
            footer,
            present,
            offset: 0.0,
            count,
            thumb_box,
            thumbs,
        };
        layout.offset = offset.clamp(0.0, layout.max_scroll());
        Some(layout)
    }

    /// Height of one row: the thumbnail box plus the name line under it.
    /// The same for every row, whatever shape the boards are.
    pub fn row_height(&self) -> f32 {
        THUMB_BOX_H + NAME_GAP + NAME_H
    }

    /// Row pitch — a row plus the gap that follows it.
    fn row_stride(&self) -> f32 {
        self.row_height() + ROW_GAP
    }

    /// Total height of the row stack, including the trailing padding
    /// that keeps the last row clear of the footer.
    pub fn content_height(&self) -> f32 {
        if self.count == 0 {
            return 0.0;
        }
        ROW_GAP + self.count as f32 * self.row_stride()
    }

    /// How far the list can scroll.
    pub fn max_scroll(&self) -> f32 {
        (self.content_height() - self.list.size.y).max(0.0)
    }

    /// Rect of row `index`, in screen space. Not clamped to the list: a
    /// row scrolled out of view still has a position, which is what lets
    /// the drag arithmetic work on rows the user cannot see.
    pub fn row_rect(&self, index: usize) -> Rect {
        Rect {
            origin: Point2D::new(
                self.list.origin.x,
                self.list.origin.y - self.offset + ROW_GAP + index as f32 * self.row_stride(),
            ),
            size: Point2D::new(self.list.size.x, self.row_height()),
        }
    }

    /// The fixed-size box row `index` gives its thumbnail — the same
    /// shape in every row, whatever the board inside it looks like.
    pub fn thumb_box_rect(&self, index: usize) -> Rect {
        let row = self.row_rect(index);
        Rect {
            origin: Point2D::new(
                row.origin.x + ROW_PAD_X + INDEX_COL_W + INDEX_GAP,
                row.origin.y,
            ),
            size: self.thumb_box,
        }
    }

    /// Where row `index`'s board actually paints: its own aspect scaled
    /// to fit [`Self::thumb_box_rect`] and centred in it — letterboxed
    /// above and below for a wide board, pillarboxed either side for a
    /// tall one. This is the rect a host blits its rendered board into,
    /// so the picture never stretches to a shape the board is not.
    pub fn thumb_rect(&self, index: usize) -> Rect {
        let boxed = self.thumb_box_rect(index);
        let size = self
            .thumbs
            .get(index)
            .copied()
            .unwrap_or_else(|| fit_into(self.thumb_box, DEFAULT_BOARD_ASPECT));
        Rect {
            origin: Point2D::new(
                boxed.origin.x + (boxed.size.x - size.x) / 2.0,
                boxed.origin.y + (boxed.size.y - size.y) / 2.0,
            ),
            size,
        }
    }

    /// The part of row `index`'s thumbnail that is inside the list
    /// band, or `None` when none of it is.
    ///
    /// A host blitting a rendered board MUST clip to this rather than to
    /// [`Self::thumb_rect`]: the widget clips its own placeholder to the
    /// band, so an unclipped blit would put the last row's picture over
    /// the footer while the placeholder under it stopped at the edge.
    pub fn visible_thumb_rect(&self, index: usize) -> Option<Rect> {
        let thumb = self.thumb_rect(index);
        let top = thumb.origin.y.max(self.list.origin.y);
        let bottom = (thumb.origin.y + thumb.size.y).min(self.list.origin.y + self.list.size.y);
        (bottom > top).then(|| Rect {
            origin: Point2D::new(thumb.origin.x, top),
            size: Point2D::new(thumb.size.x, bottom - top),
        })
    }

    /// The rows with any part inside the list band, paired with their
    /// rects. Hosts render thumbnails for exactly these, so an
    /// off-screen slide never costs a raster.
    pub fn visible_rows(&self) -> Vec<(usize, Rect)> {
        (0..self.count)
            .map(|index| (index, self.row_rect(index)))
            .filter(|(_, rect)| {
                rect.origin.y + rect.size.y > self.list.origin.y
                    && rect.origin.y < self.list.origin.y + self.list.size.y
            })
            .collect()
    }

    /// Which row `point` lands on. Only the part of a row inside the
    /// band counts — a half-scrolled row must not be clickable where it
    /// is not painted.
    pub fn row_at(&self, point: Point2D) -> Option<usize> {
        if !contains(self.list, point) {
            return None;
        }
        self.visible_rows()
            .into_iter()
            .find_map(|(index, rect)| contains(rect, point).then_some(index))
    }

    /// What `point` lands on anywhere in the panel.
    pub fn hit(&self, point: Point2D) -> Option<SlidesPanelTarget> {
        if let Some(tab) = self.tabs.hit(point) {
            return Some(tab);
        }
        if contains(self.present, point) {
            return Some(SlidesPanelTarget::Present);
        }
        self.row_at(point).map(SlidesPanelTarget::Slide)
    }

    /// Whether `point` is anywhere on the panel. The press path uses
    /// this to decide the press is the panel's business and must not
    /// reach the surfaces below.
    pub fn contains_point(&self, point: Point2D) -> bool {
        contains(self.panel, point)
    }

    /// The slot a row dropped at `pointer_y` would be inserted before,
    /// in the range `0..=count`. Counted from row CENTRES, so the drop
    /// flips as the dragged row passes the middle of its neighbour.
    pub fn insertion_slot(&self, pointer_y: f32) -> usize {
        let content_y = pointer_y - (self.list.origin.y - self.offset) - ROW_GAP;
        (0..self.count)
            .filter(|index| {
                let centre = *index as f32 * self.row_stride() + self.row_height() / 2.0;
                content_y > centre
            })
            .count()
    }

    /// Screen y of the bar marking `slot`, clamped into the band so it
    /// stays visible at either end of a scrolled list.
    fn drop_bar_y(&self, slot: usize) -> f32 {
        let content_y = if slot == 0 {
            ROW_GAP / 2.0
        } else {
            ROW_GAP + (slot - 1) as f32 * self.row_stride() + self.row_height() + ROW_GAP / 2.0
        };
        (self.list.origin.y - self.offset + content_y).clamp(
            self.list.origin.y,
            self.list.origin.y + self.list.size.y - DROP_BAR_H,
        )
    }
}

/// Whether a drag has travelled far enough to be a reorder rather than
/// a click that has not been released yet.
pub fn drag_is_live(drag: &SlidesDrag) -> bool {
    (drag.pointer_y - drag.press_y).abs() > DRAG_THRESHOLD_PX
}

/// The panel, ready to paint.
pub struct SlidesPanel<'a> {
    pub chips: &'a [BoardChip],
    /// The slide the camera is looking at, if any board resolves.
    pub active: Option<usize>,
    pub hover: Option<SlidesPanelTarget>,
    pub drag: Option<SlidesDrag>,
    /// Whether the host will paint a rendered board over the thumbnail
    /// box. When it will not, the box carries the slide number at size
    /// instead of staying empty — an empty plate reads as broken, a
    /// numbered one reads as a slide without a preview.
    pub thumbnails_supported: bool,
    pub layers_label: &'a str,
    pub slides_label: &'a str,
    pub present_label: &'a str,
}

impl SlidesPanel<'_> {
    pub fn paint(&self, cx: &mut PaintCx<'_>, layout: &SlidesPanelLayout, theme: &Theme) {
        cx.backend.fill_rect(layout.panel, theme.card);
        // Right-edge hairline, so the rail reads as a distinct surface
        // from the canvas next to it — same as the layers tab.
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(
                    layout.panel.origin.x + layout.panel.size.x - 1.0,
                    layout.panel.origin.y,
                ),
                size: Point2D::new(1.0, layout.panel.size.y),
            },
            theme.border,
        );
        layout
            .tabs
            .paint(cx, theme, self.hover, self.layers_label, self.slides_label);

        cx.backend.save();
        cx.backend.clip_rect(layout.list);
        let dragging = self.drag.filter(drag_is_live);
        for (index, rect) in layout.visible_rows() {
            let ghosted = dragging.is_some_and(|drag| drag.from == index);
            self.paint_row(cx, theme, layout, index, rect, ghosted);
        }
        if let Some(drag) = dragging {
            let slot = layout.insertion_slot(drag.pointer_y);
            cx.backend.fill_rect(
                Rect {
                    origin: Point2D::new(layout.list.origin.x + ROW_PAD_X, layout.drop_bar_y(slot)),
                    size: Point2D::new((layout.list.size.x - ROW_PAD_X * 2.0).max(0.0), DROP_BAR_H),
                },
                theme.primary,
            );
        }
        cx.backend.restore();

        self.paint_footer(cx, theme, layout);
    }

    fn paint_footer(&self, cx: &mut PaintCx<'_>, theme: &Theme, layout: &SlidesPanelLayout) {
        cx.backend.fill_rect(layout.footer, theme.card);
        cx.backend.fill_rect(
            Rect {
                origin: layout.footer.origin,
                size: Point2D::new(layout.footer.size.x, 1.0),
            },
            theme.border,
        );
        let hovered = self.hover == Some(SlidesPanelTarget::Present);
        let button = layout.present;
        cx.backend.fill_round_rect(
            button,
            6.0,
            if hovered {
                theme.primary
            } else {
                Color {
                    a: 0.86,
                    ..theme.primary
                }
            },
        );
        let icon_size = 13.0;
        let label_w = cx.backend.measure_text(self.present_label, TAB_FONT);
        let content_w = icon_size + 6.0 + label_w;
        let icon_x = button.origin.x + (button.size.x - content_w) / 2.0;
        draw_icon(
            cx.backend,
            Icon::Play,
            Point2D::new(icon_x, button.origin.y + (button.size.y - icon_size) / 2.0),
            icon_size,
            theme.primary_foreground,
            1.6,
        );
        cx.backend.draw_text(
            &TextLayout::single_run(
                self.present_label,
                "system-ui",
                TAB_FONT,
                theme.primary_foreground.to_jian(),
                Point2D::ZERO,
            )
            .with_font_weight(600),
            Point2D::new(
                icon_x + icon_size + 6.0,
                button.origin.y + button.size.y / 2.0 + TAB_FONT / 2.0 - 1.5,
            ),
        );
    }

    fn paint_row(
        &self,
        cx: &mut PaintCx<'_>,
        theme: &Theme,
        layout: &SlidesPanelLayout,
        index: usize,
        row: Rect,
        ghosted: bool,
    ) {
        let active = self.active == Some(index);
        let alpha = if ghosted { GHOST_ALPHA } else { 1.0 };
        // The board's own fitted rect, not the box around it: the plate
        // has to sit exactly where the host will blit, or a tall card
        // would show a wide plate with its picture floating inside.
        let thumb = layout.thumb_rect(index);
        if self.hover == Some(SlidesPanelTarget::Slide(index)) && !ghosted {
            cx.backend.fill_round_rect(
                Rect {
                    origin: Point2D::new(row.origin.x + 4.0, row.origin.y - 3.0),
                    size: Point2D::new((row.size.x - 8.0).max(0.0), row.size.y + 6.0),
                },
                6.0,
                theme.button_hover,
            );
        }
        // Thumbnail placeholder. The host paints its rendered board over
        // this rect; on a host without a renderer the placeholder is
        // what the user sees, so it has to stand on its own.
        cx.backend
            .fill_round_rect(thumb, THUMB_RADIUS, fade(theme.muted, alpha));
        cx.backend.stroke_round_rect(
            thumb,
            THUMB_RADIUS,
            fade(if active { theme.primary } else { theme.border }, alpha),
            if active { 2.0 } else { 1.0 },
        );

        let number = format!("{}", index + 1);
        let number_color = if active {
            fade(theme.primary, alpha)
        } else {
            fade(theme.muted_foreground, alpha)
        };
        let number_w = cx.backend.measure_text(&number, ROW_FONT);
        cx.backend.draw_text(
            &TextLayout::single_run(
                &number,
                "system-ui",
                ROW_FONT,
                number_color.to_jian(),
                Point2D::ZERO,
            )
            .with_font_weight(600),
            Point2D::new(
                row.origin.x + ROW_PAD_X + (INDEX_COL_W - number_w).max(0.0),
                row.origin.y + ROW_FONT + 2.0,
            ),
        );
        if !self.thumbnails_supported {
            // No renderer will cover this box, so fill it rather than
            // leave a blank plate.
            let size = (thumb.size.y * 0.34).min(34.0);
            let width = cx.backend.measure_text(&number, size);
            cx.backend.draw_text(
                &TextLayout::single_run(
                    &number,
                    "system-ui",
                    size,
                    fade(theme.muted_foreground, alpha * 0.55).to_jian(),
                    Point2D::ZERO,
                )
                .with_font_weight(600),
                Point2D::new(
                    thumb.origin.x + (thumb.size.x - width) / 2.0,
                    thumb.origin.y + thumb.size.y / 2.0 + size / 2.0 - size * 0.15,
                ),
            );
        }

        let Some(chip) = self.chips.get(index) else {
            return;
        };
        if chip.name.is_empty() {
            return;
        }
        // Anchored to the row's fixed box, not to the letterboxed
        // picture: the names have to sit on one baseline down the list,
        // and a tall board's narrow plate must not indent its own label.
        let boxed = layout.thumb_box_rect(index);
        let name =
            crate::widgets::file_menu::truncate_to_width(cx, &chip.name, ROW_FONT, boxed.size.x);
        if name.is_empty() {
            return;
        }
        let color = if active {
            fade(theme.primary, alpha)
        } else {
            fade(theme.card_foreground, alpha)
        };
        cx.backend.draw_text(
            &TextLayout::single_run(&name, "system-ui", ROW_FONT, color.to_jian(), Point2D::ZERO),
            Point2D::new(
                boxed.origin.x,
                boxed.origin.y + boxed.size.y + NAME_GAP + ROW_FONT,
            ),
        );
    }
}

/// The largest `aspect`-shaped rectangle that fits inside `boxed`.
///
/// A non-finite or non-positive aspect falls back to 16:9 rather than
/// producing a NaN rect — an unresolved board still has to have somewhere
/// to paint, and the scene has not built one on the first frame after a
/// document opens.
fn fit_into(boxed: Point2D, aspect: f32) -> Point2D {
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        DEFAULT_BOARD_ASPECT
    };
    let by_width = Point2D::new(boxed.x, boxed.x / aspect);
    if by_width.y <= boxed.y {
        by_width
    } else {
        Point2D::new(boxed.y * aspect, boxed.y)
    }
}

fn fade(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}

pub(super) fn contains(rect: Rect, point: Point2D) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.x
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.y
}

#[cfg(test)]
#[path = "slides_panel_tests.rs"]
mod slides_panel_tests;
