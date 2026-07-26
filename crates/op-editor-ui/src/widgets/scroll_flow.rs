//! Panel wheel / trackpad-pan scrolling shared by the native and web
//! widget hosts (their `widget_host/scroll.rs` twins carried these as a
//! verbatim copy-paste run).
//!
//! Each entry point returns `Option<bool>`: `None` when the pointer was
//! not over the surface (the caller keeps looking, and eventually zooms
//! the canvas), `Some(dirty)` when the surface swallowed the event —
//! `dirty` then asks the host to repaint. The panel rects themselves
//! stay host-side because they are derived from host viewport
//! bookkeeping, so callers pass them in.

use op_editor_core::EditorState;

use crate::util::scroll_by_max;
use crate::widgets::layer_panel::LayerPanel;
use crate::{Point2D, Rect};

/// Screen rect of the left-hand layer rail — the canonical walk lives
/// in [`crate::widgets::host_canvas_geometry`]; re-exported here so the
/// scroll callers need only one import.
pub fn layer_panel_rect(state: &EditorState, viewport_height: f32) -> Rect {
    crate::widgets::host_canvas_geometry::layer_panel_rect(state, viewport_height)
}

/// Scroll the floating VariablesPanel row list (TS `overflow-y-auto`
/// rows region). The whole panel rect swallows the event so a wheel
/// over its header can't zoom the canvas beneath.
pub fn scroll_variables_panel(
    state: &mut EditorState,
    panel_rect: Option<Rect>,
    point: Point2D,
    delta_y: f32,
) -> Option<bool> {
    if !state.editor_ui.variables_panel_open {
        return None;
    }
    let panel_rect = panel_rect?;
    if !panel_rect.contains(point) {
        return None;
    }
    use crate::widgets::variables_panel::VariablesPanel;
    let max = VariablesPanel::for_editor(state).max_scroll(panel_rect);
    Some(scroll_by_max(
        &mut state.editor_ui.variables_scroll,
        -delta_y,
        max,
    ))
}

/// Scroll the floating Design-MD panel body.
pub fn scroll_design_md_panel(
    state: &mut EditorState,
    panel_rect: Option<Rect>,
    point: Point2D,
    delta_y: f32,
) -> Option<bool> {
    let panel_rect = panel_rect?;
    if !panel_rect.contains(point) {
        return None;
    }
    let max = crate::widgets::DesignMdPanel::for_editor(state)?.max_scroll(panel_rect);
    Some(scroll_by_max(
        &mut state.editor_ui.design_md_panel.scroll,
        -delta_y,
        max,
    ))
}

/// Scroll the TopBar locale dropdown. Scrolling always drops the row
/// hover so the highlight can't stick to a row that moved away.
pub fn scroll_locale_picker(
    state: &mut EditorState,
    picker_rect: Rect,
    point: Point2D,
    delta_y: f32,
) -> Option<bool> {
    if !state.editor_ui.locale_picker.open {
        return None;
    }
    if !picker_rect.contains(point) {
        return None;
    }
    let ui = &mut state.editor_ui.locale_picker;
    let next = (ui.scroll.offset - delta_y).clamp(0.0, crate::widgets::LocalePicker::max_scroll());
    let changed = next != ui.scroll.offset || ui.hover.is_some();
    ui.scroll.offset = next;
    ui.hover = None;
    Some(changed)
}

/// Scroll the open icon picker's list. The picker loads up to 120 local
/// and remote icons — far more than fit — so the list must scroll; this
/// runs before `over_topmost_panel`, which would otherwise swallow the
/// event without advancing the rows.
pub fn scroll_icon_picker(
    state: &mut EditorState,
    panel_rect: Option<Rect>,
    point: Point2D,
    delta_y: f32,
) -> Option<bool> {
    if !state.editor_ui.icon_picker.open {
        return None;
    }
    let rect = panel_rect?;
    if !rect.contains(point) {
        return None;
    }
    use crate::widgets::icon_picker_panel::IconPickerPanel;
    let mut dirty = false;
    if let Some(panel) = IconPickerPanel::for_editor(state) {
        let max = panel.icon_picker_max_scroll(rect);
        let scroll = &mut state.editor_ui.icon_picker.scroll;
        let next = (scroll.offset - delta_y).clamp(0.0, max);
        if next != scroll.offset {
            scroll.offset = next;
            dirty = true;
        }
    }
    Some(dirty)
}

/// Scroll the right-rail PropertyPanel body once the hosts' own
/// popover-priority preamble has declined the event: the Code tab's
/// horizontal framework strip, the Code preview box, then the panel's
/// own vertical scroll. `None` when the pointer is outside the rail.
pub fn scroll_property_panel_body(
    state: &mut EditorState,
    panel: &crate::widgets::PropertyPanel,
    property_rect: Rect,
    point: Point2D,
    delta_y: f32,
) -> Option<bool> {
    use crate::widgets::property_panel_code;
    if !property_rect.contains(point) {
        return None;
    }
    if matches!(
        state.editor_ui.property_tab,
        op_editor_core::PropertyTab::Code
    ) {
        // Code tab: a wheel over the framework strip scrolls it
        // horizontally (it's a single row), not the panel vertically.
        let (band_top, band_bottom) =
            property_panel_code::framework_row_band(property_rect.origin.y);
        if point.y >= band_top && point.y <= band_bottom {
            let max = property_panel_code::framework_row_overflow(property_rect.size.x);
            let cg = &mut state.codegen;
            return Some(scroll_by_max(&mut cg.framework_scroll, -delta_y, max));
        }
        if property_panel_code::code_preview_rect(property_rect, &state.codegen)
            .is_some_and(|rect| rect.contains(point))
        {
            let max = property_panel_code::code_preview_max_scroll(property_rect, &state.codegen)
                .unwrap_or(0.0);
            let cg = &mut state.codegen;
            return Some(scroll_by_max(&mut cg.code_scroll, -delta_y, max));
        }
    }
    let max = (panel.content_height(property_rect) - property_rect.size.y).max(0.0);
    Some(scroll_by_max(
        &mut state.editor_ui.property_panel_scroll,
        -delta_y,
        max,
    ))
}

/// Scroll the layer rail: the Layers region below `layers_rows_top`,
/// the Pages region above it, each on both axes.
pub fn scroll_layer_panel(
    state: &mut EditorState,
    panel: &LayerPanel,
    rect: Rect,
    point: Point2D,
    delta_x: f32,
    delta_y: f32,
) -> Option<bool> {
    if !state.editor_ui.sidebar_open {
        return None;
    }
    if !rect.contains(point) {
        return None;
    }
    let r = panel.regions(rect);
    let mut changed = false;
    let (vertical, horizontal, max_v, max_h) = if point.y >= r.layers_rows_top {
        (
            &mut state.editor_ui.layer_layers_scroll,
            &mut state.editor_ui.layer_layers_h_scroll,
            r.layers.max_offset,
            r.layers.max_horizontal_offset,
        )
    } else {
        (
            &mut state.editor_ui.layer_pages_scroll,
            &mut state.editor_ui.layer_pages_h_scroll,
            r.pages.max_offset,
            r.pages.max_horizontal_offset,
        )
    };
    if delta_y != 0.0 && scroll_by_max(vertical, -delta_y, max_v) {
        changed = true;
    }
    if delta_x != 0.0 && scroll_by_max(horizontal, -delta_x, max_h) {
        changed = true;
    }
    Some(changed)
}

/// Scroll the layer rail so the selection anchor's row is visible.
/// Returns whether the offset moved.
pub fn reveal_layer_panel_selection(
    state: &mut EditorState,
    panel: &LayerPanel,
    rect: Rect,
) -> bool {
    if !state.editor_ui.sidebar_open {
        return false;
    }
    let selected = state.selection.anchor.clone();
    if !selected.is_real() {
        return false;
    }
    let Some(next) = panel.layers_offset_revealing(rect, &selected) else {
        return false;
    };
    let scroll = &mut state.editor_ui.layer_layers_scroll;
    if (scroll.offset - next).abs() <= f32::EPSILON {
        return false;
    }
    scroll.offset = next;
    true
}
