//! Deck filmstrip arm — the web host's platform tail.
//!
//! Byte-for-byte the same decisions as native's
//! `op-host-native/src/widget_host/deck_filmstrip.rs`, because both call
//! the same `op_editor_ui::widgets::deck_filmstrip_flow`. Only the host
//! type differs; nothing about a slide deck is platform-shaped.

use super::WidgetHost;
use op_editor_ui::widgets::deck_filmstrip_flow as flow;
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::{FilmstripChip, FilmstripLayout};
use op_editor_ui::Point2D;

/// What a cursor move over the strip amounted to — see the native twin.
pub(in crate::widget_host) struct FilmstripHover {
    pub(in crate::widget_host) owns: bool,
    pub(in crate::widget_host) changed: bool,
}

/// Everything an event or a paint needs about the strip.
pub(in crate::widget_host) struct FilmstripFrame {
    pub(in crate::widget_host) chips: Vec<FilmstripChip>,
    pub(in crate::widget_host) active: Option<usize>,
    pub(in crate::widget_host) layout: FilmstripLayout,
}

impl WidgetHost {
    /// Resolve the strip for the current document, or `None` when this
    /// document shows none.
    pub(in crate::widget_host) fn deck_filmstrip_frame(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<FilmstripFrame> {
        let chips = flow::filmstrip_chips(&self.editor_state)?;
        let canvas = canvas_geometry::canvas_rect(&self.editor_state, viewport_w, viewport_h);
        self.refresh_layout_scene();
        let active =
            flow::active_chip_index(&chips, &self.layout_scene, &self.editor_state, canvas);
        let layout = flow::filmstrip_layout(&chips, active, canvas)?;
        Some(FilmstripFrame {
            chips,
            active,
            layout,
        })
    }

    pub(in crate::widget_host) fn paint_deck_filmstrip(
        &self,
        backend: &mut dyn op_editor_ui::RenderBackend,
        strip: &FilmstripFrame,
    ) {
        use op_editor_ui::widgets::PaintCx;
        let widget = flow::filmstrip_widget(&strip.chips, strip.active, &self.editor_state);
        let mut cx = PaintCx { backend };
        widget.paint(&mut cx, &strip.layout, &self.theme);
    }

    /// Route a press. Returns whether the strip claimed it.
    pub(in crate::widget_host) fn deck_filmstrip_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let Some(strip) = self.deck_filmstrip_frame(viewport_w, viewport_h) else {
            return false;
        };
        match flow::press(&mut self.editor_state, &strip.layout, Point2D::new(x, y)) {
            flow::FilmstripPress::Missed => false,
            flow::FilmstripPress::Claimed(_) => {
                self.mark_dirty();
                true
            }
        }
    }

    /// Track the cursor over the strip.
    pub(in crate::widget_host) fn deck_filmstrip_hover(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> FilmstripHover {
        let Some(strip) = self.deck_filmstrip_frame(viewport_w, viewport_h) else {
            let changed = self.editor_state.editor_ui.deck_filmstrip.clear();
            if changed {
                self.mark_dirty();
            }
            return FilmstripHover {
                owns: false,
                changed,
            };
        };
        let point = Point2D::new(x, y);
        let changed = flow::cursor_move(&mut self.editor_state, &strip.layout, point);
        if changed {
            self.mark_dirty();
        }
        FilmstripHover {
            owns: strip.layout.contains_point(point)
                || self.editor_state.editor_ui.deck_filmstrip.drag.is_some(),
            changed,
        }
    }

    /// Cursor-move tier for the strip, in the same band as the StatusBar.
    /// `Some(dirty)` when the strip claimed the move.
    ///
    /// Where the two pills overlap the StatusBar still wins — it paints on
    /// top, and hit-test order has to follow paint order. A live chip drag
    /// keeps ownership wherever the pointer went, so a reorder does not
    /// cancel the moment the cursor strays off the strip.
    pub(in crate::widget_host) fn deck_filmstrip_cursor_tier(
        &mut self,
        point: Point2D,
        over_topmost: bool,
    ) -> Option<bool> {
        if over_topmost {
            return None;
        }
        let over_status = self
            .status_bar_rect(self.last_viewport_w, self.last_viewport_h)
            .is_some_and(|rect| rect.contains(point));
        let hover =
            self.deck_filmstrip_hover(point.x, point.y, self.last_viewport_w, self.last_viewport_h);
        if hover.owns && !over_status {
            let below_changed = self.clear_chat_and_lower_hover();
            return Some(hover.changed || below_changed);
        }
        hover.changed.then_some(true)
    }

    /// Close a filmstrip gesture. Returns whether one was in flight.
    pub(in crate::widget_host) fn deck_filmstrip_release(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if self.editor_state.editor_ui.deck_filmstrip.pressed.is_none() {
            return false;
        }
        let Some(strip) = self.deck_filmstrip_frame(viewport_w, viewport_h) else {
            self.editor_state.editor_ui.deck_filmstrip.clear();
            return false;
        };
        match flow::release(&mut self.editor_state, &strip.layout) {
            flow::FilmstripRelease::Idle => false,
            flow::FilmstripRelease::Cancelled => {
                self.mark_dirty();
                true
            }
            flow::FilmstripRelease::Activate(index) => {
                if let Some(chip) = strip.chips.get(index) {
                    self.frame_deck_board(&chip.id.clone(), viewport_w, viewport_h);
                }
                self.mark_dirty();
                true
            }
            flow::FilmstripRelease::Reorder { from, to } => {
                if let Some(chip) = strip.chips.get(from) {
                    self.apply_filmstrip_reorder(&chip.id.clone(), to);
                }
                self.mark_dirty();
                true
            }
        }
    }

    /// Frame one board in the canvas region. Camera only — navigating a
    /// deck must not land on the undo stack.
    fn frame_deck_board(&mut self, board_id: &str, viewport_w: f32, viewport_h: f32) {
        self.refresh_layout_scene();
        op_editor_ui::widgets::host_overlay_geometry::zoom_to_fit_node(
            &mut self.editor_state,
            &self.layout_scene,
            board_id,
            viewport_w,
            viewport_h,
        );
    }

    /// Move a board to a new position in the page's child order, undoably.
    ///
    /// No collaboration gate here, unlike native's twin: the web host
    /// carries no collaboration session at all, so there is nothing to
    /// ask. If one lands, this is the line that grows the check.
    fn apply_filmstrip_reorder(&mut self, board_id: &str, to: usize) {
        flow::apply_reorder(&mut self.editor_state, board_id, to);
    }

    /// Screen rect of the strip, for the tests.
    #[cfg(test)]
    pub(in crate::widget_host) fn deck_filmstrip_rect(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<op_editor_ui::Rect> {
        Some(
            self.deck_filmstrip_frame(viewport_w, viewport_h)?
                .layout
                .strip,
        )
    }
}

#[cfg(test)]
#[path = "deck_filmstrip_tests.rs"]
mod deck_filmstrip_tests;
