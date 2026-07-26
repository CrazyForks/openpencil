//! PropertyPanel compositing-select dispatch: maps the widget-facade
//! action/target types onto the shared `EditorUiState` transitions
//! (`op_editor_core::host_ui_transitions`).

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
            | PropertyPanelAction::SetImageFillMode(_)
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
        self.editor_state.editor_ui.close_compositing_picker();
    }

    pub(in crate::widget_host) fn toggle_compositing_picker(&mut self, target: CompositingTarget) {
        self.editor_state
            .editor_ui
            .toggle_compositing_picker(core_target(target));
    }
}
