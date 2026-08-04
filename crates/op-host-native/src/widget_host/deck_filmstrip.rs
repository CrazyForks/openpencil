//! Deck filmstrip arm — the native host's platform tail.
//!
//! Which slides exist, where the chips sit, and what a press / drag /
//! release means all live in `op_editor_ui::widgets::deck_filmstrip*`;
//! this file only supplies the canvas rect, frames a board when a chip is
//! clicked, and issues the reorder command when one is dropped. Its web
//! twin (`op-host-web/src/widget_host/deck_filmstrip.rs`) is the same
//! shape over the same shared flow.

use super::WidgetHostNative;
use op_editor_ui::widgets::deck_filmstrip_flow as flow;
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::{FilmstripChip, FilmstripLayout};
use op_editor_ui::Point2D;

/// What a cursor move over the strip amounted to: whether the strip owns
/// the point (so lower hover tiers stop) and whether anything visible
/// changed (the repaint signal).
pub(in crate::widget_host) struct FilmstripHover {
    pub(in crate::widget_host) owns: bool,
    pub(in crate::widget_host) changed: bool,
}

/// Everything an event or a paint needs about the strip: the slides, the
/// one the camera is on, and where the chips are.
pub(in crate::widget_host) struct FilmstripFrame {
    pub(in crate::widget_host) chips: Vec<FilmstripChip>,
    pub(in crate::widget_host) active: Option<usize>,
    pub(in crate::widget_host) layout: FilmstripLayout,
}

impl WidgetHostNative {
    /// Resolve the strip for the current document, or `None` when this
    /// document shows none. Every filmstrip entry point starts here, so
    /// paint and hit-test can never lay the chips out differently.
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
        frame_backend: &mut dyn op_editor_ui::RenderBackend,
        strip: &FilmstripFrame,
    ) {
        use op_editor_ui::widgets::PaintCx;
        let widget = flow::filmstrip_widget(&strip.chips, strip.active, &self.editor_state);
        let mut cx = PaintCx {
            backend: frame_backend,
        };
        widget.paint(&mut cx, &strip.layout, &self.theme);
    }

    /// Route a press. Returns whether the strip claimed it — a press
    /// anywhere on the pill is the strip's, so it never also reaches the
    /// canvas behind it.
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
            // The strip vanished (document changed, preview started) —
            // drop any hover it left behind.
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
            // A drag keeps ownership wherever the pointer went: releasing
            // it is still the strip's business.
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
                    self.frame_deck_board(&chip.id, viewport_w, viewport_h);
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

    /// Move a board to a new position in the page's child order.
    ///
    /// Goes through the ordinary command path, so the reorder is undoable
    /// like every other document edit — and, being a pure structural
    /// move, it carries no property rewrites for the collaboration gate
    /// to reject.
    fn apply_filmstrip_reorder(&mut self, board_id: &str, to: usize) {
        if !self.collab_allows_document_mutation(op_editor_core::CollabDocumentMutation::NodeMove) {
            return;
        }
        flow::apply_reorder(&mut self.editor_state, board_id, to);
    }

    /// Screen rect of the strip, for the tests + any caller that needs to
    /// know whether a point is the strip's before routing it.
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
