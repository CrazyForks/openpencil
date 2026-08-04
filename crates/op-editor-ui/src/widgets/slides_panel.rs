//! Slides panel — the left rail's page-navigator tab.
//!
//! One row per top-level board, in page order: a number, a real
//! rendered thumbnail of the board, and its name. Clicking a row frames
//! that board; dragging one reorders the deck; the footer starts the
//! presentation. It is the same navigator the canvas filmstrip is,
//! given the room to show pictures — and the two share their model
//! (`FilmstripChip`), their "which slide is the camera on" answer
//! (`deck_filmstrip_flow::active_chip_index`) and their reorder commit
//! (`deck_filmstrip_flow::apply_reorder`), so the deck can never be in
//! two orders at once.
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

use op_editor_core::{LeftPanelTab, SlidesDrag, SlidesPanelTarget};

use crate::widgets::deck_filmstrip::FilmstripChip;
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
/// Fallback board aspect (16:9) for a deck whose boards have no
/// resolvable bounds yet — the scene may not have been built when the
/// first frame paints.
pub const DEFAULT_BOARD_ASPECT: f32 = 16.0 / 9.0;
/// How far a press has to travel before it stops being a click. Matches
/// the filmstrip's threshold so the two navigators feel the same.
pub const DRAG_THRESHOLD_PX: f32 = 3.0;
const FOOTER_BUTTON_H: f32 = 30.0;
const DROP_BAR_H: f32 = 2.0;
const GHOST_ALPHA: f32 = 0.35;

/// Rects of the two-tab row that heads the rail.
///
/// Resolved on its own (not only as part of the full slides layout)
/// because the row also paints — and takes clicks — while the LAYERS
/// tab owns the rest of the rail.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlidesPanelTabs {
    pub row: Rect,
    pub layers: Rect,
    pub slides: Rect,
}

impl SlidesPanelTabs {
    /// Lay the tab row across the top of `panel`.
    pub fn new(panel: Rect) -> Self {
        let row = Rect {
            origin: panel.origin,
            size: Point2D::new(panel.size.x, TAB_ROW_HEIGHT),
        };
        let inner_w = (row.size.x - TAB_INSET_X * 2.0).max(0.0);
        let half = inner_w / 2.0;
        let y = row.origin.y + TAB_INSET_Y;
        let h = (TAB_ROW_HEIGHT - TAB_INSET_Y * 2.0).max(0.0);
        Self {
            row,
            layers: Rect {
                origin: Point2D::new(row.origin.x + TAB_INSET_X, y),
                size: Point2D::new(half, h),
            },
            slides: Rect {
                origin: Point2D::new(row.origin.x + TAB_INSET_X + half, y),
                size: Point2D::new(half, h),
            },
        }
    }

    /// Which tab `point` lands on, or `None` when it is off the row.
    pub fn hit(&self, point: Point2D) -> Option<SlidesPanelTarget> {
        if !contains(self.row, point) {
            return None;
        }
        if contains(self.layers, point) {
            return Some(SlidesPanelTarget::LayersTab);
        }
        contains(self.slides, point).then_some(SlidesPanelTarget::SlidesTab)
    }

    /// The rail below the tab row — what the Layers tree gets when it
    /// is the tab on show.
    pub fn content_rect(&self, panel: Rect) -> Rect {
        Rect {
            origin: Point2D::new(panel.origin.x, panel.origin.y + TAB_ROW_HEIGHT),
            size: Point2D::new(panel.size.x, (panel.size.y - TAB_ROW_HEIGHT).max(0.0)),
        }
    }

    /// Paint the tab row. `active` is the tab currently on show;
    /// `hover` is whichever tab the cursor is over, if any.
    pub fn paint(
        &self,
        cx: &mut PaintCx<'_>,
        theme: &Theme,
        active: LeftPanelTab,
        hover: Option<SlidesPanelTarget>,
        layers_label: &str,
        slides_label: &str,
    ) {
        cx.backend.fill_rect(self.row, theme.card);
        // The segmented track, so the two tabs read as one control
        // rather than two loose buttons.
        cx.backend.fill_round_rect(
            Rect {
                origin: self.layers.origin,
                size: Point2D::new(self.layers.size.x + self.slides.size.x, self.layers.size.y),
            },
            TAB_RADIUS,
            theme.muted,
        );
        for (rect, tab, label) in [
            (self.layers, LeftPanelTab::Layers, layers_label),
            (self.slides, LeftPanelTab::Slides, slides_label),
        ] {
            let target = match tab {
                LeftPanelTab::Layers => SlidesPanelTarget::LayersTab,
                LeftPanelTab::Slides => SlidesPanelTarget::SlidesTab,
            };
            let selected = active == tab;
            if selected {
                cx.backend.fill_round_rect(rect, TAB_RADIUS, theme.card);
                cx.backend
                    .stroke_round_rect(rect, TAB_RADIUS, theme.border, 1.0);
            } else if hover == Some(target) {
                cx.backend
                    .fill_round_rect(rect, TAB_RADIUS, theme.button_hover);
            }
            let color = if selected {
                theme.foreground
            } else {
                theme.muted_foreground
            };
            let width = cx.backend.measure_text(label, TAB_FONT);
            let baseline = rect.origin.y + rect.size.y / 2.0 + TAB_FONT / 2.0 - 1.5;
            cx.backend.draw_text(
                &TextLayout::single_run(
                    label,
                    "system-ui",
                    TAB_FONT,
                    color.to_jian(),
                    Point2D::ZERO,
                )
                .with_font_weight(if selected { 600 } else { 400 }),
                Point2D::new(rect.origin.x + (rect.size.x - width) / 2.0, baseline),
            );
        }
    }
}

/// Where the slides tab's rows, list viewport and footer sit.
///
/// Built once per event / per paint and shared by both, which is what
/// keeps a row's painted rect and its click target identical.
#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// Thumbnail box size — uniform across the deck (see `new`).
    pub thumb: Point2D,
}

impl SlidesPanelLayout {
    /// Lay the slides tab out inside `panel`.
    ///
    /// `aspect` is the board's width / height. ONE aspect for the whole
    /// deck, not one per board: uniform tiles keep the list readable as
    /// a sequence (the eye counts positions, not shapes) and make the
    /// drag arithmetic a division instead of a scan. Decks are uniform
    /// by construction, so the first board answers for all of them.
    ///
    /// `None` when the rail is too small to show a row — a list that
    /// cannot show a slide is worse than no list: it is a strip that
    /// eats clicks and explains nothing.
    pub fn new(panel: Rect, count: usize, aspect: f32, offset: f32) -> Option<Self> {
        let tabs = SlidesPanelTabs::new(panel);
        let thumb_w = panel.size.x - ROW_PAD_X * 2.0 - INDEX_COL_W - INDEX_GAP;
        let aspect = if aspect.is_finite() && aspect > 0.0 {
            aspect
        } else {
            DEFAULT_BOARD_ASPECT
        };
        let thumb_h = thumb_w / aspect;
        if thumb_w <= 0.0 || !thumb_h.is_finite() || thumb_h <= 0.0 {
            return None;
        }
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
            thumb: Point2D::new(thumb_w, thumb_h),
        };
        layout.offset = offset.clamp(0.0, layout.max_scroll());
        Some(layout)
    }

    /// Height of one row: thumbnail plus the name line under it.
    pub fn row_height(&self) -> f32 {
        self.thumb.y + NAME_GAP + NAME_H
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

    /// The thumbnail box inside row `index` — the rect a host blits its
    /// rendered board into.
    pub fn thumb_rect(&self, index: usize) -> Rect {
        let row = self.row_rect(index);
        Rect {
            origin: Point2D::new(
                row.origin.x + ROW_PAD_X + INDEX_COL_W + INDEX_GAP,
                row.origin.y,
            ),
            size: self.thumb,
        }
    }

    /// The part of row `index`'s thumbnail box that is inside the list
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
    pub chips: &'a [FilmstripChip],
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
        layout.tabs.paint(
            cx,
            theme,
            LeftPanelTab::Slides,
            self.hover,
            self.layers_label,
            self.slides_label,
        );

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
        let thumb = Rect {
            origin: Point2D::new(
                row.origin.x + ROW_PAD_X + INDEX_COL_W + INDEX_GAP,
                row.origin.y,
            ),
            size: layout.thumb,
        };
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
        let name =
            crate::widgets::file_menu::truncate_to_width(cx, &chip.name, ROW_FONT, thumb.size.x);
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
                thumb.origin.x,
                thumb.origin.y + thumb.size.y + NAME_GAP + ROW_FONT,
            ),
        );
    }
}

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
#[path = "slides_panel_tests.rs"]
mod slides_panel_tests;
