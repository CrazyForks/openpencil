//! Figma-style image-fill crop editing for the web canvas host.

use super::WidgetHost;
use op_editor_core::{EditorSnapshot, NodeId};

#[derive(Debug, Clone)]
pub(in crate::widget_host) struct ImageCropDragState {
    node_id: NodeId,
    last_screen_x: f32,
    last_screen_y: f32,
    node_width: f32,
    node_height: f32,
    inverse_transforms: Vec<(f32, bool, bool)>,
    moved: bool,
    pre_drag_snapshot: EditorSnapshot,
}

impl WidgetHost {
    pub(in crate::widget_host) fn enter_selected_image_crop_edit(&mut self) -> bool {
        if !self.editor_state.can_edit_selected_image_crop() {
            return false;
        }
        let id = self.editor_state.selection.anchor.clone();
        let changed = self.editor_state.editor_ui.image_crop_editing.as_ref() != Some(&id);
        self.editor_state.editor_ui.image_crop_editing = Some(id);
        self.node_drag = None;
        if changed {
            self.mark_dirty();
        }
        true
    }

    pub(in crate::widget_host) fn exit_image_crop_edit(&mut self) -> bool {
        let had_drag = self.finish_image_crop_drag();
        let had_edit = self
            .editor_state
            .editor_ui
            .image_crop_editing
            .take()
            .is_some();
        if had_edit {
            self.mark_dirty();
        }
        had_drag || had_edit
    }

    pub(in crate::widget_host) fn start_active_image_crop_drag(
        &mut self,
        target: &NodeId,
        hit_path: &[NodeId],
        x: f32,
        y: f32,
    ) -> bool {
        if self.editor_state.editor_ui.image_crop_editing.as_ref() != Some(target)
            || &self.editor_state.selection.anchor != target
            || !self.editor_state.can_edit_selected_image_crop()
        {
            return false;
        }
        self.refresh_layout_scene();
        let Some(page) = self.layout_scene.active_page() else {
            return false;
        };
        let Some(scene_node) = page.find(target.as_str()) else {
            return false;
        };
        if scene_node.bounds.size.x <= 0.0 || scene_node.bounds.size.y <= 0.0 {
            return false;
        }
        let mut inverse_transforms = Vec::new();
        let mut found_target = false;
        for id in hit_path {
            let Some(node) = page.find(id.as_str()) else {
                continue;
            };
            inverse_transforms.push((node.rotation, node.flip_x, node.flip_y));
            if id == target {
                found_target = true;
                break;
            }
        }
        if !found_target {
            return false;
        }
        self.image_crop_drag = Some(ImageCropDragState {
            node_id: target.clone(),
            last_screen_x: x,
            last_screen_y: y,
            node_width: scene_node.bounds.size.x,
            node_height: scene_node.bounds.size.y,
            inverse_transforms,
            moved: false,
            pre_drag_snapshot: self.editor_state.snapshot_for_history(),
        });
        self.node_drag = None;
        true
    }

    pub(in crate::widget_host) fn apply_image_crop_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> Option<bool> {
        let drag = self.image_crop_drag.as_ref()?;
        if drag.node_id != self.editor_state.selection.anchor {
            self.image_crop_drag = None;
            self.editor_state.editor_ui.image_crop_editing = None;
            self.mark_dirty();
            return Some(true);
        }
        let screen_dx = x - drag.last_screen_x;
        let screen_dy = y - drag.last_screen_y;
        if screen_dx == 0.0 && screen_dy == 0.0 {
            return Some(false);
        }
        let zoom = self.editor_state.viewport.zoom.max(0.0001);
        let doc_dx = screen_dx / zoom;
        let doc_dy = screen_dy / zoom;
        let mut local_dx = doc_dx;
        let mut local_dy = doc_dy;
        for (rotation, flip_x, flip_y) in &drag.inverse_transforms {
            let cos = rotation.cos();
            let sin = rotation.sin();
            let rotated_x = cos * local_dx + sin * local_dy;
            let rotated_y = -sin * local_dx + cos * local_dy;
            local_dx = if *flip_x { -rotated_x } else { rotated_x };
            local_dy = if *flip_y { -rotated_y } else { rotated_y };
        }
        let node_width = drag.node_width;
        let node_height = drag.node_height;
        if let Some(active) = self.image_crop_drag.as_mut() {
            active.last_screen_x = x;
            active.last_screen_y = y;
        }
        self.editor_state.editor_ui.last_canvas_click = None;
        let changed = self.editor_state.translate_selected_image_crop(
            node_width,
            node_height,
            local_dx,
            local_dy,
        );
        if changed {
            if let Some(active) = self.image_crop_drag.as_mut() {
                active.moved = true;
            }
            self.scene_cache.invalidate();
            self.mark_dirty();
        }
        Some(changed)
    }

    pub(in crate::widget_host) fn finish_image_crop_drag(&mut self) -> bool {
        let Some(drag) = self.image_crop_drag.take() else {
            return false;
        };
        if drag.moved {
            self.editor_state.history_push_past(drag.pre_drag_snapshot);
            self.scene_cache.invalidate();
            self.mark_dirty();
        }
        true
    }
}
