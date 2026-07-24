//! Cursor-move dispatch for the web `WidgetHost` — canvas pan-drag,
//! marquee / layer / chat drags, and every hover wash (agent
//! settings, toolbar, top bar, status bar, chat, property panel,
//! code panel, align toolbar). Split out of `widget_host.rs` to keep
//! the spine under the repo's 800-line cap (mirrors the native
//! host's `widget_host/input.rs` split).

use op_editor_ui::widgets::{CanvasViewport, PropertyPanel, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

use super::WidgetHost;

impl WidgetHost {
    // Cursor-move coalescing hint — tested + ready to wire; the CanvasKit mount
    // repaints every mousemove rather than scheduling deferred frames.
    #[allow(dead_code)]
    pub(crate) fn cursor_move_requires_immediate_frame(&self) -> bool {
        let color_picker_drag = self
            .editor_state
            .ui
            .color_picker
            .as_ref()
            .and_then(|state| state.drag)
            .is_some();
        self.variables_resize.is_some()
            || color_picker_drag
            || self.design_md_drag.is_some()
            || self.component_browser_drag.is_some()
            || self.icon_picker_drag.is_some()
            || self.code_selection_drag.is_some()
            || self.image_input_selection_drag.is_some()
            || self.chat_input_selection_drag.is_some()
            || self.chat_text_selection_drag.is_some()
            || self.create_drag.is_some()
            || self.path_anchor_drag.is_some()
            || self.handle_drag.is_some()
            || self.image_crop_drag.is_some()
            || self.node_drag.is_some()
            || self.marquee_drag.is_some()
            || self.layer_drag.is_some()
            || self.chat_drag.is_some()
            || self.image_adjustment_drag.is_some()
            || self.effect_radius_drag.is_some()
            || self.drag.is_some()
    }

    /// Sync every agent-settings hover flag from the cursor.
    /// Returns `true` when any hover state changed.
    pub(in crate::widget_host) fn update_agent_settings_hover(&mut self, x: f32, y: f32) -> bool {
        use op_editor_core::AgentSettingsTab;
        use op_editor_ui::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
        let point = Point2D::new(x, y);
        let (
            close_hover,
            server_hover,
            copy_hover,
            add_provider_hover,
            add_acp_hover,
            image_search_test_hover,
            image_search_register_link_hover,
            image_add_hover,
            image_profile_header_hover,
            image_profile_remove_hover,
            image_profile_provider_hover,
            image_profile_test_hover,
            image_provider_option_hover,
            new_hover,
        ) = {
            let panel = AgentSettingsPanel::for_web_editor(&self.editor_state);
            let panel_rect = panel.rect(self.last_viewport_w, self.last_viewport_h);
            let hit = panel.hit_test(panel_rect, point);
            let is_agents = matches!(
                self.editor_state.editor_ui.agent_settings.tab,
                AgentSettingsTab::Agents
            );
            let is_images = matches!(
                self.editor_state.editor_ui.agent_settings.tab,
                AgentSettingsTab::Images
            );
            let copy_hover = matches!(
                self.editor_state.editor_ui.agent_settings.tab,
                AgentSettingsTab::Mcp
            ) && matches!(hit, AgentSettingsHit::CopyMcpClientConfig);
            let close_hover = matches!(hit, AgentSettingsHit::Close);
            let server_hover = matches!(
                self.editor_state.editor_ui.agent_settings.tab,
                AgentSettingsTab::Mcp
            ) && matches!(hit, AgentSettingsHit::ToggleMcpServer);
            let add_provider_hover = is_agents && matches!(hit, AgentSettingsHit::AddProvider);
            let add_acp_hover = is_agents && matches!(hit, AgentSettingsHit::AddAcpAgent);
            let image_search_test_hover =
                is_images && panel.image_search_test_button_hover_at(panel_rect, point);
            let image_search_register_link_hover =
                is_images && matches!(hit, AgentSettingsHit::OpenImageRegisterLink);
            let image_add_hover =
                is_images && panel.image_gen_add_button_hover_at(panel_rect, point);
            let image_profile_header_hover = if is_images {
                match hit {
                    AgentSettingsHit::ToggleGenConfigEditor(index) => Some(index),
                    _ => None,
                }
            } else {
                None
            };
            let image_profile_remove_hover = if is_images {
                match hit {
                    AgentSettingsHit::RemoveGenConfig(index) => Some(index),
                    _ => None,
                }
            } else {
                None
            };
            let image_profile_provider_hover = if is_images {
                match hit {
                    AgentSettingsHit::ToggleGenProviderMenu(index) => Some(index),
                    _ => None,
                }
            } else {
                None
            };
            let image_profile_test_hover = if is_images {
                panel.image_gen_profile_test_button_hover_at(panel_rect, point)
            } else {
                None
            };
            let image_provider_option_hover = if is_images {
                match hit {
                    AgentSettingsHit::SelectGenProvider { index, provider } => {
                        Some((index, provider))
                    }
                    _ => None,
                }
            } else {
                None
            };
            let new_hover = panel.builtin_preset_hover_at(panel_rect, point);
            (
                close_hover,
                server_hover,
                copy_hover,
                add_provider_hover,
                add_acp_hover,
                image_search_test_hover,
                image_search_register_link_hover,
                image_add_hover,
                image_profile_header_hover,
                image_profile_remove_hover,
                image_profile_provider_hover,
                image_profile_test_hover,
                image_provider_option_hover,
                new_hover,
            )
        };
        let mut changed = false;
        if close_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_agent_settings_close
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_agent_settings_close = close_hover;
            changed = true;
        }
        if server_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_server_button
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_server_button = server_hover;
            changed = true;
        }
        if copy_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_client_config_copy
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_mcp_client_config_copy = copy_hover;
            changed = true;
        }
        if new_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .builtin_preset_menu_hover
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .builtin_preset_menu_hover = new_hover;
            changed = true;
        }
        if add_provider_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_add_provider
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_add_provider = add_provider_hover;
            changed = true;
        }
        if add_acp_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_add_acp_agent
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_add_acp_agent = add_acp_hover;
            changed = true;
        }
        if image_search_test_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_test_button
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_test_button = image_search_test_hover;
            changed = true;
        }
        if image_search_register_link_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_register_link
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_search_register_link = image_search_register_link_hover;
            changed = true;
        }
        if image_add_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_add_button
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_add_button = image_add_hover;
            changed = true;
        }
        if image_profile_header_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_header
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_header = image_profile_header_hover;
            changed = true;
        }
        if image_profile_remove_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_remove
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_remove = image_profile_remove_hover;
            changed = true;
        }
        if image_profile_provider_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_provider
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_provider = image_profile_provider_hover;
            changed = true;
        }
        if image_profile_test_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_test
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_profile_test = image_profile_test_hover;
            changed = true;
        }
        if image_provider_option_hover
            != self
                .editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_provider_option
        {
            self.editor_state
                .editor_ui
                .agent_settings
                .hover_image_gen_provider_option = image_provider_option_hover;
            changed = true;
        }
        let (new_missing_hover, font_picker_hover, font_import_hover) = if matches!(
            self.editor_state.editor_ui.agent_settings.tab,
            AgentSettingsTab::Fonts
        ) {
            use op_editor_ui::widgets::agent_settings_fonts::FontsHit;
            let panel = AgentSettingsPanel::for_editor(&self.editor_state);
            let panel_rect = panel.rect(self.last_viewport_w, self.last_viewport_h);
            match panel.hit_test(panel_rect, point) {
                AgentSettingsHit::Fonts(FontsHit::ChooseFont(row)) => (
                    Some(op_editor_core::missing_fonts::MissingFontsHover::ChooseFile(row)),
                    None,
                    false,
                ),
                AgentSettingsHit::Fonts(FontsHit::RemoveImportedFont(row)) => (
                    Some(op_editor_core::missing_fonts::MissingFontsHover::RemoveImported(row)),
                    None,
                    false,
                ),
                AgentSettingsHit::Fonts(FontsHit::SelectFont(index)) => (None, Some(index), false),
                AgentSettingsHit::Fonts(FontsHit::ImportFont(_)) => (None, None, true),
                _ => (None, None, false),
            }
        } else {
            (None, None, false)
        };
        if new_missing_hover != self.editor_state.editor_ui.missing_fonts_hover {
            self.editor_state.editor_ui.missing_fonts_hover = new_missing_hover;
            changed = true;
        }
        if font_picker_hover != self.editor_state.editor_ui.font_picker.hover {
            self.editor_state.editor_ui.font_picker.hover = font_picker_hover;
            changed = true;
        }
        if font_import_hover != self.editor_state.editor_ui.font_picker_import_hover {
            self.editor_state.editor_ui.font_picker_import_hover = font_import_hover;
            changed = true;
        }
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Cursor-move handler — drives canvas pan-drag, marquee /
    /// layer / chat / overlay drags, and the chrome hover washes.
    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        self.last_cursor_x = x;
        self.last_cursor_y = y;
        // Session-switch owner rotation before the cursor_probe resolve stores
        // the canonical build (mirrors native).
        self.rotate_chat_owner_if_session_changed();
        // Refresh the derived paint doc once up front so every hit-test
        // below (layer context menu, layer drag, align toolbar) reads
        // current geometry, never a stale snapshot.
        self.refresh_layout_scene();
        if self.apply_path_anchor_drag_move(x, y) {
            return true;
        }
        // In-flight VariablesPanel edge resize — owns the cursor.
        if self.variables_resize.is_some()
            && self.apply_variables_panel_resize(x, y, self.last_viewport_w, self.last_viewport_h)
        {
            return true;
        }
        // Missing-fonts modal — owns the cursor while open. Hover the
        // per-row choose-file buttons + the dismiss action.
        if self.editor_state.editor_ui.missing_fonts_modal_open {
            use op_editor_ui::widgets::missing_fonts_panel::MissingFontsPanel;
            let viewport = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(self.last_viewport_w, self.last_viewport_h),
            };
            let (new_hover, picker_hover, import_hover) =
                MissingFontsPanel::for_editor(&self.editor_state)
                    .map(|panel| {
                        let rect = panel.rect(self.last_viewport_w, self.last_viewport_h);
                        if panel.picker_layout(rect, viewport).is_some() {
                            let (entry, import) =
                                panel.picker_hover(rect, viewport, Point2D::new(x, y));
                            (None, entry, import)
                        } else {
                            (
                                op_editor_ui::widgets::editor_state_ext::missing_fonts_button(
                                    panel.hit_test(rect, viewport, Point2D::new(x, y)),
                                ),
                                None,
                                false,
                            )
                        }
                    })
                    .unwrap_or((None, None, false));
            let changed = new_hover != self.editor_state.editor_ui.missing_fonts_hover
                || picker_hover != self.editor_state.editor_ui.font_picker.hover
                || import_hover != self.editor_state.editor_ui.font_picker_import_hover;
            if changed {
                let ui = &mut self.editor_state.editor_ui;
                ui.missing_fonts_hover = new_hover;
                ui.font_picker.hover = picker_hover;
                ui.font_picker_import_hover = import_hover;
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.editor_ui.agent_settings_open {
            return self.update_agent_settings_hover(x, y);
        }
        let picker_open = self.editor_state.editor_ui.chat_model_picker.open;
        let point = Point2D::new(x, y);
        // Cheap transcript-free ownership probe used only to keep lower
        // floating panels and stale higher-hover exits from returning before
        // the exact combined Chat probe below. The exact ownership decision
        // still comes from `cursor_probe.hit` and is resolved once per move.
        let chat_surface_owns_point = !picker_open
            && self
                .ai_chat_rect(self.last_viewport_w, self.last_viewport_h)
                .is_some_and(|rect| {
                    rect.contains(point)
                        || op_editor_ui::widgets::AIChatPlaceholder::from_editor(&self.editor_state)
                            .resize_edge_at(rect, point)
                            .is_some()
                });
        let chat_or_picker_surface_owns_point = picker_open || chat_surface_owns_point;
        let mut upper_hover_changed = false;
        // Property dropdowns are handled by the overlay dispatcher before the
        // base right-rail hover pass. Cache the construction attempt (including
        // a `None` result) so this cursor event never rebuilds the same panel.
        let property_dropdown_open = self.editor_state.editor_ui.fill_type_picker.open
            || self.editor_state.editor_ui.effect_add_picker_open
            || self.editor_state.editor_ui.export_scale_picker_open
            || self.editor_state.editor_ui.export_format_picker_open
            || self.editor_state.editor_ui.interaction_menu_open
            || self.editor_state.editor_ui.padding_mode_popover_open
            || self.editor_state.editor_ui.stroke_mode_popover_open
            || self.editor_state.editor_ui.font_weight_picker_open
            || self.editor_state.editor_ui.font_picker.open
            || self.editor_state.editor_ui.image_fill_popover_open
            || self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open;
        let mut property_panel_probe =
            property_dropdown_open.then(|| PropertyPanel::for_selection(&self.editor_state));
        // Floating-overlay drags + hovers (colour picker, Design-MD /
        // Icon-picker / Component-Browser panels, open dropdowns) own
        // the cursor before lower context menus. This matches native:
        // a topmost panel covering a path-anchor / layer menu must
        // block that lower menu's hover wash.
        let overlay_property_panel = property_panel_probe.as_ref().and_then(Option::as_ref);
        if self.apply_overlay_cursor_move(
            x,
            y,
            overlay_property_panel,
            chat_or_picker_surface_owns_point,
            &mut upper_hover_changed,
        ) {
            return true;
        }
        let over_path_menu = self
            .editor_state
            .ui
            .path_anchor_menu
            .clone()
            .map(|state| {
                op_editor_ui::widgets::path_anchor_context_menu::PathAnchorContextMenu::for_state(
                    &self.editor_state,
                    state,
                )
                .rect()
                .contains(Point2D::new(x, y))
            })
            .unwrap_or(false);
        let path_menu_changed = self.update_path_anchor_menu_hover(x, y);
        if over_path_menu {
            let lower_cleared = self.clear_chat_and_lower_hover();
            return path_menu_changed || lower_cleared;
        }
        if path_menu_changed {
            if chat_or_picker_surface_owns_point {
                upper_hover_changed = true;
            } else {
                return true;
            }
        }
        if let Some(state) = self.editor_state.editor_ui.layer_context_menu.clone() {
            use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.editor_state, state.clone());
            let over_menu = menu.rect().contains(Point2D::new(x, y));
            let new_hover = menu.hovered_row_at(Point2D::new(x, y));
            if new_hover != state.menu.hover {
                let mut next = state;
                next.menu.hover = new_hover;
                self.editor_state.editor_ui.layer_context_menu = Some(next);
                self.mark_dirty();
                if over_menu {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if chat_or_picker_surface_owns_point {
                    upper_hover_changed = true;
                } else {
                    return true;
                }
            }
            if over_menu {
                let lower_cleared = self.clear_chat_and_lower_hover();
                return lower_cleared;
            }
        }
        // Export-section select-popup row hover highlight.
        if self.editor_state.editor_ui.export_scale_picker_open
            || self.editor_state.editor_ui.export_format_picker_open
        {
            if property_panel_probe.is_none() {
                property_panel_probe = Some(PropertyPanel::for_selection(&self.editor_state));
            }
            if let Some(panel) = property_panel_probe.as_ref().and_then(Option::as_ref) {
                let property_rect = Rect {
                    origin: Point2D::new(
                        self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                        op_editor_ui::widgets::TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (self.last_viewport_h - op_editor_ui::widgets::TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                let point = Point2D::new(x, y);
                let over_popup = panel.export_picker_contains(property_rect, point);
                let new_hover = panel.export_picker_row_at(property_rect, point);
                let changed = new_hover != self.editor_state.editor_ui.export_picker_hover;
                if changed {
                    self.editor_state.editor_ui.export_picker_hover = new_hover;
                    self.mark_dirty();
                }
                if over_popup {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if changed {
                    if chat_or_picker_surface_owns_point {
                        upper_hover_changed = true;
                    } else {
                        return true;
                    }
                }
            }
        }
        if let Some(panel) = property_panel_probe.as_ref().and_then(Option::as_ref) {
            let property_rect = Rect {
                origin: Point2D::new(
                    self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.editor_state.editor_ui.property_panel_width,
                    (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let point = Point2D::new(x, y);
            if self.editor_state.editor_ui.interaction_menu_open {
                let new_hover = panel.interaction_menu_row_at(property_rect, point);
                let changed = new_hover != self.editor_state.editor_ui.interaction_menu_hover;
                if changed {
                    self.editor_state.editor_ui.interaction_menu_hover = new_hover;
                    self.mark_dirty();
                }
                let over_popup = !matches!(
                    panel.interaction_menu_hit(property_rect, point),
                    op_editor_ui::widgets::InteractionMenuHit::Outside
                );
                if over_popup {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if changed {
                    if chat_or_picker_surface_owns_point {
                        upper_hover_changed = true;
                    } else {
                        return true;
                    }
                }
            }
            if self.editor_state.editor_ui.padding_mode_popover_open
                || self.editor_state.editor_ui.stroke_mode_popover_open
            {
                use op_editor_ui::widgets::PropertyPanelAction as A;
                let new_hover = match panel.hit_test_action(property_rect, point) {
                    Some(A::SetPaddingMode(mode)) | Some(A::SetStrokeMode(mode)) => {
                        op_editor_core::PaddingEditMode::ALL
                            .iter()
                            .position(|candidate| *candidate == mode)
                    }
                    _ => None,
                };
                let mut changed = false;
                if self.editor_state.editor_ui.padding_mode_popover_open
                    && new_hover != self.editor_state.editor_ui.padding_mode_popover_hover
                {
                    self.editor_state.editor_ui.padding_mode_popover_hover = new_hover;
                    changed = true;
                }
                if self.editor_state.editor_ui.stroke_mode_popover_open
                    && new_hover != self.editor_state.editor_ui.stroke_mode_popover_hover
                {
                    self.editor_state.editor_ui.stroke_mode_popover_hover = new_hover;
                    changed = true;
                }
                if changed {
                    self.mark_dirty();
                }
                let over_popup = (self.editor_state.editor_ui.padding_mode_popover_open
                    && panel.padding_mode_popover_contains(property_rect, point))
                    || (self.editor_state.editor_ui.stroke_mode_popover_open
                        && panel.stroke_mode_popover_contains(property_rect, point));
                if over_popup {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if changed {
                    if chat_or_picker_surface_owns_point {
                        upper_hover_changed = true;
                    } else {
                        return true;
                    }
                }
            }
            if self.editor_state.editor_ui.font_weight_picker_open {
                use op_editor_ui::widgets::PropertyPanelAction as A;
                let new_hover = match panel.hit_test_action(property_rect, point) {
                    Some(A::SetFontWeight(choice)) => op_editor_ui::widgets::FontWeightChoice::ALL
                        .iter()
                        .position(|candidate| *candidate == choice),
                    _ => None,
                };
                let changed = new_hover != self.editor_state.editor_ui.font_weight_picker_hover;
                if changed {
                    self.editor_state.editor_ui.font_weight_picker_hover = new_hover;
                    self.mark_dirty();
                }
                if panel.font_weight_picker_contains(property_rect, point) {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if changed {
                    if chat_or_picker_surface_owns_point {
                        upper_hover_changed = true;
                    } else {
                        return true;
                    }
                }
            }
            if self.editor_state.editor_ui.font_picker.open {
                let over_popup = panel.font_picker_contains(property_rect, point);
                let entry_hover = if over_popup {
                    panel.font_picker_entry_index_at(property_rect, point)
                } else {
                    None
                };
                let import_hover =
                    over_popup && panel.font_picker_import_action_at(property_rect, point);
                let changed = entry_hover != self.editor_state.editor_ui.font_picker.hover
                    || import_hover != self.editor_state.editor_ui.font_picker_import_hover;
                if changed {
                    self.editor_state.editor_ui.font_picker.hover = entry_hover;
                    self.editor_state.editor_ui.font_picker_import_hover = import_hover;
                    self.mark_dirty();
                }
                if over_popup {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if changed {
                    if chat_or_picker_surface_owns_point {
                        upper_hover_changed = true;
                    } else {
                        return true;
                    }
                }
            }
        }
        if self.apply_image_input_selection_drag_cursor_move(x, y) {
            return true;
        }
        if self.apply_chat_text_selection_drag_cursor_move(x, y) {
            return true;
        }
        if self.apply_chat_input_selection_drag_cursor_move(x, y) {
            return true;
        }
        if self.apply_code_selection_drag_cursor_move(x, y) {
            return true;
        }
        if let Some(consumed) = self.apply_image_crop_drag_cursor_move(x, y) {
            return consumed;
        }
        if let Some(consumed) = self.apply_node_drag_cursor_move(x, y) {
            return consumed;
        }
        if self.apply_selection_handle_drag_move(x, y) {
            return true;
        }
        if self.update_create_drag(x, y) {
            return true;
        }
        if let Some(m) = self.marquee_drag.as_mut() {
            m.current_screen_x = x;
            m.current_screen_y = y;
            return true;
        }
        if self.layer_drag.is_some() {
            // Drop the gesture if the source disappeared mid-drag —
            // see the native host for the rationale.
            let source_id = self.layer_drag.as_ref().unwrap().source.clone();
            let still_present = self
                .layout_scene
                .active_page()
                .map(|p| p.find(source_id.as_str()).is_some())
                .unwrap_or(false);
            if !still_present {
                self.layer_drag = None;
                return true;
            }
            let d = self.layer_drag.as_mut().unwrap();
            d.current_x = x;
            d.current_y = y;
            // VERTICAL-ONLY activation (4 px). See the native host
            // for the rationale: pure horizontal wiggle must not
            // steal click-feel from row-level gestures.
            if !d.active && (y - d.start_y).abs() > 4.0 {
                d.active = true;
            }
            return true;
        }
        if let Some(d) = self.chat_drag.as_mut() {
            d.pos_x = x - d.grab_dx;
            d.pos_y = y - d.grab_dy;
            return true;
        }
        if let Some(field) = self.image_adjustment_drag {
            if property_panel_probe.is_none() {
                property_panel_probe = Some(PropertyPanel::for_selection(&self.editor_state));
            }
            if let Some(panel) = property_panel_probe.as_ref().and_then(Option::as_ref) {
                let property_rect = Rect {
                    origin: Point2D::new(
                        self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                        op_editor_ui::widgets::TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (self.last_viewport_h - op_editor_ui::widgets::TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                if let Some(action) = panel.image_adjustment_drag_action(property_rect, field, x) {
                    self.apply_property_action(action);
                    return true;
                }
            }
        }
        if let Some(effect) = self.effect_radius_drag {
            if property_panel_probe.is_none() {
                property_panel_probe = Some(PropertyPanel::for_selection(&self.editor_state));
            }
            if let Some(panel) = property_panel_probe.as_ref().and_then(Option::as_ref) {
                let property_rect = Rect {
                    origin: Point2D::new(
                        self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                        TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                if let Some(action) = panel.effect_radius_drag_action(property_rect, effect, x) {
                    self.apply_property_action(action);
                    return true;
                }
            }
        }
        if let Some(drag) = self.drag.as_mut() {
            let dx = x - drag.last_x;
            let dy = y - drag.last_y;
            drag.last_x = x;
            drag.last_y = y;
            self.editor_state.viewport.pan(dx, dy);
            // Canvas pan only translates the viewport; the layout-resolved
            // scene is document-space (camera applied at paint time), so keep
            // the layout cache intact — `mark_dirty()` here forced a full
            // serde reconversion of the document every move (matches native
            // `op-host-native/.../input.rs`). The listener still repaints off
            // this `true` return.
            return true;
        }
        // Variables is painted below Chat even though the shared historical
        // `over_topmost_panel` helper groups it with floating panels. Only the
        // three panels painted after Chat may suppress ordinary Chat ownership.
        let over_topmost = !picker_open
            && (self
                .design_md_panel_rect(self.last_viewport_w, self.last_viewport_h)
                .is_some_and(|rect| rect.contains(point))
                || self
                    .icon_picker_panel_rect(self.last_viewport_w, self.last_viewport_h)
                    .is_some_and(|rect| rect.contains(point))
                || self
                    .component_browser_panel_rect(self.last_viewport_w, self.last_viewport_h)
                    .is_some_and(|rect| rect.contains(point)));
        // StatusBar paints and presses above Chat. Preserve its ownership
        // before the model picker truncates LayerPanel / toolbar / base UI.
        if let Some(status_rect) = self.status_bar_rect(self.last_viewport_w, self.last_viewport_h)
        {
            let over_status = !over_topmost && status_rect.contains(point);
            let new_hover = if over_topmost {
                None
            } else {
                op_editor_ui::widgets::StatusBar::for_editor(&self.editor_state)
                    .control_at(status_rect, point)
            };
            if new_hover != self.editor_state.editor_ui.statusbar_hover {
                self.editor_state.editor_ui.statusbar_hover = new_hover;
                self.mark_dirty();
                if over_status {
                    self.clear_chat_and_lower_hover();
                    return true;
                }
                if chat_or_picker_surface_owns_point {
                    upper_hover_changed = true;
                } else {
                    return true;
                }
            }
            if over_status {
                let lower_cleared = self.clear_chat_and_lower_hover();
                return lower_cleared;
            }
        }
        // AlignToolbar paints above both the regular Chat card and its model
        // picker. Its whole opaque pill (including padding and group gutters)
        // receives first refusal, not only the action buttons.
        let (new_align_hover, align_owns_point) = if self.editor_state.selection_count() >= 2 {
            use op_editor_ui::widgets::{AlignToolbar, TOP_BAR_HEIGHT};
            let (cx, _, cw, ch) = self.canvas_region(self.last_viewport_w, self.last_viewport_h);
            let canvas_region = op_editor_ui::Rect {
                origin: Point2D::new(cx, TOP_BAR_HEIGHT),
                size: Point2D::new(cw, ch),
            };
            AlignToolbar::for_canvas_region(canvas_region, &self.editor_state)
                .map(|toolbar| (toolbar.hit_test(point), toolbar.rect().contains(point)))
                .unwrap_or((None, false))
        } else {
            (None, false)
        };
        let mut retained_hover_changed = upper_hover_changed;
        if new_align_hover != self.editor_state.editor_ui.align_toolbar_hover {
            self.editor_state.editor_ui.align_toolbar_hover = new_align_hover;
            self.mark_dirty();
            retained_hover_changed = true;
        }
        if picker_open && align_owns_point {
            let lower_changed = self.clear_chat_and_lower_hover();
            return retained_hover_changed || lower_changed;
        }
        // The visible chat model picker owns every lower hover surface. Keep
        // higher overlays and active drags above it, then stop before the
        // LayerPanel / toolbar / base chat and property probes.
        if self.editor_state.editor_ui.chat_model_picker.open {
            let Some(picker) =
                self.chat_model_picker_rect(self.last_viewport_w, self.last_viewport_h)
            else {
                // A hidden chat (embed/collapse) or a viewport too small to
                // lay it out must not leave an invisible modal lock behind.
                self.editor_state.editor_ui.close_chat_model_picker();
                self.mark_dirty();
                return true;
            };
            let hover_changed = self.update_chat_model_picker_hover(x, y, picker);
            let lower_changed = self.clear_hover_below_chat_model_picker();
            return upper_hover_changed || hover_changed || lower_changed;
        }
        // TopBar chrome-button hover wash (sidebar / file-menu / figma /
        // theme / locale / fullscreen / agent chip). Git and Preview are
        // compiled out on wasm32; every visible button lights up the same
        // as native. Reuses the click hit-test so paint can't drift.
        {
            let tb_rect = self.top_bar_rect(self.last_viewport_w);
            let new_hover = (!chat_surface_owns_point)
                .then(|| self.top_bar().hit_test(tb_rect, point))
                .flatten()
                .map(op_editor_ui::widgets::editor_state_ext::topbar_button_hover);
            if new_hover != self.editor_state.editor_ui.topbar_button_hover {
                self.editor_state.editor_ui.topbar_button_hover = new_hover;
                self.mark_dirty();
                if chat_surface_owns_point {
                    retained_hover_changed = true;
                } else {
                    return true;
                }
            }
        }
        // Construct the chat panel once for this cursor event and resolve all
        // hover results from that instance. Besides fingerprinting the
        // transcript once, this avoids rebuilding translated labels and tabs
        // for each sub-control.
        let (chat_probe, chat_tab_hover, chat_footer_hover, parallel_hover, example_hover) =
            if let Some(chat_rect) = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h) {
                use op_editor_ui::widgets::AIChatPlaceholder;
                let panel = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                    .owned_by(self.chat_panel_owner);
                (
                    Some(panel.cursor_probe(chat_rect, point)),
                    panel.tab_hover_at(chat_rect, point),
                    panel.footer_hover_at(chat_rect, point),
                    panel.parallel_agents_picker_hover_at(chat_rect, point),
                    panel.example_hover_at(chat_rect, point),
                )
            } else {
                (None, None, None, None, None)
            };
        let chat_owns_point = !over_topmost
            && !align_owns_point
            && chat_probe.as_ref().is_some_and(|probe| probe.hit.is_some());
        let chat_header_hover = if !over_topmost && !align_owns_point {
            chat_probe
                .as_ref()
                .and_then(|probe| probe.hit.as_ref())
                .and_then(op_editor_ui::widgets::editor_state_ext::chat_header_hover)
        } else {
            None
        };
        let (chat_tab_hover, chat_footer_hover, parallel_hover, example_hover) =
            if !over_topmost && !align_owns_point {
                (
                    chat_tab_hover,
                    chat_footer_hover,
                    parallel_hover,
                    example_hover,
                )
            } else {
                (None, None, None, None)
            };
        let design_hover = if !over_topmost && !align_owns_point {
            chat_probe
                .as_ref()
                .and_then(|probe| probe.design_block_hover)
        } else {
            None
        };
        // Update every Chat hover from the same probe before deciding whether
        // the panel owns the move. A control transition must not prevent the
        // panel's blank body from blocking lower layers.
        let mut chat_hover_changed = false;
        {
            let ui = &mut self.editor_state.editor_ui;
            if chat_header_hover != ui.chat_header_hover {
                ui.chat_header_hover = chat_header_hover;
                chat_hover_changed = true;
            }
            if chat_tab_hover != ui.chat_tab_hover {
                ui.chat_tab_hover = chat_tab_hover;
                chat_hover_changed = true;
            }
            if chat_footer_hover != ui.chat_footer_hover {
                ui.chat_footer_hover = chat_footer_hover;
                chat_hover_changed = true;
            }
            if parallel_hover != ui.parallel_agents_picker_hover {
                ui.parallel_agents_picker_hover = parallel_hover;
                chat_hover_changed = true;
            }
            if example_hover != ui.chat_example_hover {
                ui.chat_example_hover = example_hover;
                chat_hover_changed = true;
            }
        }
        if chat_hover_changed {
            self.mark_dirty();
        }
        chat_hover_changed |= self.apply_chat_design_hover(design_hover);
        if chat_owns_point || align_owns_point {
            let lower_changed = self.clear_hover_below_chat_panel();
            return retained_hover_changed || chat_hover_changed || lower_changed;
        }

        // Chat sits above these base surfaces. They are dispatched only after
        // the combined Chat ownership probe has missed.
        if self.update_layer_hover(x, y, self.last_viewport_h) {
            return true;
        }
        if self.update_toolbar_hover(x, y) {
            return true;
        }
        if self.editor_state.editor_ui.variables_panel_open {
            use op_editor_ui::widgets::variables_panel::VariablesPanel;
            if let Some(vars_rect) =
                self.variables_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                if (vars_rect).contains(point) {
                    let new_hover = VariablesPanel::for_editor_at(&self.editor_state, self.now_ms)
                        .hover_at(vars_rect, point);
                    let changed = new_hover != self.editor_state.editor_ui.variables_panel_hover;
                    if changed {
                        self.editor_state.editor_ui.variables_panel_hover = new_hover;
                        self.mark_dirty();
                    }
                    return retained_hover_changed || chat_hover_changed || changed;
                }
            }
            if self
                .editor_state
                .editor_ui
                .variables_panel_hover
                .take()
                .is_some()
            {
                self.mark_dirty();
                return true;
            }
        } else if self
            .editor_state
            .editor_ui
            .variables_panel_hover
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        // PropertyPanel tab/action hover wash. Shown with a selection.
        let mut property_hover_changed = false;
        let property_rect = Rect {
            origin: Point2D::new(
                self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                TOP_BAR_HEIGHT,
            ),
            size: Point2D::new(
                self.editor_state.editor_ui.property_panel_width,
                (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0),
            ),
        };
        let point = Point2D::new(x, y);
        let over_property_panel = self.editor_state.property_panel_visible()
            && !over_topmost
            && property_rect.contains(point);
        let needs_property_probe = self.editor_state.property_panel_visible()
            && !over_topmost
            && (over_property_panel
                || self.editor_state.editor_ui.fill_type_picker.open
                || self.editor_state.editor_ui.compositing_picker.open);
        if needs_property_probe && property_panel_probe.is_none() {
            property_panel_probe = Some(PropertyPanel::for_selection(&self.editor_state));
        }
        let property_panel = if needs_property_probe {
            property_panel_probe.as_ref().and_then(Option::as_ref)
        } else {
            None
        };
        let inside_property_panel = over_property_panel && property_panel.is_some();
        if let Some(panel) = property_panel {
            let new_tab_hover = panel.tab_hover_at(property_rect, point);
            if new_tab_hover != self.editor_state.editor_ui.property_tab_hover {
                self.editor_state.editor_ui.property_tab_hover = new_tab_hover;
                property_hover_changed = true;
            }
            let new_fill_type_hover = panel.fill_type_picker_row_at(property_rect, point);
            if new_fill_type_hover != self.editor_state.editor_ui.fill_type_picker.hover {
                self.editor_state.editor_ui.fill_type_picker.hover = new_fill_type_hover;
                property_hover_changed = true;
            }
            let new_compositing_hover = panel.compositing_picker_row_at(property_rect, point);
            if new_compositing_hover != self.editor_state.editor_ui.compositing_picker.hover {
                self.editor_state.editor_ui.compositing_picker.hover = new_compositing_hover;
                property_hover_changed = true;
            }
            let new_action_hover = panel.action_hover_index(property_rect, point);
            if new_action_hover != self.editor_state.editor_ui.property_action_hover {
                self.editor_state.editor_ui.property_action_hover = new_action_hover;
                property_hover_changed = true;
            }
        } else {
            let ui = &mut self.editor_state.editor_ui;
            property_hover_changed |= ui.property_tab_hover.take().is_some();
            property_hover_changed |= ui.fill_type_picker.hover.take().is_some();
            property_hover_changed |= ui.compositing_picker.hover.take().is_some();
            property_hover_changed |= ui.property_action_hover.take().is_some();
        }
        // Code-panel hover wash. Reuses the panel's click geometry for
        // framework chips, scroll chevrons, and body actions.
        let (new_fw_hover, new_action_hover) = if self.editor_state.property_panel_visible()
            && matches!(
                self.editor_state.editor_ui.property_tab,
                op_editor_core::PropertyTab::Code
            ) {
            let pw = self.editor_state.editor_ui.property_panel_width;
            let panel_x = self.last_viewport_w - pw;
            let panel_rect = Rect {
                origin: Point2D::new(panel_x, TOP_BAR_HEIGHT),
                size: Point2D::new(pw, (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0)),
            };
            if x >= panel_x && x <= self.last_viewport_w {
                op_editor_ui::widgets::property_panel_code::code_hover_at_with_locale(
                    panel_rect,
                    &self.editor_state.codegen,
                    Point2D::new(x, y),
                    self.editor_state.editor_ui.locale,
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        if new_fw_hover != self.editor_state.codegen.framework_hover
            || new_action_hover != self.editor_state.codegen.action_hover
        {
            self.editor_state.codegen.framework_hover = new_fw_hover;
            self.editor_state.codegen.action_hover = new_action_hover;
            self.mark_dirty();
            return true;
        }
        if inside_property_panel {
            let lower_hover_changed = self.clear_hover_below_property_panel();
            if property_hover_changed && !lower_hover_changed {
                self.mark_dirty();
            }
            return true;
        }
        if property_hover_changed {
            self.mark_dirty();
            return true;
        }
        // Canvas hierarchy hover. The scene hit path is resolved to
        // the current level's primary target; shared canvas paint then
        // draws that node solid plus all of its direct children dashed.
        // Frame labels participate as explicit root targets.
        let hover_eligible = !over_topmost
            && matches!(self.editor_state.tool, op_editor_core::Tool::Select)
            && self.over_canvas(x, y, self.last_viewport_w, self.last_viewport_h);
        let new_canvas_hover = if hover_eligible {
            let (cx0, cy0, cw, ch) = self.canvas_region(self.last_viewport_w, self.last_viewport_h);
            let canvas_rect = Rect {
                origin: Point2D::new(cx0, cy0),
                size: Point2D::new(cw, ch),
            };
            let canvas = CanvasViewport::from_editor(&self.editor_state, &self.layout_scene);
            if let Some(root) = canvas.frame_label_at_point(canvas_rect, Point2D::new(x, y)) {
                Some(op_editor_core::NodeId::new(root))
            } else {
                let local = Point2D::new(x - cx0, y - cy0);
                let doc = self.editor_state.viewport.to_document(local);
                self.layout_scene
                    .node_path_at_doc_point(doc, self.editor_state.viewport.zoom)
                    .and_then(|path| {
                        let path = path
                            .into_iter()
                            .map(op_editor_core::NodeId::new)
                            .collect::<Vec<_>>();
                        op_editor_core::selection_resolve::resolve_canvas_depth_targets(
                            &path,
                            self.editor_state.editor_ui.entered_container.as_ref(),
                        )
                        .map(|targets| targets.primary)
                    })
            }
        } else {
            None
        };
        if new_canvas_hover != self.editor_state.editor_ui.canvas_hover_node {
            self.editor_state.editor_ui.canvas_hover_node = new_canvas_hover;
            self.mark_dirty();
            return true;
        }
        retained_hover_changed || chat_hover_changed
    }

    fn clear_hover_below_property_panel(&mut self) -> bool {
        let mut changed = false;
        {
            let ui = &mut self.editor_state.editor_ui;
            changed |= ui.canvas_hover_node.take().is_some();
            changed |= ui.hovered_layer_id.take().is_some();
            changed |= ui.hovered_page_index.take().is_some();
            changed |= ui.toolbar_hover.take().is_some();
            changed |= ui.align_toolbar_hover.take().is_some();
            changed |= ui.statusbar_hover.take().is_some();
            changed |= ui.chat_design_block_hover.take().is_some();
            changed |= ui.chat_footer_hover.take().is_some();
            changed |= ui.chat_example_hover.take().is_some();
            changed |= ui.chat_tab_hover.take().is_some();
            if let Some(menu) = ui.layer_context_menu.as_mut() {
                changed |= menu.menu.hover.take().is_some();
            }
        }
        if let Some(menu) = self.editor_state.ui.path_anchor_menu.as_mut() {
            changed |= menu.menu.hover.take().is_some();
        }
        if changed {
            self.mark_dirty();
        }
        changed
    }
}
