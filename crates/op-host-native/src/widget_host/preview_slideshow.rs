//! Deck slideshow arm of Preview (Play) mode.
//!
//! Previewing a deck presents it: one board filling the viewport, the rest
//! of the canvas backdrop, arrow keys to move. The decision of WHETHER to
//! present, which boards there are, and where the presenter is in the deck
//! all live in `op_editor_core::preview_slideshow`; this file is the
//! platform arm — camera, paint, and the key forwarding.
//!
//! It reuses the existing preview pipeline rather than adding a second one:
//! the board is framed by the shared viewport fit math, and painted by
//! `PreviewSession::paint_scene`, the same painter every other preview uses.
//! The only thing this adds is a clip to the board's own rect, which is what
//! makes the neighbouring boards disappear and the surround read as a
//! letterbox.

use super::*;
use op_editor_ui::Point2D;

/// Scene-space rect through the canvas pan/zoom into screen space.
///
/// Same formula as the canvas painter's (`canvas_origin + pan + doc *
/// zoom`); taken from the passed `canvas` rect rather than re-deriving the
/// canvas region so paint and this clip can never disagree by a frame.
fn board_screen_rect(canvas: Rect, viewport: &op_editor_core::Viewport, board: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            canvas.origin.x + viewport.pan_x + board.origin.x * viewport.zoom,
            canvas.origin.y + viewport.pan_y + board.origin.y * viewport.zoom,
        ),
        size: Point2D::new(board.size.x * viewport.zoom, board.size.y * viewport.zoom),
    }
}

impl WidgetHostNative {
    /// Whether preview is currently presenting a deck.
    ///
    /// Both halves matter: the editor state carries the deck, the host
    /// carries the live session that paints it. A slideshow without a
    /// session would paint nothing and swallow the arrow keys.
    pub fn preview_slideshow_active(&self) -> bool {
        self.preview.is_some() && self.editor_state.preview_slideshow().is_some()
    }

    /// Start the slideshow if the document entering preview is a deck, and
    /// frame its first board. Returns whether a slideshow began.
    ///
    /// Called from `enter_preview` once the session exists. A document that
    /// is not a deck leaves preview untouched.
    pub(in crate::widget_host) fn begin_slideshow_if_deck(
        &mut self,
        canvas_size: (f32, f32),
    ) -> bool {
        if !self.editor_state.begin_preview_slideshow() {
            return false;
        }
        // A deck has no phone / desktop dimension to it — it is presented at
        // its own aspect. Canvas is the device mode that adds no chrome, so
        // the switcher (hidden while presenting) still reads truthfully if
        // the user leaves the slideshow.
        self.editor_state.editor_ui.preview.device = Some(PreviewDeviceKind::Canvas);
        self.frame_slideshow_board(canvas_size);
        true
    }

    /// Fit the board on screen into the canvas region, centred. Returns
    /// whether the camera moved.
    ///
    /// Reuses `Viewport::fit_to`, the same fit the editor uses to frame a
    /// freshly opened document, so a board is framed by exactly the math
    /// everything else is. Zero padding: a presentation fills the surface,
    /// and the surround is the letterbox rather than a margin.
    ///
    /// Called once per frame from paint, which is the only place that knows
    /// the true canvas size — the cached viewport is still zero on a fresh
    /// host and stale for a frame after a resize. Re-framing every frame is
    /// also what keeps a presentation framed while the window is resized,
    /// and is a no-op (no repaint) once the camera has settled.
    pub(in crate::widget_host) fn frame_slideshow_board(
        &mut self,
        canvas_size: (f32, f32),
    ) -> bool {
        let Some(board) = self.slideshow_board_scene_rect() else {
            return false;
        };
        let before = self.editor_state.viewport;
        self.editor_state
            .viewport
            .fit_to(board, canvas_size.0, canvas_size.1, 0.0);
        let moved = self.editor_state.viewport != before;
        if moved {
            self.mark_dirty();
        }
        moved
    }

    /// Scene-space rect of the board being presented.
    fn slideshow_board_scene_rect(&self) -> Option<Rect> {
        let session = self.preview.as_ref()?;
        let board = self.editor_state.preview_slideshow()?.current_board()?;
        session.root_scene_rect(board)
    }

    /// Move the presentation one board forward (`1`) or back (`-1`) and
    /// re-frame. Returns whether the board on screen changed — `false`
    /// outside a slideshow and at the clamped ends of the deck.
    ///
    /// Public so the desktop runner's key handling can reach it for the
    /// keys the preview runtime never sees (Space / PageUp / PageDown).
    pub fn preview_slideshow_step(&mut self, delta: i32) -> bool {
        if !self.preview_slideshow_active() {
            return false;
        }
        if !self.editor_state.step_preview_slideshow(delta) {
            // At either end of the deck the board holds. The press was still
            // ours — the caller swallows it either way, which is what keeps
            // the last slide on screen instead of letting the key fall
            // through to the editor underneath.
            return false;
        }
        // The camera follows in `frame_slideshow_board`, which paint runs
        // against the authoritative canvas size on the next frame.
        self.mark_dirty();
        true
    }

    /// Paint the board on screen, letterboxed, plus the slide counter.
    ///
    /// The caller has already filled the canvas backdrop, so everything
    /// outside the board's own rect stays backdrop — that IS the letterbox.
    pub(crate) fn paint_slideshow(
        &self,
        frame_backend: &mut dyn op_editor_ui::RenderBackend,
        canvas_rect: Rect,
    ) {
        use op_editor_ui::widgets::{PaintCx, SlideshowCounter};

        let Some(session) = self.preview.as_ref() else {
            return;
        };
        let Some(slideshow) = self.editor_state.preview_slideshow() else {
            return;
        };
        if let Some(board) = self.slideshow_board_scene_rect() {
            let screen = board_screen_rect(canvas_rect, &self.editor_state.viewport, board);
            frame_backend.save();
            frame_backend.clip_rect(screen);
            session.paint_scene(
                frame_backend,
                canvas_rect,
                (
                    self.editor_state.viewport.pan_x,
                    self.editor_state.viewport.pan_y,
                ),
                self.editor_state.viewport.zoom,
                self.now_ms,
            );
            frame_backend.restore();
        }
        let label = slideshow.counter_label();
        let counter = SlideshowCounter { label: &label };
        let mut cx = PaintCx {
            backend: frame_backend,
        };
        counter.paint(&mut cx, canvas_rect, &self.theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_board_maps_through_pan_and_zoom_like_the_canvas_painter() {
        let canvas = Rect {
            origin: Point2D::new(100.0, 40.0),
            size: Point2D::new(1000.0, 700.0),
        };
        let mut viewport = op_editor_core::Viewport::IDENTITY;
        viewport.zoom = 0.5;
        viewport.pan_x = 20.0;
        viewport.pan_y = 10.0;
        let board = Rect {
            origin: Point2D::new(200.0, 400.0),
            size: Point2D::new(1920.0, 1080.0),
        };

        let screen = board_screen_rect(canvas, &viewport, board);

        assert_eq!(screen.origin.x, 100.0 + 20.0 + 100.0);
        assert_eq!(screen.origin.y, 40.0 + 10.0 + 200.0);
        assert_eq!(screen.size.x, 960.0);
        assert_eq!(screen.size.y, 540.0);
    }
}
