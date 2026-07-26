//! Window-lifecycle handlers (close / resize / move / DPI / file drop)
//! for `DesktopApp`'s `window_event` dispatcher. Carved out of the
//! `app_handler.rs` spine to keep it under the 800-line cap; pure code
//! motion.

use crate::{chat_session, figma_import_session, html_import_session, persistence, DesktopApp};
use std::path::PathBuf;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

impl DesktopApp {
    pub(super) fn on_close_requested(&mut self, event_loop: &ActiveEventLoop) {
        // Finalize-lifecycle invariant (0718-1-k3-1 postmortem): the
        // process exiting must not silently drop an in-flight,
        // unfinalized design loop — see `chat_session::
        // finalize_design_session_if_needed`'s doc comment.
        // Best-effort (the worker thread itself is abandoned either
        // way — no graceful async shutdown here), but it must run
        // BEFORE `confirm_close` below, not after: the unsaved-
        // changes prompt's `document_is_dirty` check + its own Save
        // decide what gets persisted, so a finalize mutation applied
        // afterward would never make it to disk.
        chat_session::finalize_design_session_if_needed(
            &mut self.host,
            &self.current_chat,
            "teardown-backstop",
        );
        // The unsaved-changes prompt can abort the close.
        if self.confirm_close() {
            self.host.flush_settings_input();
            op_host_services::settings_io::save(self.host.editor_state());
            event_loop.exit();
        }
    }

    pub(super) fn on_resized(&mut self, event_loop: &ActiveEventLoop, size: PhysicalSize<u32>) {
        if let Some(ctx) = self.ctx.as_mut() {
            if let Err(err) = ctx.resize(size.width, size.height) {
                eprintln!("openpencil-desktop: resize failed: {err}");
                self.error = Some(err);
                event_loop.exit();
            }
        } else {
            self.try_init_render_context(event_loop);
        }
        self.viewport_width = size.width as f32 / self.dpi;
        self.viewport_height = size.height as f32 / self.dpi;
        // Re-lay-out the preview runtime (if active) against the
        // new canvas region so the live scene tracks the resize.
        self.host
            .preview_resize(self.viewport_width, self.viewport_height);
        // Track geometry for window-state persistence. Only a
        // *windowed* (non-maximized) size is remembered so a
        // restart restores a sensible un-maximize size.
        self.win_maximized = self.window.as_ref().is_some_and(Window::is_maximized);
        if !self.win_maximized {
            self.win_size = Some((size.width, size.height));
        }
        // Fullscreen enter / exit arrives as a `Resized` on
        // macOS — refresh the flag so the TopBar drops its
        // traffic-light reservation when the native lights
        // hide in fullscreen.
        let fullscreen = self
            .window
            .as_ref()
            .is_some_and(|w| w.fullscreen().is_some());
        if self.host.editor_state().editor_ui.window_fullscreen != fullscreen {
            self.host.editor_state_mut().editor_ui.window_fullscreen = fullscreen;
            self.host.mark_editor_state_dirty();
        }
        self.request_redraw(true);
    }

    pub(super) fn on_moved(&mut self, pos: PhysicalPosition<i32>) {
        // Remember the windowed position for window-state
        // persistence; skip while maximized so the restored
        // position is the user's chosen spot.
        if !self.window.as_ref().is_some_and(Window::is_maximized) {
            self.win_pos = Some((pos.x, pos.y));
        }
    }

    pub(super) fn on_hovered_file(&mut self) {
        // A file is being dragged over the window — show the
        // full-canvas drop overlay so the target is obvious.
        if !self.host.editor_state().editor_ui.file_drop_active {
            self.host.editor_state_mut().editor_ui.file_drop_active = true;
            self.host.mark_editor_state_dirty();
            self.request_redraw(true);
        }
    }

    pub(super) fn on_hovered_file_cancelled(&mut self) {
        // The drag left the window without dropping — hide it.
        if self.host.editor_state().editor_ui.file_drop_active {
            self.host.editor_state_mut().editor_ui.file_drop_active = false;
            self.host.mark_editor_state_dirty();
            self.request_redraw(true);
        }
    }

    pub(super) fn on_dropped_file(&mut self, path: PathBuf) {
        // Clear the drag overlay now that the drop has landed.
        self.host.editor_state_mut().editor_ui.file_drop_active = false;
        self.host.mark_editor_state_dirty();
        self.request_redraw(true);
        // Drag-and-drop open. `.op` / `.pen` documents route
        // through the canonical loader; `.fig` Figma exports
        // route through the background Figma import worker
        // (the parse + layout pass takes seconds for large
        // dashboards, so doing it inline would freeze the
        // window). Anything else is ignored silently so a
        // stray drop can't disrupt the current document.
        if op_host_services::doc_io::is_supported_figma_import(&path) {
            let _ = self.begin_figma_import(path);
        } else if op_host_services::doc_io::is_supported_html_import(&path) {
            figma_import_session::cancel(&mut self.host, &mut self.current_figma_import);
            html_import_session::cancel(&mut self.host, &mut self.current_html_import);
            self.current_html_import = Some(html_import_session::spawn(&mut self.host, path));
            self.request_redraw(true);
        } else if op_host_services::doc_io::is_supported_document(&path) {
            if persistence::open_path(
                &mut self.host,
                path,
                &mut self.current_path,
                self.window.as_ref(),
            ) {
                self.mark_document_saved();
                self.request_redraw(true);
            }
        } else {
            eprintln!(
                        "openpencil-desktop: ignored dropped file (not .op / .pen / .fig / .html / .zip): {}",
                        path.display()
                    );
        }
    }

    pub(super) fn on_scale_factor_changed(&mut self, scale_factor: f64) {
        self.dpi = scale_factor as f32;
        if let Some(backend) = self.backend.as_mut() {
            backend.set_dpi(scale_factor as f32);
        }
        // Logical = physical / dpi; refresh to keep input + paint coords in sync after a DPI flip.
        if let Some(window) = self.window.as_ref() {
            let size = window.inner_size();
            self.viewport_width = size.width as f32 / self.dpi;
            self.viewport_height = size.height as f32 / self.dpi;
        }
        self.request_redraw(true);
    }
}
