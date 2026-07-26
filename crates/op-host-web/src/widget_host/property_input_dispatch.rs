//! Web property-input commits and focus-value formatting.

use super::super::WidgetHost;
use op_editor_core::PropertyFocus;
use op_editor_ui::util::{
    color_to_hex, color_to_hex_with_alpha, format_panel_number, format_panel_number_roundtrip,
};
use op_editor_ui::widgets::property_panel_commit as commit;

impl WidgetHost {
    /// Commit the floating image-fill editor's numeric draft before an action
    /// hides or replaces that editor. Keeping this guard here gives every
    /// close path the same focus/draft cleanup without disturbing unrelated
    /// property inputs.
    pub(in crate::widget_host) fn commit_image_tile_scale_focus_if_any(&mut self) -> bool {
        if self.editor_state.ui.property_focus != Some(PropertyFocus::ImageTileScale) {
            return false;
        }
        self.commit_property_focus_if_any();
        true
    }

    pub(in crate::widget_host) fn commit_effect_param_focus_if_any(&mut self) {
        if commit::commit_effect_param_focus(&mut self.editor_state) {
            self.mark_dirty();
        }
    }

    pub(in crate::widget_host) fn commit_property_focus_if_any(&mut self) {
        self.commit_variables_panel_header_focus_if_any();
        self.commit_variable_row_focus_if_any();
        self.commit_effect_param_focus_if_any();
        if commit::commit_property_focus(&mut self.editor_state) {
            self.mark_dirty();
        }
    }
}

pub(in crate::widget_host) fn property_focus_initial(
    focus: PropertyFocus,
    panel: &op_editor_ui::widgets::PropertyPanel,
) -> String {
    match focus {
        PropertyFocus::PageBackgroundHex => panel.page_background.clone().unwrap_or_default(),
        PropertyFocus::PositionX => panel.snapshot.x.to_string(),
        PropertyFocus::PositionY => panel.snapshot.y.to_string(),
        PropertyFocus::SizeW => panel.snapshot.width.to_string(),
        PropertyFocus::SizeH => panel.snapshot.height.to_string(),
        PropertyFocus::LayoutGap => format_panel_number(panel.snapshot.layout_gap),
        PropertyFocus::PaddingTop
        | PropertyFocus::PaddingRight
        | PropertyFocus::PaddingBottom
        | PropertyFocus::PaddingLeft => panel
            .snapshot
            .layout_padding
            .value_for(focus)
            .map(format_panel_number)
            .unwrap_or_else(|| "0".to_string()),
        PropertyFocus::Rotation => (panel.snapshot.rotation_deg.round() as i32).to_string(),
        PropertyFocus::PositionR => {
            if op_editor_ui::widgets::property_panel_corner::radii_are_uniform(
                panel.snapshot.corner_radii,
            ) {
                (panel.snapshot.corner_radius.round() as i32).to_string()
            } else {
                String::new()
            }
        }
        PropertyFocus::CornerTL
        | PropertyFocus::CornerTR
        | PropertyFocus::CornerBL
        | PropertyFocus::CornerBR => op_editor_ui::widgets::property_panel_corner::value_for_focus(
            panel.snapshot.corner_radii,
            focus,
        )
        .map(format_panel_number)
        .unwrap_or_else(|| "0".to_string()),
        PropertyFocus::EffectRadius(index) => panel
            .snapshot
            .effects
            .get(index)
            .map(|effect| format_panel_number(effect.blur))
            .unwrap_or_else(|| "0".to_string()),
        PropertyFocus::Opacity => format_panel_number(panel.snapshot.opacity_percent),
        PropertyFocus::PolygonSides => panel.snapshot.polygon_sides.unwrap_or(3).to_string(),
        PropertyFocus::EllipseStart => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.start_deg)
                .unwrap_or(0.0),
        ),
        PropertyFocus::EllipseSweep => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.sweep_deg)
                .unwrap_or(360.0),
        ),
        PropertyFocus::EllipseInnerRadius => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.inner_percent)
                .unwrap_or(0.0),
        ),
        PropertyFocus::FontSize => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.font_size))
            .unwrap_or_else(|| "16".to_string()),
        PropertyFocus::FontWeight => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| t.font_weight.to_string())
            .unwrap_or_else(|| "400".to_string()),
        PropertyFocus::LineHeight => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.line_height_percent))
            .unwrap_or_else(|| "120".to_string()),
        PropertyFocus::LetterSpacing => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.letter_spacing))
            .unwrap_or_else(|| "0".to_string()),
        PropertyFocus::FillOpacity(index) => {
            let opacity = panel
                .snapshot
                .fills
                .get(index)
                .map(|f| f.opacity)
                .unwrap_or(panel.snapshot.fill_opacity);
            ((opacity * 100.0).round() as i32).to_string()
        }
        PropertyFocus::ImageTileScale => panel
            .snapshot
            .image_fill
            .as_ref()
            .and_then(|image| image.tile_scale)
            .map(format_panel_number_roundtrip)
            .unwrap_or_else(|| "1".to_string()),
        PropertyFocus::FillHex(index) => panel
            .snapshot
            .fills
            .get(index)
            .map(|f| f.color)
            .or(panel.snapshot.fill)
            .map(color_to_hex_with_alpha)
            .unwrap_or_else(|| "#FFFFFF".to_string()),
        PropertyFocus::StrokeHex => color_to_hex(panel.snapshot.stroke_swatch_color()),
        // Seed the SAME width the inline input paints (0 when unset, and
        // un-rounded) so clicking in never changes the displayed value.
        PropertyFocus::StrokeWidth => {
            format_panel_number(panel.snapshot.stroke.map(|s| s.width).unwrap_or(0.0))
        }
        PropertyFocus::StrokeTopWidth
        | PropertyFocus::StrokeRightWidth
        | PropertyFocus::StrokeBottomWidth
        | PropertyFocus::StrokeLeftWidth => panel
            .snapshot
            .stroke_side_width_for(focus)
            .map(format_panel_number)
            .unwrap_or_else(|| "0".to_string()),
        PropertyFocus::GradientAngle => {
            let a = panel.snapshot.gradient_angle.unwrap_or(0.0);
            if a.fract() == 0.0 {
                format!("{}", a as i32)
            } else {
                format!("{a}")
            }
        }
        PropertyFocus::GradientStopHex(i) => panel
            .snapshot
            .gradient_stops
            .get(i)
            .map(|s| op_editor_ui::widgets::property_panel_fill::stop_hex_rgb_only(&s.hex))
            .unwrap_or_else(|| "#000000".to_string()),
        PropertyFocus::GradientStopOffset(i) => panel
            .snapshot
            .gradient_stops
            .get(i)
            .map(|s| ((s.offset * 100.0).round() as i32).to_string())
            .unwrap_or_else(|| "0".to_string()),
        // Widget-section fields — seed the input draft from the selected
        // widget's current value so editing starts from it (mirrors the
        // native `property_focus_initial`).
        PropertyFocus::WidgetPlaceholder => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.placeholder.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetValue => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.value.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetLabel => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.label.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetLeadingIcon => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.leading_icon.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetTrailingIcon => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.trailing_icon.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetBindKey => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.bind_key.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetMin => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.min.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetMax => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.max.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetStep => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.step.clone())
            .unwrap_or_default(),
    }
}
