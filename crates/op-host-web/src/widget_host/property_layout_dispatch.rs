//! Layout-scene-backed size resolution for the web property panel.
//!
//! The layout / sizing / typography writers themselves are shared with
//! the native host in `op_editor_ui::widgets::property_panel_layout_ops`;
//! only this axis probe needs the host's resolved `LayoutScene`.

use super::WidgetHost;

impl WidgetHost {
    pub(in crate::widget_host) fn selected_resolved_size(&mut self, width: bool) -> Option<f64> {
        self.refresh_layout_scene();
        let id = self.editor_state.selection.anchor.as_str();
        self.layout_scene
            .active_page()
            .and_then(|page| page.find(id))
            .map(|node| node.aggregate_bounds())
            .map(|bounds| {
                if width {
                    f64::from(bounds.size.x)
                } else {
                    f64::from(bounds.size.y)
                }
            })
            .filter(|value| value.is_finite() && *value >= 0.0)
    }
}
