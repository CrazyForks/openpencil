//! Hit-testing + hover / geometry queries for [`AgentSettingsPanel`] —
//! `hit_test` (the tab-dispatching click router) plus the per-surface
//! `*_at` hover probes and content-height maths. Carved off
//! `agent_settings_panel.rs` to keep every file under the 800-line cap.

use super::paint::agents_content_height;
use super::*;

impl AgentSettingsPanel<'_> {
    pub fn hit_test(&self, panel: Rect, point: Point2D) -> AgentSettingsHit {
        if !(panel).contains(point) {
            return AgentSettingsHit::Outside;
        }
        if (close_rect(panel)).contains(point) {
            return AgentSettingsHit::Close;
        }
        for (i, tab) in self.mode.visible_tabs().iter().enumerate() {
            if (nav_item_rect(panel, i)).contains(point) {
                return AgentSettingsHit::SelectTab(*tab);
            }
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        match self.mode.active_tab(&self.settings) {
            AgentSettingsTab::Agents => {
                match agent_settings_builtin::hit_test(
                    content_rect(panel),
                    &self.settings,
                    scrolled,
                ) {
                    BuiltinHit::AddProvider => return AgentSettingsHit::AddProvider,
                    BuiltinHit::Focus { index, field } => {
                        return AgentSettingsHit::FocusBuiltinAgent { index, field };
                    }
                    BuiltinHit::FocusDraft(field) => {
                        return AgentSettingsHit::FocusBuiltinAgentDraft(field);
                    }
                    BuiltinHit::ToggleKind(index) => {
                        return AgentSettingsHit::ToggleBuiltinAgentKind(index);
                    }
                    BuiltinHit::ToggleDraftKind => {
                        return AgentSettingsHit::ToggleBuiltinAgentDraftKind;
                    }
                    BuiltinHit::TogglePresetMenu(index) => {
                        return AgentSettingsHit::ToggleBuiltinAgentPresetMenu(index);
                    }
                    BuiltinHit::SelectPreset { index, preset } => {
                        return AgentSettingsHit::SelectBuiltinAgentPreset { index, preset };
                    }
                    BuiltinHit::SaveDraft => return AgentSettingsHit::SaveBuiltinAgentDraft,
                    BuiltinHit::CancelDraft => return AgentSettingsHit::CancelBuiltinAgentDraft,
                    BuiltinHit::ToggleEnabled(index) => {
                        return AgentSettingsHit::ToggleBuiltinAgentEnabled(index);
                    }
                    BuiltinHit::Edit(index) => {
                        return AgentSettingsHit::EditBuiltinAgent(index);
                    }
                    BuiltinHit::Remove(index) => {
                        return AgentSettingsHit::RemoveBuiltinAgent(index);
                    }
                    BuiltinHit::None => {}
                }
                if !self.mode.shows_external_agents() {
                    return AgentSettingsHit::Inside;
                }
                let content = content_rect(panel);
                let acp_y = acp_section_y(content, &self.settings);
                match agent_settings_acp::hit_test(content, &self.settings, scrolled, acp_y) {
                    AcpHit::AddAgent => return AgentSettingsHit::AddAcpAgent,
                    AcpHit::AddPreset(index) => {
                        return AgentSettingsHit::AddAcpPreset(index);
                    }
                    AcpHit::Focus { index, field } => {
                        return AgentSettingsHit::FocusAcpAgent { index, field };
                    }
                    AcpHit::FocusDraft(field) => {
                        return AgentSettingsHit::FocusAcpAgentDraft(field)
                    }
                    AcpHit::SaveDraft => return AgentSettingsHit::SaveAcpAgentDraft,
                    AcpHit::CancelDraft => return AgentSettingsHit::CancelAcpAgentDraft,
                    AcpHit::Edit(index) => return AgentSettingsHit::EditAcpAgent(index),
                    AcpHit::Remove(index) => return AgentSettingsHit::RemoveAcpAgent(index),
                    AcpHit::ToggleConnected(index) => {
                        return AgentSettingsHit::ToggleAcpConnected(index);
                    }
                    AcpHit::None => {}
                }
                for (i, provider) in AgentProvider::ALL.iter().enumerate() {
                    let card = agent_card_rect_in(panel, i, &self.settings);
                    if !(card).contains(scrolled) {
                        continue;
                    }
                    if self.settings.provider_verified_connected_at(i) {
                        let disc = disconnect_btn_rect_at(card);
                        if (disc).contains(scrolled) {
                            return AgentSettingsHit::Connect(*provider);
                        }
                    } else if (connect_btn_rect_at(card)).contains(scrolled) {
                        return AgentSettingsHit::Connect(*provider);
                    }
                }
            }
            AgentSettingsTab::Mcp => {
                match agent_settings_mcp::hit_test(content_rect(panel), &self.settings, scrolled) {
                    McpHit::ToggleServer => return AgentSettingsHit::ToggleMcpServer,
                    McpHit::ToggleCli(cli) => return AgentSettingsHit::ToggleMcpCli(cli),
                    McpHit::CopyClientConfig => return AgentSettingsHit::CopyMcpClientConfig,
                    McpHit::FocusPort => return AgentSettingsHit::FocusMcpPort,
                    McpHit::None => {}
                }
            }
            AgentSettingsTab::Images => {
                match agent_settings_images::hit_test(content_rect(panel), &self.settings, scrolled)
                {
                    ImagesHit::ToggleAdvanced => return AgentSettingsHit::ToggleImagesAdvanced,
                    ImagesHit::FocusSearchField(field) => {
                        return AgentSettingsHit::FocusSearchField(field);
                    }
                    ImagesHit::OpenRegisterLink => {
                        return AgentSettingsHit::OpenImageRegisterLink;
                    }
                    ImagesHit::TestSearch => return AgentSettingsHit::TestImageSearch,
                    ImagesHit::AddGenConfig => return AgentSettingsHit::AddGenConfig,
                    ImagesHit::ToggleGenConfigEditor(index) => {
                        return AgentSettingsHit::ToggleGenConfigEditor(index);
                    }
                    ImagesHit::SetActiveGenConfig(index) => {
                        return AgentSettingsHit::SetActiveGenConfig(index);
                    }
                    ImagesHit::RemoveGenConfig(index) => {
                        return AgentSettingsHit::RemoveGenConfig(index);
                    }
                    ImagesHit::TestGenConfig(index) => {
                        return AgentSettingsHit::TestGenConfig(index);
                    }
                    ImagesHit::ToggleGenProviderMenu(index) => {
                        return AgentSettingsHit::ToggleGenProviderMenu(index);
                    }
                    ImagesHit::SelectGenProvider { index, provider } => {
                        return AgentSettingsHit::SelectGenProvider { index, provider };
                    }
                    ImagesHit::FocusGenConfig { index, field } => {
                        return AgentSettingsHit::FocusGenConfig { index, field };
                    }
                    ImagesHit::None => {}
                }
            }
            AgentSettingsTab::Fonts => {
                let hit = agent_settings_fonts::hit_test(
                    panel,
                    content_rect(panel),
                    self.ui,
                    point,
                    self.settings.scroll_y.offset,
                );
                if hit != FontsHit::None {
                    return AgentSettingsHit::Fonts(hit);
                }
            }
            AgentSettingsTab::System => {
                match agent_settings_system::hit_test(content_rect(panel), scrolled) {
                    SystemHit::ToggleAutoUpdate => return AgentSettingsHit::ToggleAutoUpdate,
                    SystemHit::ToggleExperimental => return AgentSettingsHit::ToggleExperimental,
                    SystemHit::SelectPencilCursor(style) => {
                        return AgentSettingsHit::SelectPencilCursor(style)
                    }
                    SystemHit::None => {}
                }
            }
            AgentSettingsTab::Account => {
                match agent_settings_account::hit_test(content_rect(panel), self.ui, scrolled) {
                    AccountTabHit::SignIn => return AgentSettingsHit::OpenLoginModal,
                    AccountTabHit::SignOut => return AgentSettingsHit::SignOutAccount,
                    AccountTabHit::None => {}
                }
            }
        }
        AgentSettingsHit::Inside
    }

    pub fn nav_at(&self, panel: Rect, point: Point2D) -> Option<AgentSettingsTab> {
        for (i, tab) in self.mode.visible_tabs().iter().enumerate() {
            if (nav_item_rect(panel, i)).contains(point) {
                return Some(*tab);
            }
        }
        None
    }

    pub fn card_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !(panel).contains(point) {
            return None;
        }
        if !self.mode.shows_external_agents() {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        (0..AgentProvider::ALL.len())
            .find(|&i| (agent_card_rect_in(panel, i, &self.settings)).contains(scrolled))
    }

    pub fn builtin_card_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !(panel).contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        agent_settings_builtin::card_at(content_rect(panel), &self.settings, scrolled)
    }

    pub fn acp_card_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !(panel).contains(point) {
            return None;
        }
        if !self.mode.shows_external_agents() {
            return None;
        }
        let content = content_rect(panel);
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        let section_y = acp_section_y(content, &self.settings);
        agent_settings_acp::card_at(content, &self.settings, scrolled, section_y)
    }

    /// Visible quick-add preset row under `point`, for the hosts' hover
    /// ladder. Same geometry contract as [`Self::acp_card_at`].
    pub fn acp_preset_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !(panel).contains(point) {
            return None;
        }
        if !self.mode.shows_external_agents() {
            return None;
        }
        let content = content_rect(panel);
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        let section_y = acp_section_y(content, &self.settings);
        agent_settings_acp::preset_row_at(content, &self.settings, scrolled, section_y)
    }

    pub fn builtin_preset_hover_at(
        &self,
        panel: Rect,
        point: Point2D,
    ) -> Option<BuiltinAgentPresetKey> {
        if !(panel).contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        agent_settings_builtin::preset_hover_at(content_rect(panel), &self.settings, scrolled)
    }

    pub fn builtin_preset_scroll_max_at(&self, panel: Rect, point: Point2D) -> Option<f32> {
        if !(panel).contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        agent_settings_builtin::preset_scroll_max_at(content_rect(panel), &self.settings, scrolled)
    }

    pub fn image_search_test_button_hover_at(&self, panel: Rect, point: Point2D) -> bool {
        if !(panel).contains(point) {
            return false;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        agent_settings_images::search_test_button_hover_at(
            content_rect(panel),
            &self.settings,
            scrolled,
        )
    }

    pub fn image_gen_add_button_hover_at(&self, panel: Rect, point: Point2D) -> bool {
        if !(panel).contains(point) {
            return false;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        agent_settings_images::add_gen_button_hover_at(
            content_rect(panel),
            &self.settings,
            scrolled,
        )
    }

    pub fn image_gen_profile_test_button_hover_at(
        &self,
        panel: Rect,
        point: Point2D,
    ) -> Option<usize> {
        if !(panel).contains(point) {
            return None;
        }
        let scrolled = Point2D::new(point.x, point.y + self.settings.scroll_y.offset);
        agent_settings_images::profile_test_button_hover_at(
            content_rect(panel),
            &self.settings,
            scrolled,
        )
    }

    pub fn content_total_height(&self) -> f32 {
        match self.mode.active_tab(&self.settings) {
            AgentSettingsTab::Agents => agents_content_height(&self.settings, self.mode),
            AgentSettingsTab::Mcp => agent_settings_mcp::content_height(&self.settings),
            AgentSettingsTab::Images => agent_settings_images::content_height(&self.settings),
            AgentSettingsTab::Fonts => agent_settings_fonts::content_height(self.ui),
            AgentSettingsTab::System => agent_settings_system::content_height(),
            AgentSettingsTab::Account => agent_settings_account::content_height(),
        }
    }

    pub fn font_picker_layout(
        &self,
        panel: Rect,
    ) -> Option<crate::widgets::property_panel_typography::FontPickerLayout> {
        agent_settings_fonts::picker_layout(
            panel,
            content_rect(panel),
            self.ui,
            self.settings.scroll_y.offset,
        )
    }
}
