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
    acp_section_y, agent_card_rect_at, agent_card_rect_in, agents_body_top, close_rect,
    connect_btn_rect_at, content_paint_clip_rect, content_rect, disconnect_btn_rect_at,
    full_settings_tabs, hero_body_rect, nav_item_rect, tab_i18n_label,
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

/// Modal size ceiling. The dialog is a wide centred workspace rather
/// than a fixed small window: [`AgentSettingsPanel::rect`] shrinks it to
/// fit the viewport, so these are maxima, not fixed dimensions.
pub const PANEL_WIDTH: f32 = 1100.0;
pub const PANEL_HEIGHT: f32 = 850.0;
/// Floor for the shrink-to-fit clamp — below this the tab strip and the
/// provider rows stop being readable, so the modal overflows a tiny
/// viewport instead of collapsing.
const PANEL_MIN_WIDTH: f32 = 620.0;
const PANEL_MIN_HEIGHT: f32 = 420.0;
/// Breathing room kept between the modal and the viewport edges.
const VIEWPORT_MARGIN_X: f32 = 32.0;
const VIEWPORT_MARGIN_BOTTOM: f32 = 48.0;

pub(super) const PAD: f32 = 32.0;
pub(super) const CONTENT_BOTTOM_PAD: f32 = 24.0;
/// Horizontal tab strip: inset from the panel top, pill height, and the
/// per-pill width band. Pills share one width so the hit-test geometry
/// stays measurement-free (paint centres icon + label inside).
pub(super) const TAB_BAR_TOP: f32 = 18.0;
pub(super) const TAB_HEIGHT: f32 = 34.0;
pub(super) const TAB_GAP: f32 = 6.0;
pub(super) const TAB_MAX_WIDTH: f32 = 132.0;
pub(super) const TAB_MIN_WIDTH: f32 = 76.0;
/// Fixed header band: `TAB_BAR_TOP` + `TAB_HEIGHT` + the gap down to the
/// scrollable body.
pub(super) const HEADER_HEIGHT: f32 = TAB_BAR_TOP + TAB_HEIGHT + 32.0;
/// Total vertical inset between the panel rect and its scrollable
/// content viewport. Hosts subtract this from the panel height to derive
/// the scroll viewport (`content_rect` height) instead of hardcoding it.
pub const CONTENT_VERTICAL_INSET: f32 = HEADER_HEIGHT + CONTENT_BOTTOM_PAD;
pub(super) const SECTION_GAP: f32 = 28.0;
/// Provider rows carry a name over a status subtitle, so they take the
/// modal's two-line row box — the same one the MCP server row and the
/// System auto-update row use. Hairline-separated list rows sit flush
/// against each other: the separator IS the gap.
pub(super) const CARD_HEIGHT: f32 = crate::widgets::agent_settings_rows::row_height(
    crate::widgets::agent_settings_rows::RowLines::Two,
);
pub(super) const CARD_GAP: f32 = 0.0;
pub(super) const CONNECT_BTN_W: f32 = 84.0;
pub(super) const CONNECT_BTN_H: f32 = 30.0;
pub(super) const AVATAR_SIZE: f32 = 34.0;
pub(super) const AVATAR_ICON: f32 = 20.0;
pub(super) const NAME_FONT: f32 = 14.0;
pub(super) const SUB_FONT: f32 = 12.0;
/// Width reserved at the right edge of a provider row for the status
/// pill. The painted pill hugs its label and is right-aligned inside the
/// slot; the slot itself is fixed so hit-test geometry needs no text
/// measurement.
pub(super) const STATUS_PILL_SLOT: f32 = 152.0;
pub(super) const STATUS_PILL_HEIGHT: f32 = 26.0;
pub(super) const STATUS_PILL_FONT: f32 = 12.0;
/// Agents-tab hero block (title + provider roll + blurb) above the first
/// section — the vertical offset from the content viewport's top to the
/// first section header. Public so host tests can anchor to it; it must
/// stay equal to `agent_settings_rows::tab_hero_height(2)`, which a unit
/// test asserts.
pub const AGENTS_HERO_HEIGHT: f32 = 96.0;

/// Scrollable body viewport inside `panel` — everything below the top
/// tab strip, inset by the modal's content padding. Exported so hosts and
/// their tests read the body geometry from its one definition instead of
/// re-deriving the insets.
pub fn content_viewport(panel: Rect) -> Rect {
    content_rect(panel)
}

/// Body viewport for the tabs the panel paints a hero on behalf of
/// (Images / Fonts / Account): [`content_viewport`] pushed down past that
/// hero. Those tabs' own geometry is written against the top of their
/// body, so this is the rect they — and anything testing them — must use.
pub fn secondary_tab_body(panel: Rect) -> Rect {
    hero_body_rect(content_rect(panel))
}

/// Agents tab: the external-provider row boxes, paired with their line
/// count — walked by the traversal layout audit.
#[cfg(test)]
pub(super) fn provider_row_boxes(
    panel: Rect,
) -> Vec<(Rect, crate::widgets::agent_settings_rows::RowLines)> {
    let settings = AgentSettings::default();
    (0..AgentProvider::ALL.len())
        .map(|i| {
            (
                agent_card_rect_in(panel, i, &settings),
                crate::widgets::agent_settings_rows::RowLines::Two,
            )
        })
        .collect()
}

/// MCP tab: the server Start/Stop button.
pub fn mcp_server_button(panel: Rect) -> Rect {
    agent_settings_mcp::server_button_rect(content_rect(panel))
}

/// MCP tab: the "Copy MCP config" action in the custom-configuration
/// section header.
pub fn mcp_copy_config_button(panel: Rect) -> Rect {
    agent_settings_mcp::client_config_copy_button_rect(content_rect(panel))
}

/// System tab: the switch on the Auto-update row.
pub fn system_auto_update_switch(panel: Rect) -> Rect {
    agent_settings_system::auto_update_switch_rect(content_rect(panel))
}

/// Close button rect in the header band, exported alongside
/// [`content_viewport`] so hosts and their tests target it by geometry
/// rather than by a copied offset.
pub fn close_button_rect(panel: Rect) -> Rect {
    close_rect(panel)
}

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
    SelectThemeMode(op_editor_core::editor_ui_state::ThemeMode),
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

    /// Centred modal rect, clamped to the viewport. Width and height both
    /// shrink below their ceiling when the window is small, so the dialog
    /// never runs off screen or under the top bar.
    pub fn rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        let top_limit = crate::widgets::TOP_BAR_HEIGHT + 8.0;
        let w = PANEL_WIDTH
            .min(viewport_w - VIEWPORT_MARGIN_X * 2.0)
            .max(PANEL_MIN_WIDTH);
        let h = PANEL_HEIGHT
            .min(viewport_h - crate::widgets::TOP_BAR_HEIGHT - VIEWPORT_MARGIN_BOTTOM)
            .max(PANEL_MIN_HEIGHT);
        let x = ((viewport_w - w) / 2.0).max(8.0);
        let y = ((viewport_h - h) / 2.0).max(top_limit);
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(w, h),
        }
    }
}

mod hero;
mod hit_test;
mod paint;
mod tabs;

pub use paint::drag_for_hit;
