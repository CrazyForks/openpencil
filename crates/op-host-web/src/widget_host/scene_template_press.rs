//! Thin web dispatch arm for the shared Scene Template Center press flow.
//!
//! The template request is drained here, in the same call that raised it,
//! rather than from a frame pump. It can be: bringing a template in is pure
//! `EditorState` work on this host — the boards and their variables are
//! embedded, so there is no loader, no file dialog, and nothing to await. The
//! desktop drains from its event loop because its starter path also unbinds a
//! file path and rewrites the window title.

use op_editor_core::scene_template_append::template_boards;
use op_editor_core::scene_template_catalog::{scene_template_by_id, scene_template_document};
use op_editor_ui::widgets::press_flow;
use op_editor_ui::Point2D;

use super::WidgetHost;

impl WidgetHost {
    pub(in crate::widget_host) fn dispatch_scene_template_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let Some(panel_rect) = self.scene_template_panel_rect(viewport_width, viewport_height)
        else {
            return false;
        };
        let Some(changed) = press_flow::press_scene_template_center(
            &mut self.editor_state,
            panel_rect,
            Point2D::new(x, y),
            self.now_ms,
        ) else {
            return false;
        };
        let brought_in = self.drain_pending_scene_template(viewport_width, viewport_height);
        if changed || brought_in {
            self.mark_dirty();
        }
        true
    }

    /// Bring a chosen template into the document. Returns whether it changed.
    fn drain_pending_scene_template(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        let Some(template_id) = self
            .editor_state
            .editor_ui
            .scene_template_center
            .take_pending_open()
        else {
            return false;
        };
        let Some(source) = scene_template_document(&template_id) else {
            return false;
        };
        let Some(boards) = template_boards(source, &template_id) else {
            return false;
        };
        // `adopt` picks replace-vs-append from what is on the page: an
        // untouched starter is taken over, anything else is added beside.
        if !self.editor_state.adopt_template_boards(boards) {
            return false;
        }
        // Only when the document has no scenario yet — a template dropped
        // next to other work does not redefine what the file is.
        if self.editor_state.editor_ui.scenario.is_none() {
            self.editor_state.editor_ui.scenario =
                scene_template_by_id(&template_id).map(|template| template.scene);
        }
        self.fit_content_to_viewport(viewport_w, viewport_h);
        self.force_rotate_layer_panel_owner();
        true
    }
}
