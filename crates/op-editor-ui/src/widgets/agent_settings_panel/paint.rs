//! Paint pass for [`AgentSettingsPanel`] — the modal frame, sidebar nav,
//! close button, Agents tab body, and the shared section-header
//! primitives, plus the `Widget` impl and the drag hook. Carved off
//! `agent_settings_panel.rs` to keep every file under the 800-line cap.

use super::*;

pub(super) fn agents_content_height(settings: &AgentSettings, mode: AgentSettingsPanelMode) -> f32 {
    let builtin = 12.0 + agent_settings_builtin::content_height(settings);
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
                content_rect(rect),
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
    cx.backend.fill_round_rect(panel, 14.0, theme.card);
    cx.backend.stroke_round_rect(panel, 14.0, theme.border, 1.0);
    paint_sidebar(cx, theme, settings, _ui, panel, mode);
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
            agent_settings_images::paint_images_tab(cx, theme, settings, _ui, content_rect, now_ms)
        }
        AgentSettingsTab::Fonts => {
            agent_settings_fonts::paint_fonts_tab(cx, theme, _ui, content_rect)
        }
        AgentSettingsTab::System => {
            agent_settings_system::paint_system_tab(cx, theme, settings, _ui, content_rect)
        }
        AgentSettingsTab::Account => {
            agent_settings_account::paint_account_tab(cx, theme, _ui, content_rect)
        }
    }
    cx.backend.restore();
    paint_close(cx, theme, settings, _ui, panel);
}

fn paint_sidebar(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    panel: Rect,
    mode: AgentSettingsPanelMode,
) {
    let sidebar = Rect {
        origin: panel.origin,
        size: Point2D::new(SIDEBAR_WIDTH, panel.size.y),
    };
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(sidebar.origin.x + sidebar.size.x - 1.0, sidebar.origin.y),
            size: Point2D::new(1.0, sidebar.size.y),
        },
        theme.border,
    );
    let title = TextLayout::single_run(
        t_settings(ui, "settings.title"),
        "system-ui",
        15.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(panel.origin.x + 16.0, panel.origin.y + 31.0),
    );
    let active_tab = mode.active_tab(settings);
    for (i, tab) in mode.visible_tabs().iter().enumerate() {
        let r = nav_item_rect(panel, i);
        let selected = *tab == active_tab;
        let hovered = !selected && settings.hover_nav == Some(*tab);
        if selected {
            cx.backend.fill_round_rect(r, 8.0, theme.muted);
        } else if hovered {
            cx.backend.fill_round_rect(r, 8.0, theme.accent);
        }
        let icon = match tab {
            AgentSettingsTab::Agents => Icon::Pen,
            AgentSettingsTab::Mcp => Icon::Terminal,
            AgentSettingsTab::Images => Icon::Image,
            AgentSettingsTab::Fonts => Icon::Type,
            AgentSettingsTab::System => Icon::Settings,
            AgentSettingsTab::Account => Icon::User,
        };
        let icon_color = if selected {
            theme.foreground
        } else {
            theme.muted_foreground
        };
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(r.origin.x + 12.0, r.origin.y + 7.0),
            14.0,
            icon_color,
            1.6,
        );
        let label = TextLayout::single_run(
            tab_i18n_label(ui, *tab),
            "system-ui",
            13.0,
            (icon_color).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&label, Point2D::new(r.origin.x + 38.0, r.origin.y + 18.0));
    }
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
    let mut y = content.origin.y + 12.0;
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
        let action_w = cx.backend.measure_text(action, 12.0);
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
