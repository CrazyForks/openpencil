use crate::theme::Theme;
use crate::widgets::agent_settings_account::{self, AccountTabHit};
use crate::widgets::agent_settings_acp::{self, AcpHit};
use crate::widgets::agent_settings_builtin::{self, BuiltinHit};
use crate::widgets::agent_settings_fonts::{self, FontsHit};
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_images::{self, ImagesHit};
use crate::widgets::agent_settings_mcp::{self, McpHit};
use crate::widgets::agent_settings_panel_card::paint_agent_card;
use crate::widgets::agent_settings_panel_geometry::{
    acp_section_y, agent_card_rect_at, agent_card_rect_in, close_rect, connect_btn_rect_at,
    content_paint_clip_rect, content_rect, disconnect_btn_rect_at, full_settings_tabs,
    nav_item_rect, tab_i18n_label,
};
use crate::widgets::agent_settings_system::{self, SystemHit};
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{
    AcpAgentField, AgentProvider, AgentSettings, AgentSettingsTab, BuiltinAgentField,
    ImageGenField, ImageSearchField, McpCli,
};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::BuiltinAgentPresetKey;
use op_editor_core::EditorState;
use op_editor_core::{AgentSettingsButton, ButtonPressTarget};

pub const PANEL_WIDTH: f32 = 720.0;
pub const PANEL_HEIGHT: f32 = 720.0;
pub(super) const SIDEBAR_WIDTH: f32 = 200.0;
pub(super) const PAD: f32 = 24.0;
/// Total vertical inset (top + bottom `PAD`) between the panel rect and
/// its scrollable content viewport. Hosts subtract this from the panel
/// height to derive the scroll viewport (`content_rect` height) instead
/// of hardcoding the value.
pub const CONTENT_VERTICAL_INSET: f32 = PAD * 2.0;
pub(super) const NAV_ITEM_STEP: f32 = 30.0;
pub(super) const NAV_ITEM_HEIGHT: f32 = 28.0;
pub(super) const NAV_TOP: f32 = 56.0;
pub(super) const SECTION_GAP: f32 = 28.0;
pub(super) const CARD_HEIGHT: f32 = 56.0;
pub(super) const CARD_GAP: f32 = 8.0;
pub(super) const CONNECT_BTN_W: f32 = 76.0;
pub(super) const CONNECT_BTN_H: f32 = 30.0;
pub(super) const AVATAR_SIZE: f32 = 28.0;
pub(super) const AVATAR_ICON: f32 = 16.0;
pub(super) const NAME_FONT: f32 = 13.0;
pub(super) const SUB_FONT: f32 = 11.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsPanelMode {
    Full,
    /// `Full` plus the Account tab — selected when the host enabled the
    /// runtime account gate (`EditorUiState::account_ui_available`).
    FullWithAccount,
    WebBuiltinOnly,
    /// `WebBuiltinOnly` plus the Account tab (web with a daemon-side
    /// auth backend).
    WebBuiltinOnlyWithAccount,
    McpOnly,
}

impl AgentSettingsPanelMode {
    fn visible_tabs(self) -> &'static [AgentSettingsTab] {
        match self {
            AgentSettingsPanelMode::Full => full_settings_tabs(false),
            AgentSettingsPanelMode::FullWithAccount => full_settings_tabs(true),
            AgentSettingsPanelMode::WebBuiltinOnly => &[
                AgentSettingsTab::Agents,
                AgentSettingsTab::Images,
                AgentSettingsTab::Fonts,
                AgentSettingsTab::System,
            ],
            AgentSettingsPanelMode::WebBuiltinOnlyWithAccount => &[
                AgentSettingsTab::Agents,
                AgentSettingsTab::Images,
                AgentSettingsTab::Fonts,
                AgentSettingsTab::System,
                AgentSettingsTab::Account,
            ],
            AgentSettingsPanelMode::McpOnly => &[AgentSettingsTab::Mcp],
        }
    }

    fn active_tab(self, settings: &AgentSettings) -> AgentSettingsTab {
        if self.visible_tabs().contains(&settings.tab) {
            settings.tab
        } else {
            self.visible_tabs()[0]
        }
    }

    fn shows_external_agents(self) -> bool {
        matches!(
            self,
            AgentSettingsPanelMode::Full | AgentSettingsPanelMode::FullWithAccount
        )
    }
}

fn mode_for_ui(ui: &EditorUiState, base: AgentSettingsPanelMode) -> AgentSettingsPanelMode {
    if ui.embed == op_editor_core::EmbedHost::VsCode {
        AgentSettingsPanelMode::McpOnly
    } else if base == AgentSettingsPanelMode::Full && ui.account_ui_available {
        AgentSettingsPanelMode::FullWithAccount
    } else if base == AgentSettingsPanelMode::WebBuiltinOnly && ui.account_ui_available {
        AgentSettingsPanelMode::WebBuiltinOnlyWithAccount
    } else {
        base
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsHit {
    Close,
    SelectTab(AgentSettingsTab),
    Connect(AgentProvider),
    AddProvider,
    FocusBuiltinAgent {
        index: usize,
        field: BuiltinAgentField,
    },
    FocusBuiltinAgentDraft(BuiltinAgentField),
    ToggleBuiltinAgentKind(usize),
    ToggleBuiltinAgentDraftKind,
    ToggleBuiltinAgentPresetMenu(Option<usize>),
    SelectBuiltinAgentPreset {
        index: Option<usize>,
        preset: BuiltinAgentPresetKey,
    },
    SaveBuiltinAgentDraft,
    CancelBuiltinAgentDraft,
    ToggleBuiltinAgentEnabled(usize),
    EditBuiltinAgent(usize),
    RemoveBuiltinAgent(usize),
    AddAcpAgent,
    /// Quick-add row pressed — positional index into
    /// `AgentSettings::visible_acp_presets` for the same frame.
    AddAcpPreset(usize),
    FocusAcpAgent {
        index: usize,
        field: AcpAgentField,
    },
    FocusAcpAgentDraft(AcpAgentField),
    SaveAcpAgentDraft,
    CancelAcpAgentDraft,
    EditAcpAgent(usize),
    RemoveAcpAgent(usize),
    ToggleAcpConnected(usize),
    ToggleMcpServer,
    ToggleMcpCli(McpCli),
    CopyMcpClientConfig,
    ToggleImagesAdvanced,
    FocusSearchField(ImageSearchField),
    OpenImageRegisterLink,
    TestImageSearch,
    AddGenConfig,
    ToggleGenConfigEditor(usize),
    SetActiveGenConfig(usize),
    RemoveGenConfig(usize),
    TestGenConfig(usize),
    ToggleGenProviderMenu(usize),
    SelectGenProvider {
        index: usize,
        provider: op_editor_core::agent_settings::ImageGenProvider,
    },
    FocusGenConfig {
        index: usize,
        field: ImageGenField,
    },
    Fonts(FontsHit),
    ToggleAutoUpdate,
    ToggleExperimental,
    SelectPencilCursor(op_editor_core::PencilCursorStyle),
    FocusMcpPort,
    OpenLoginModal,
    SignOutAccount,
    Outside,
    Inside,
}

pub struct AgentSettingsPanel<'a> {
    pub id: WidgetId,
    pub theme: Theme,
    pub settings: AgentSettings,
    pub now_ms: u64,
    mode: AgentSettingsPanelMode,
    ui: &'a EditorUiState,
}

impl<'a> AgentSettingsPanel<'a> {
    pub fn for_editor(state: &'a EditorState) -> Self {
        Self::for_editor_at(state, 0)
    }

    pub fn for_editor_at(state: &'a EditorState, now_ms: u64) -> Self {
        Self {
            id: WidgetId::new(5200),
            theme: theme_for(&state.editor_ui),
            settings: state.editor_ui.agent_settings.clone(),
            now_ms,
            mode: mode_for_ui(&state.editor_ui, AgentSettingsPanelMode::Full),
            ui: &state.editor_ui,
        }
    }

    pub fn for_web_editor(state: &'a EditorState) -> Self {
        Self::for_web_editor_at(state, 0)
    }

    pub fn for_web_editor_at(state: &'a EditorState, now_ms: u64) -> Self {
        Self {
            id: WidgetId::new(5200),
            theme: theme_for(&state.editor_ui),
            settings: state.editor_ui.agent_settings.clone(),
            now_ms,
            mode: mode_for_ui(&state.editor_ui, AgentSettingsPanelMode::WebBuiltinOnly),
            ui: &state.editor_ui,
        }
    }

    pub fn rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        let x = ((viewport_w - PANEL_WIDTH) / 2.0).max(8.0);
        let y = ((viewport_h - PANEL_HEIGHT) / 2.0).max(crate::widgets::TOP_BAR_HEIGHT + 8.0);
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(PANEL_WIDTH, PANEL_HEIGHT),
        }
    }
}

mod hit_test;
mod paint;

pub use paint::drag_for_hit;
