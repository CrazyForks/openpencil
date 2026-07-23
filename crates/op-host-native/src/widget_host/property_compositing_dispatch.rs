//! Shared state transitions for PropertyPanel compositing selects.

use super::super::WidgetHostNative;
use op_editor_core::CompositingPickerTarget;
use op_editor_ui::widgets::property_panel_action::CompositingTarget;
use op_editor_ui::widgets::PropertyPanelAction;

pub(super) fn updates_document(action: &PropertyPanelAction) -> bool {
    matches!(
        action,
        PropertyPanelAction::SetNodeBlendMode(_)
            | PropertyPanelAction::SetNodeMaskType(_)
            | PropertyPanelAction::SetFillBlendMode { .. }
            | PropertyPanelAction::ClearPageBackground
    )
}

fn core_target(target: CompositingTarget) -> CompositingPickerTarget {
    match target {
        CompositingTarget::NodeBlend => CompositingPickerTarget::NodeBlend,
        CompositingTarget::NodeMask => CompositingPickerTarget::NodeMask,
        CompositingTarget::FillBlend(index) => CompositingPickerTarget::FillBlend(index),
    }
}

impl WidgetHostNative {
    pub(in crate::widget_host) fn close_compositing_picker(&mut self) {
        let ui = &mut self.editor_state.editor_ui;
        ui.compositing_picker.open = false;
        ui.compositing_picker.hover = None;
        ui.compositing_picker.pressed = None;
        ui.compositing_picker_target = None;
    }

    pub(in crate::widget_host) fn toggle_compositing_picker(&mut self, target: CompositingTarget) {
        let target = core_target(target);
        let ui = &mut self.editor_state.editor_ui;
        let opening =
            !ui.compositing_picker.open || ui.compositing_picker_target.as_ref() != Some(&target);
        ui.compositing_picker.open = opening;
        ui.compositing_picker.hover = None;
        ui.compositing_picker.pressed = None;
        ui.compositing_picker.scroll.offset = 0.0;
        ui.compositing_picker_target = opening.then_some(target);
        if opening {
            ui.close_fill_type_picker();
            ui.image_fill_popover_open = false;
            ui.close_font_picker();
            ui.font_weight_picker_open = false;
            ui.padding_mode_popover_open = false;
            ui.stroke_mode_popover_open = false;
            ui.export_scale_picker_open = false;
            ui.export_format_picker_open = false;
            ui.property_color_variable_picker_open = None;
        }
    }
}
