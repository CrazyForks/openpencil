//! Built-in provider section for the Agent settings panel.

use crate::theme::Theme;
use crate::widgets::agent_settings_builtin_layout::{
    add_provider_rect, card_height, card_rect, compact_edit_rect, compact_remove_rect,
    compact_switch_rect, draft_card_height, expanded_card_height, field_input_rect, is_editing,
    sync_error_height, CARD_GAP, EMPTY_HEIGHT, HEADER_HEIGHT, SUBTITLE_HEIGHT,
};
use crate::widgets::agent_settings_builtin_parts;
use crate::widgets::agent_settings_caret::settings_input_text;
use crate::widgets::agent_settings_form_actions::{
    cancel_button_rect, paint_form_actions, save_button_rect,
};
use crate::widgets::agent_settings_header_action::{
    header_action_rect, header_action_text_baseline_y, header_action_text_x,
};
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_switch::paint_settings_switch;
use crate::widgets::brand_icons::{paint_brand_logo, BrandLogo};
use crate::widgets::button::paint_ghost_button_feedback;
use crate::widgets::button::tokens_from_theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::settings_form::{self, draw_text, ellipsize, paint_action};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use jian_widgets::components::card::Card;
use op_editor_core::agent_settings::{
    AgentSettings, BuiltinAgentConfig, BuiltinAgentField, BuiltinAgentKind,
    BuiltinAgentPresetMenuTarget, SettingsFocus,
};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::{AgentSettingsButton, BuiltinAgentPresetKey, ButtonPressTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinHit {
    AddProvider,
    Focus {
        index: usize,
        field: BuiltinAgentField,
    },
    FocusDraft(BuiltinAgentField),
    ToggleKind(usize),
    ToggleDraftKind,
    TogglePresetMenu(Option<usize>),
    SelectPreset {
        index: Option<usize>,
        preset: BuiltinAgentPresetKey,
    },
    SaveDraft,
    CancelDraft,
    ToggleEnabled(usize),
    Edit(usize),
    Remove(usize),
    None,
}

pub fn content_height(settings: &AgentSettings) -> f32 {
    let has_draft = settings.builtin_agent_draft.is_some();
    let list_h = if settings.builtin_agents.is_empty() && !has_draft {
        EMPTY_HEIGHT
    } else {
        let saved_h: f32 = settings
            .builtin_agents
            .iter()
            .enumerate()
            .map(|(index, _)| card_height(settings, index) + CARD_GAP)
            .sum();
        saved_h
            + if has_draft {
                draft_card_height(settings) + CARD_GAP
            } else {
                0.0
            }
    };
    HEADER_HEIGHT + SUBTITLE_HEIGHT + sync_error_height(settings) + list_h
}

pub fn hit_test(content: Rect, settings: &AgentSettings, point: Point2D) -> BuiltinHit {
    let y = content.origin.y + 12.0;
    if (add_provider_rect(content, y)).contains(point) {
        return BuiltinHit::AddProvider;
    }
    let mut card_y = y + HEADER_HEIGHT + SUBTITLE_HEIGHT + sync_error_height(settings);
    for (index, agent) in settings.builtin_agents.iter().enumerate() {
        let card = card_rect(
            content.origin.x,
            card_y,
            content.size.x,
            card_height(settings, index),
        );
        if is_editing(settings, index) {
            if (agent_settings_builtin_parts::provider_select_rect(card)).contains(point) {
                return BuiltinHit::TogglePresetMenu(Some(index));
            }
            if settings.builtin_preset_menu_open == Some(BuiltinAgentPresetMenuTarget::Agent(index))
            {
                if let Some(preset) = agent_settings_builtin_parts::preset_at(
                    card,
                    point,
                    settings.builtin_preset_menu_scroll.offset,
                ) {
                    return BuiltinHit::SelectPreset {
                        index: Some(index),
                        preset,
                    };
                }
            }
            if agent_settings_builtin_parts::kind_toggle_target(agent, card, point).is_some() {
                return BuiltinHit::ToggleKind(index);
            }
            for (row, field) in [
                BuiltinAgentField::DisplayName,
                BuiltinAgentField::ApiKey,
                BuiltinAgentField::Model,
                BuiltinAgentField::BaseUrl,
            ]
            .into_iter()
            .enumerate()
            {
                if field == BuiltinAgentField::BaseUrl && !agent.base_url_editable() {
                    continue;
                }
                if (field_input_rect(settings, card, Some(index), row)).contains(point) {
                    return BuiltinHit::Focus { index, field };
                }
            }
        } else if (compact_switch_rect(card)).contains(point) {
            return BuiltinHit::ToggleEnabled(index);
        } else if settings.hover_builtin_agent == index && (compact_edit_rect(card)).contains(point)
        {
            return BuiltinHit::Edit(index);
        } else if settings.hover_builtin_agent == index
            && (compact_remove_rect(card)).contains(point)
        {
            return BuiltinHit::Remove(index);
        }
        card_y += card.size.y + CARD_GAP;
    }
    if let Some(agent) = settings.builtin_agent_draft.as_ref() {
        let card = card_rect(
            content.origin.x,
            card_y,
            content.size.x,
            draft_card_height(settings),
        );
        if (agent_settings_builtin_parts::provider_select_rect(card)).contains(point) {
            return BuiltinHit::TogglePresetMenu(None);
        }
        if settings.builtin_preset_menu_open == Some(BuiltinAgentPresetMenuTarget::Draft) {
            if let Some(preset) = agent_settings_builtin_parts::preset_at(
                card,
                point,
                settings.builtin_preset_menu_scroll.offset,
            ) {
                return BuiltinHit::SelectPreset {
                    index: None,
                    preset,
                };
            }
        }
        if agent_settings_builtin_parts::kind_toggle_target(agent, card, point).is_some() {
            return BuiltinHit::ToggleDraftKind;
        }
        for (row, field) in [
            BuiltinAgentField::DisplayName,
            BuiltinAgentField::ApiKey,
            BuiltinAgentField::Model,
            BuiltinAgentField::BaseUrl,
        ]
        .into_iter()
        .enumerate()
        {
            if field == BuiltinAgentField::BaseUrl && !agent.base_url_editable() {
                continue;
            }
            if (field_input_rect(settings, card, None, row)).contains(point) {
                return BuiltinHit::FocusDraft(field);
            }
        }
        let form_h = expanded_card_height(settings, None);
        if (save_button_rect(card, form_h)).contains(point) {
            return BuiltinHit::SaveDraft;
        }
        if (cancel_button_rect(card, form_h)).contains(point) {
            return BuiltinHit::CancelDraft;
        }
    }
    BuiltinHit::None
}

pub fn card_at(content: Rect, settings: &AgentSettings, point: Point2D) -> Option<usize> {
    settings_form::card_index_at(
        content.origin.x,
        content.size.x,
        content.origin.y + 12.0 + HEADER_HEIGHT + SUBTITLE_HEIGHT + sync_error_height(settings),
        CARD_GAP,
        (0..settings.builtin_agents.len()).map(|index| card_height(settings, index)),
        point,
    )
}

pub fn preset_hover_at(
    content: Rect,
    settings: &AgentSettings,
    point: Point2D,
) -> Option<BuiltinAgentPresetKey> {
    let card = open_preset_menu_card(content, settings, point)?;
    agent_settings_builtin_parts::preset_hover_at(
        card,
        point,
        settings.builtin_preset_menu_scroll.offset,
    )
}

pub fn preset_scroll_max_at(
    content: Rect,
    settings: &AgentSettings,
    point: Point2D,
) -> Option<f32> {
    let _ = open_preset_menu_card(content, settings, point)?;
    Some(agent_settings_builtin_parts::preset_scroll_max())
}

fn open_preset_menu_card(content: Rect, settings: &AgentSettings, point: Point2D) -> Option<Rect> {
    let mut card_y =
        content.origin.y + 12.0 + HEADER_HEIGHT + SUBTITLE_HEIGHT + sync_error_height(settings);
    for (index, _) in settings.builtin_agents.iter().enumerate() {
        let card = card_rect(
            content.origin.x,
            card_y,
            content.size.x,
            card_height(settings, index),
        );
        if settings.builtin_preset_menu_open == Some(BuiltinAgentPresetMenuTarget::Agent(index))
            && agent_settings_builtin_parts::preset_menu_contains(card, point)
        {
            return Some(card);
        }
        card_y += card.size.y + CARD_GAP;
    }
    if settings.builtin_preset_menu_open == Some(BuiltinAgentPresetMenuTarget::Draft)
        && settings.builtin_agent_draft.is_some()
    {
        let card = card_rect(
            content.origin.x,
            card_y,
            content.size.x,
            draft_card_height(settings),
        );
        if agent_settings_builtin_parts::preset_menu_contains(card, point) {
            return Some(card);
        }
    }
    None
}

pub fn paint_builtin_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
    y: f32,
    now_ms: u64,
) -> f32 {
    let mut y = paint_header(
        cx,
        theme,
        t_settings(ui, "settings.agents.builtin"),
        t_settings(ui, "settings.agents.addProvider"),
        HeaderFrame {
            x: content.origin.x,
            y,
            w: content.size.x,
        },
        settings.hover_add_provider,
        ui.button_pressed(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::AddProvider,
        )),
    );
    y = settings_form::paint_subtitle(
        cx,
        theme,
        t_settings(ui, "settings.agents.builtinSubtitle"),
        content.origin.x,
        y,
    );
    if let Some(error) = settings.web_credential_sync_error.as_deref() {
        let text = format!("{} {error}", t_settings(ui, "settings.agents.syncError"));
        let layout = TextLayout::single_run(
            &text,
            "system-ui",
            12.0,
            (theme.destructive).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&layout, Point2D::new(content.origin.x, y + 14.0));
        y += sync_error_height(settings);
    }
    if settings.builtin_agents.is_empty() && settings.builtin_agent_draft.is_none() {
        return settings_form::paint_empty(
            cx,
            theme,
            t_settings(ui, "settings.agents.builtinEmpty"),
            content.origin.x,
            y,
            content.size.x,
        );
    }
    for (index, agent) in settings.builtin_agents.iter().enumerate() {
        let card = card_rect(
            content.origin.x,
            y,
            content.size.x,
            card_height(settings, index),
        );
        paint_builtin_agent_card(cx, theme, settings, ui, agent, index, card, now_ms);
        y += card.size.y + CARD_GAP;
    }
    if let Some(draft) = settings.builtin_agent_draft.as_ref() {
        let card = card_rect(
            content.origin.x,
            y,
            content.size.x,
            draft_card_height(settings),
        );
        paint_builtin_agent_form(cx, theme, settings, ui, draft, None, card, now_ms);
        let form_h = expanded_card_height(settings, None);
        paint_form_actions(
            cx,
            theme,
            ui,
            card,
            form_h,
            ui.builtin_agent_draft_ready(),
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::BuiltinCancelDraft,
            )),
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::BuiltinSaveDraft,
            )),
        );
        y += card.size.y + CARD_GAP;
    }
    y
}

#[derive(Debug, Clone, Copy)]
struct HeaderFrame {
    x: f32,
    y: f32,
    w: f32,
}

fn paint_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    title: &str,
    action: &str,
    frame: HeaderFrame,
    action_hover: bool,
    action_pressed: bool,
) -> f32 {
    let layout = TextLayout::single_run(
        title,
        "system-ui",
        15.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&layout, Point2D::new(frame.x, frame.y + 18.0));
    let action_w = cx.backend.measure_text(action, 12.0);
    let action_rect = header_action_rect(
        Rect {
            origin: Point2D::new(frame.x, frame.y),
            size: Point2D::new(frame.w, HEADER_HEIGHT),
        },
        frame.y,
        action_w,
    );
    paint_ghost_button_feedback(cx.backend, theme, action_rect, action_hover, action_pressed);
    let act = TextLayout::single_run(
        action,
        "system-ui",
        12.0,
        (theme.primary).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &act,
        Point2D::new(
            header_action_text_x(action_rect, action_w),
            header_action_text_baseline_y(action_rect),
        ),
    );
    frame.y + HEADER_HEIGHT
}

#[allow(clippy::too_many_arguments)]
fn paint_builtin_agent_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &BuiltinAgentConfig,
    index: usize,
    card: Rect,
    now_ms: u64,
) {
    if !is_editing(settings, index) {
        paint_compact_builtin_agent_card(cx, theme, settings, ui, agent, index, card);
        return;
    }
    paint_builtin_agent_form(cx, theme, settings, ui, agent, Some(index), card, now_ms);
}

#[allow(clippy::too_many_arguments)]
fn paint_builtin_agent_form(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &BuiltinAgentConfig,
    index: Option<usize>,
    card: Rect,
    now_ms: u64,
) {
    Card {
        fill: Some(theme.muted),
        border: Some(theme.border),
        radius: 10.0,
    }
    .paint(cx.backend, card, &tokens_from_theme(theme));
    let avatar = Rect {
        origin: Point2D::new(card.origin.x + 12.0, card.origin.y + 10.0),
        size: Point2D::new(28.0, 28.0),
    };
    cx.backend.fill_round_rect(avatar, 6.0, theme.background);
    let logo = match agent.kind {
        BuiltinAgentKind::Anthropic => BrandLogo::Claude,
        BuiltinAgentKind::OpenAiCompat => BrandLogo::OpenAI,
    };
    paint_brand_logo(
        cx.backend,
        logo,
        Point2D::new(avatar.origin.x + 6.0, avatar.origin.y + 6.0),
        16.0,
        theme.foreground,
    );
    let ready = agent.ready();
    let status = if ready { "ready" } else { "api key required" };
    let status_color = if ready {
        theme.status_success
    } else {
        theme.muted_foreground
    };
    draw_text(
        cx,
        status,
        11.0,
        status_color,
        card.origin.x + 48.0,
        card.origin.y + 28.0,
    );

    agent_settings_builtin_parts::paint_kind_toggle(cx, theme, agent, card);
    agent_settings_builtin_parts::paint_provider_select(cx, theme, agent, card);
    paint_field(
        cx,
        theme,
        settings,
        ui,
        agent,
        index,
        BuiltinAgentField::DisplayName,
        0,
        card,
        now_ms,
    );
    paint_field(
        cx,
        theme,
        settings,
        ui,
        agent,
        index,
        BuiltinAgentField::ApiKey,
        1,
        card,
        now_ms,
    );
    paint_field(
        cx,
        theme,
        settings,
        ui,
        agent,
        index,
        BuiltinAgentField::Model,
        2,
        card,
        now_ms,
    );
    paint_field(
        cx,
        theme,
        settings,
        ui,
        agent,
        index,
        BuiltinAgentField::BaseUrl,
        3,
        card,
        now_ms,
    );
    agent_settings_builtin_parts::paint_preset_menu(cx, theme, settings, agent, index, card);
}

fn paint_compact_builtin_agent_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &BuiltinAgentConfig,
    index: usize,
    card: Rect,
) {
    if agent.enabled {
        Card {
            fill: Some(theme.muted),
            border: Some(theme.border),
            radius: 8.0,
        }
        .paint(cx.backend, card, &tokens_from_theme(theme));
    }
    let avatar = Rect {
        origin: Point2D::new(card.origin.x + 12.0, card.origin.y + 12.0),
        size: Point2D::new(36.0, 36.0),
    };
    cx.backend.fill_round_rect(avatar, 8.0, theme.card);
    agent_settings_builtin_parts::paint_key_glyph(cx, theme, avatar);

    let text_x = card.origin.x + 60.0;
    let name = ellipsize(cx, &agent.display_name, 250.0, 13.0);
    draw_text(
        cx,
        &name,
        13.0,
        theme.foreground,
        text_x,
        card.origin.y + 22.0,
    );
    let api_key = if agent.api_key.trim().is_empty() {
        "api key required".to_string()
    } else {
        mask_key(&agent.api_key)
    };
    let detail = format!("{}  ·  {}", agent.model, api_key);
    let detail = ellipsize(cx, &detail, 300.0, 11.0);
    draw_text(
        cx,
        &detail,
        11.0,
        theme.muted_foreground,
        text_x,
        card.origin.y + 38.0,
    );
    if agent.ready() {
        let green = theme.status_success;
        draw_icon(
            cx.backend,
            Icon::Check,
            Point2D::new(text_x, card.origin.y + 44.0),
            10.0,
            green,
            2.2,
        );
        draw_text(
            cx,
            op_i18n::translate(ui.locale, "builtin.ready"),
            11.0,
            green,
            text_x + 14.0,
            card.origin.y + 53.0,
        );
    }

    paint_settings_switch(cx, theme, compact_switch_rect(card), agent.enabled);
    if settings.hover_builtin_agent == index {
        paint_action(
            cx,
            theme,
            compact_edit_rect(card),
            Icon::Pencil,
            theme.muted_foreground,
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::BuiltinEdit(index),
            )),
        );
        paint_action(
            cx,
            theme,
            compact_remove_rect(card),
            Icon::Trash,
            theme.muted_foreground,
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::BuiltinRemove(index),
            )),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_field(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &BuiltinAgentConfig,
    index: Option<usize>,
    field: BuiltinAgentField,
    row: usize,
    card: Rect,
    now_ms: u64,
) {
    let focus = match index {
        Some(index) => SettingsFocus::BuiltinAgent { index, field },
        None => SettingsFocus::BuiltinAgentDraft(field),
    };
    let focused = settings.focus == Some(focus);
    let fallback = match field {
        BuiltinAgentField::DisplayName => agent.display_name.as_str(),
        BuiltinAgentField::ApiKey if !agent.api_key.is_empty() => "********",
        BuiltinAgentField::ApiKey => "",
        BuiltinAgentField::Model => agent.model.as_str(),
        BuiltinAgentField::BaseUrl => agent.base_url.as_str(),
    };
    let value = settings_input_text(settings, ui, focus, fallback);
    let label = match field {
        BuiltinAgentField::DisplayName => "Name",
        BuiltinAgentField::ApiKey => "API Key",
        BuiltinAgentField::Model => "Model",
        BuiltinAgentField::BaseUrl => "Base URL",
    };
    let input = field_input_rect(settings, card, index, row);
    let editable = field != BuiltinAgentField::BaseUrl || agent.base_url_editable();
    let text_x = input.origin.x + 6.0;
    let painted_focused = settings_form::paint_field_frame(
        cx,
        theme,
        ui,
        label,
        card.origin.x + 12.0,
        input.origin.y + 16.0,
        input,
        focused,
        "",
        now_ms,
    );
    if !painted_focused {
        let clipped = ellipsize(cx, value, input.size.x - 12.0, 11.0);
        draw_text(
            cx,
            &clipped,
            11.0,
            if editable {
                theme.foreground
            } else {
                theme.muted_foreground
            },
            text_x,
            input.origin.y + 16.0,
        );
    }
}

// `draw_text` / `ellipsize` / `paint_action` live in the shared
// `settings_form` module.

fn mask_key(api_key: &str) -> String {
    if api_key.len() > 12 {
        format!("{}***{}", &api_key[..7], &api_key[api_key.len() - 3..])
    } else {
        "***".to_string()
    }
}
