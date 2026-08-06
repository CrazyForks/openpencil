//! Paint pass for [`AgentSettingsPanel`] — the modal frame, close
//! button, Agents tab body, and the shared section-header primitives,
//! plus the `Widget` impl and the drag hook. The top tab strip lives in
//! the `tabs` sibling and the Agents headline in `hero`. Carved off
//! `agent_settings_panel.rs` to keep every file under the 800-line cap.

use super::*;
use crate::widgets::text_metrics;

pub(super) fn agents_content_height(settings: &AgentSettings, mode: AgentSettingsPanelMode) -> f32 {
    let builtin = AGENTS_HERO_HEIGHT + agent_settings_builtin::content_height(settings);
    if !mode.shows_external_agents() {
        return builtin + 24.0;
    }
    builtin
        + SECTION_GAP
        + (agent_settings_acp::content_height(settings) + SECTION_GAP)
        + 32.0
        + AgentProvider::ALL.len() as f32 * (CARD_HEIGHT + CARD_GAP)
        + 28.0
        + 24.0
}

impl<'a> Widget for AgentSettingsPanel<'a> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &crate::widgets::LayoutCx) -> crate::widgets::LayoutBox {
        crate::widgets::LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(PANEL_WIDTH, PANEL_HEIGHT),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        paint_panel(
            cx,
            &self.theme,
            &self.settings,
            rect,
            self.ui,
            self.now_ms,
            self.mode,
        );
        if self.mode.active_tab(&self.settings) == AgentSettingsTab::Fonts {
            agent_settings_fonts::paint_picker(
                cx,
                &self.theme,
                rect,
                hero_body_rect(content_rect(rect)),
                self.ui,
                self.settings.scroll_y.offset,
                self.now_ms,
            );
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Dialog);
        node.set_label("Settings");
        node
    }
}

fn paint_panel(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    panel: Rect,
    _ui: &EditorUiState,
    now_ms: u64,
    mode: AgentSettingsPanelMode,
) {
    cx.backend.fill_round_rect(panel, 18.0, theme.card);
    cx.backend.stroke_round_rect(panel, 18.0, theme.border, 1.0);
    super::tabs::paint_tab_strip(cx, theme, settings, _ui, panel, mode);
    let content_rect = content_rect(panel);
    cx.backend.save();
    cx.backend.clip_rect(content_paint_clip_rect(panel));
    cx.backend
        .translate(Point2D::new(0.0, -settings.scroll_y.offset));
    match mode.active_tab(settings) {
        AgentSettingsTab::Agents => {
            paint_agents_tab(cx, theme, settings, _ui, content_rect, now_ms, mode)
        }
        AgentSettingsTab::Mcp => {
            agent_settings_mcp::paint_mcp_tab(cx, theme, settings, _ui, content_rect, now_ms)
        }
        AgentSettingsTab::Images => {
            paint_secondary_hero(cx, theme, _ui, content_rect, "settings.images");
            agent_settings_images::paint_images_tab(
                cx,
                theme,
                settings,
                _ui,
                hero_body_rect(content_rect),
                now_ms,
            )
        }
        AgentSettingsTab::Fonts => {
            paint_secondary_hero(cx, theme, _ui, content_rect, "settings.fonts");
            agent_settings_fonts::paint_fonts_tab(cx, theme, _ui, hero_body_rect(content_rect))
        }
        AgentSettingsTab::System => {
            agent_settings_system::paint_system_tab(cx, theme, settings, _ui, content_rect)
        }
        AgentSettingsTab::Account => {
            paint_secondary_hero(cx, theme, _ui, content_rect, "settings.account");
            agent_settings_account::paint_account_tab(cx, theme, _ui, hero_body_rect(content_rect))
        }
    }
    cx.backend.restore();
    paint_close(cx, theme, settings, _ui, panel);
}

/// Compact heading for the tabs the panel paints one on behalf of.
/// `prefix` names
/// the i18n family (`settings.images` → `settings.images.heroTitle` +
/// `.heroSubtitle`), so the key pair can't drift from the tab it belongs
/// to.
fn paint_secondary_hero(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    content: Rect,
    prefix: &str,
) {
    let (title, subtitle) = match prefix {
        "settings.images" => ("settings.images.heroTitle", "settings.images.heroSubtitle"),
        "settings.fonts" => ("settings.fonts.heroTitle", "settings.fonts.heroSubtitle"),
        _ => (
            "settings.account.heroTitle",
            "settings.account.heroSubtitle",
        ),
    };
    crate::widgets::agent_settings_rows::paint_tab_hero(
        cx,
        theme,
        content,
        t_settings(ui, title),
        &[t_settings(ui, subtitle)],
    );
}

fn paint_close(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    panel: Rect,
) {
    let close = close_rect(panel);
    let pressed = ui.button_pressed(ButtonPressTarget::AgentSettings(AgentSettingsButton::Close));
    crate::widgets::button::paint_ghost_button_feedback(
        cx.backend,
        theme,
        close,
        settings.hover_agent_settings_close,
        pressed,
    );
    draw_icon(
        cx.backend,
        Icon::Close,
        close.origin,
        close.size.x,
        theme.foreground,
        2.0,
    );
}

fn paint_agents_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
    now_ms: u64,
    mode: AgentSettingsPanelMode,
) {
    super::hero::paint_agents_hero(cx, theme, ui, content);
    let mut y = agents_body_top(content);
    y = agent_settings_builtin::paint_builtin_section(cx, theme, settings, ui, content, y, now_ms);
    if !mode.shows_external_agents() {
        return;
    }
    y += SECTION_GAP;
    y = agent_settings_acp::paint_acp_section(cx, theme, settings, ui, content, y, now_ms);
    y += SECTION_GAP;

    y = paint_section_header(
        cx,
        theme,
        t_settings(ui, "settings.agents.title"),
        "",
        content.origin.x,
        y,
        content.size.x,
    );
    for (i, provider) in AgentProvider::ALL.iter().enumerate() {
        let card = agent_card_rect_at(content.origin.x, y, content.size.x);
        paint_agent_card(cx, theme, settings, ui, *provider, card, i);
        y += CARD_HEIGHT + CARD_GAP;
        if matches!(provider, AgentProvider::ClaudeCode)
            && settings.provider_verified_connected_at(i)
        {
            let hint = TextLayout::single_run(
                t_settings(ui, "settings.agents.claudeHint"),
                "system-ui",
                12.0,
                (theme.muted_foreground).to_jian(),
                Point2D::new(0.0, 0.0),
            );
            cx.backend
                .draw_text(&hint, Point2D::new(content.origin.x, y + 8.0));
            y += 28.0;
        }
    }
}

fn paint_section_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    title: &str,
    action: &str,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    paint_section_header_inset(cx, theme, title, action, x, y, w, 0.0)
}

#[allow(clippy::too_many_arguments)]
fn paint_section_header_inset(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    title: &str,
    action: &str,
    x: f32,
    y: f32,
    w: f32,
    right_inset: f32,
) -> f32 {
    let layout = TextLayout::single_run(
        title,
        "system-ui",
        15.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y + 18.0));
    if !action.is_empty() {
        let action_w = text_metrics::measure_chrome(cx.backend, action, 12.0);
        let act = TextLayout::single_run(
            action,
            "system-ui",
            12.0,
            (theme.primary).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&act, Point2D::new(x + w - right_inset - action_w, y + 18.0));
    }
    y + 28.0
}

pub fn drag_for_hit(
    _hit: AgentSettingsHit,
) -> Option<op_editor_core::agent_settings::AgentSettingsDrag> {
    None
}
