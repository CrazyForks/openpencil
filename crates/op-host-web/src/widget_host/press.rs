//! Web press handlers split from `widget_host.rs`; mirrors the
//! native press/click split and keeps `EditorState` as source of truth.
//! `apply_click` lives in `click.rs`, the StatusBar / overlay rect
//! helpers in `overlay_rects.rs`, and the per-overlay press
//! dispatchers in their own sibling modules (mirroring the native
//! host's layout) so this file stays under the 800-line cap.
//!
//! `apply_press` itself is now just the tier spine: the tier bodies live
//! in the `press_*_tiers.rs` siblings and `press_ctx.rs` carries the
//! per-event state they share.
use op_editor_ui::widgets::press_flow::{
    self, LayerContextMenuPress, LayerContextStep, PropertyOverlayPress,
};
use op_editor_ui::Point2D;

use super::press_ctx::PressCtx;
use super::WidgetHost;

impl WidgetHost {
    /// Right-click handler — opens the LayerPanel context menu on
    /// a layer or page row.
    pub fn apply_right_press(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        self.commit_variable_row_focus_if_any();
        self.close_image_popovers_for_higher_overlay();
        if self.over_topmost_panel(x, y, viewport_w, viewport_h) {
            return true;
        }
        // The model-picker card may hang over the LayerPanel. Keep a
        // secondary press on that visible overlay from opening the covered
        // layer/page context menu underneath it.
        if self
            .chat_model_picker_rect(viewport_w, viewport_h)
            .is_some_and(|rect| rect.contains(Point2D::new(x, y)))
        {
            return true;
        }
        if self.try_open_path_anchor_menu(x, y, viewport_w, viewport_h) {
            return true;
        }
        if !self.editor_state.editor_ui.sidebar_open {
            return self.blur_text_inputs_on_blank_press();
        }
        self.refresh_layout_scene();
        let layer_rect = self.layer_panel_rect(viewport_h);
        let panel = self.layer_panel();
        let hit = panel.hit_test(layer_rect, Point2D::new(x, y));
        match press_flow::open_layer_context_menu(&mut self.editor_state, hit, x, y) {
            LayerContextMenuPress::Opened | LayerContextMenuPress::Dismissed => {
                self.mark_dirty();
                true
            }
            LayerContextMenuPress::Missed => self.blur_text_inputs_on_blank_press(),
        }
    }

    pub(in crate::widget_host) fn dispatch_layer_context_action(
        &mut self,
        action: op_editor_ui::widgets::layer_context_menu::LayerContextAction,
        target: op_editor_core::ui_draft::LayerContextTarget,
    ) {
        match press_flow::apply_layer_context_action(
            &mut self.editor_state,
            &mut self.next_node_id,
            action,
            target,
            self.now_ms,
        ) {
            LayerContextStep::Done => {}
            LayerContextStep::Group => {
                let _ = self.apply_group();
            }
            LayerContextStep::Boolean(op) => {
                let _ = self.apply_boolean_op(op);
            }
            LayerContextStep::Refit => {
                self.fit_active_page_after_switch(self.last_viewport_w, self.last_viewport_h);
            }
        }
        self.mark_dirty();
    }

    /// Platform tail for a press routed to an open property-panel
    /// popover (`press_flow::press_*`). Every outcome consumes the
    /// press.
    pub(in crate::widget_host) fn finish_property_overlay_press(
        &mut self,
        press: PropertyOverlayPress,
    ) -> bool {
        match press {
            PropertyOverlayPress::Action(action) => self.apply_property_action(action),
            PropertyOverlayPress::Swallow => {}
            PropertyOverlayPress::Dismissed => self.mark_dirty(),
        }
        true
    }

    /// Mouse-press handler. Returns whether anything visible changed.
    ///
    /// A strictly ordered hit-test ladder: overlays before panels before
    /// canvas. Each tier helper returns `Option<bool>` — `None` means
    /// "declined, fall through to the next tier", `Some(dirty)` means
    /// "claimed the press, and this is the repaint signal".
    ///
    /// THE CALL ORDER BELOW *IS* THE BEHAVIOUR. The tier bodies live in
    /// the `press_*_tiers.rs` siblings only to respect the per-file line
    /// cap. This ladder is deliberately NOT shared with the native host:
    /// the two differ in tier order and gating in several places.
    pub fn apply_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // Cache the viewport dims so `apply_cursor_move(x, y)` (no
        // viewport params in signature) can rebuild the canvas region
        // for the floating align toolbar's hover sync. Mirrors the
        // native host's `last_viewport_w` / `_h` cache.
        self.last_viewport_w = viewport_width;
        self.last_viewport_h = viewport_height;
        self.last_cursor_x = x;
        self.last_cursor_y = y;
        // Refresh the derived paint doc once up front — every hit-test
        // below reads `&self.layout_scene`, so it must be current.
        self.refresh_layout_scene();
        // 0-pre. Commit any in-flight rename + canvas text-edit on
        // first press anywhere. Tracked so the final return reports
        // the visible change.
        let rename_committed =
            self.editor_state.ui.layer_rename.is_some() && self.editor_state.rename_commit();
        let text_edit_was_active = self.editor_state.ui.text_editing.is_some();
        let text_edit_committed = self.editor_state.text_edit_commit();
        if rename_committed || text_edit_committed {
            self.mark_dirty();
        }
        let mut ctx = PressCtx {
            x,
            y,
            viewport_width,
            viewport_height,
            rename_committed,
            text_edit_was_active,
            text_edit_committed,
            // Both resolved below, at the exact points the flat ladder did.
            over_chat_model_picker: false,
            property_focus_committed: false,
        };
        // Tier 1 — top-most overlays / floating panels / context menus.
        if let Some(consumed) = self.press_topmost_overlay_tiers(&ctx) {
            return consumed;
        }
        // Tier 2 — import dropdown + locale picker.
        if let Some(consumed) = self.press_import_locale_tiers(&ctx) {
            return consumed;
        }
        // Tier 3 — shape picker, file / export / figma / login / account.
        if let Some(consumed) = self.press_menu_modal_tiers(&ctx) {
            return consumed;
        }
        // Tier 4 — image-fill popover, StatusBar, and the model-picker
        // slice that lifts above the TopBar.
        if let Some(consumed) = self.press_rail_overlay_tiers(&ctx) {
            return consumed;
        }
        ctx.over_chat_model_picker = self
            .chat_model_picker_rect(viewport_width, viewport_height)
            .is_some_and(|rect| rect.contains(Point2D::new(x, y)));
        // Tier 5 — theme-preset dropdown + floating VariablesPanel.
        if let Some(consumed) = self.press_variables_tiers(&ctx) {
            return consumed;
        }
        // Tier 6 — TopBar chrome (and its blank-press gaps).
        if let Some(consumed) = self.press_top_bar_tier(&ctx) {
            return consumed;
        }
        // Tier 7 — property-panel popovers, then the fonts + model-picker
        // overlay band.
        if let Some(consumed) = self.press_property_overlay_tiers(&ctx) {
            return consumed;
        }
        if let Some(consumed) = self.press_font_and_picker_tiers(&ctx) {
            return consumed;
        }
        // Tier 8 — PropertyPanel input row.
        if let Some(consumed) = self.press_property_panel_tier(&ctx) {
            return consumed;
        }
        ctx.property_focus_committed = self.commit_property_family_focus_if_any();
        let property_focus_committed = ctx.property_focus_committed;
        // Tier 9 — AI chat panel.
        if let Some(consumed) = self.press_chat_tier(&ctx) {
            return consumed;
        }
        // Tier 10 — toolbar.
        if let Some(consumed) = self.press_toolbar_tier(&ctx) {
            return consumed;
        }
        // Tier 11 — LayerPanel drag peek, align toolbar, `apply_click`.
        if let Some(consumed) = self.press_layer_align_click_tiers(&ctx) {
            return consumed;
        }
        // Tier 12 — the canvas, branching on the active tool.
        if let Some(consumed) = self.press_canvas_tier(&ctx) {
            return consumed;
        }
        // Final fall-through — the press hit no interactive chrome
        // (panel-rail gaps, property-panel padding, …): blank press.
        let blurred = self.blur_text_inputs_on_blank_press();
        blurred || rename_committed || text_edit_committed || property_focus_committed
    }
}
