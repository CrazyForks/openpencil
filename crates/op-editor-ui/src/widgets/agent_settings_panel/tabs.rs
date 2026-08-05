//! The centred horizontal tab strip across the top of the settings
//! modal — the wide-workspace replacement for the old 200 px sidebar
//! nav. Each tab is an equal-width pill carrying a lucide glyph and its
//! localized label; the active pill is filled, hover washes the rest.

use super::*;
use jian_widgets::centered_text_baseline_y;

const TAB_LABEL_FONT: f32 = 13.0;
const TAB_ICON_SIZE: f32 = 16.0;
const TAB_ICON_GAP: f32 = 6.0;

fn tab_icon(tab: AgentSettingsTab) -> Icon {
    match tab {
        AgentSettingsTab::Agents => Icon::Pen,
        AgentSettingsTab::Mcp => Icon::Terminal,
        AgentSettingsTab::Images => Icon::Image,
        AgentSettingsTab::Fonts => Icon::Type,
        AgentSettingsTab::System => Icon::Settings,
        AgentSettingsTab::Account => Icon::User,
    }
}

pub(super) fn paint_tab_strip(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    panel: Rect,
    mode: AgentSettingsPanelMode,
) {
    let tabs = mode.visible_tabs();
    let active_tab = mode.active_tab(settings);
    for (i, tab) in tabs.iter().enumerate() {
        let pill = nav_item_rect(panel, i, tabs.len());
        let selected = *tab == active_tab;
        let hovered = !selected && settings.hover_nav == Some(*tab);
        if selected {
            cx.backend
                .fill_round_rect(pill, pill.size.y / 2.0, theme.muted);
        } else if hovered {
            cx.backend
                .fill_round_rect(pill, pill.size.y / 2.0, theme.accent);
        }
        let color = if selected {
            theme.foreground
        } else {
            theme.muted_foreground
        };
        // Icon + label are measured as one group and centred together, so
        // a short label ("MCP") and a long one stay optically balanced in
        // pills of identical width.
        let label_budget = pill.size.x - 20.0 - TAB_ICON_SIZE - TAB_ICON_GAP;
        let label =
            crate::util::ellipsize_to_width(tab_i18n_label(ui, *tab), label_budget.max(0.0), |s| {
                cx.backend.measure_text(s, TAB_LABEL_FONT)
            });
        let label_w = cx.backend.measure_text(&label, TAB_LABEL_FONT);
        let group_w = TAB_ICON_SIZE + TAB_ICON_GAP + label_w;
        let icon_x = pill.origin.x + (pill.size.x - group_w) / 2.0;
        draw_icon(
            cx.backend,
            tab_icon(*tab),
            Point2D::new(icon_x, pill.origin.y + (pill.size.y - TAB_ICON_SIZE) / 2.0),
            TAB_ICON_SIZE,
            color,
            1.6,
        );
        let text = TextLayout::single_run(
            &label,
            "system-ui",
            TAB_LABEL_FONT,
            (color).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &text,
            Point2D::new(
                icon_x + TAB_ICON_SIZE + TAB_ICON_GAP,
                centered_text_baseline_y(pill, TAB_LABEL_FONT),
            ),
        );
    }
}
