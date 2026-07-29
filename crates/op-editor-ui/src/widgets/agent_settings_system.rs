//! System tab of the settings modal.
//!
//! Renders the auto-update preference and live release-probe status plus the
//! experimental-features opt-in and pencil-cursor picker.

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_switch::{
    paint_settings_switch, SETTINGS_SWITCH_H, SETTINGS_SWITCH_W,
};
use crate::widgets::button::tokens_from_theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use jian_widgets::components::card::Card;
use op_editor_core::agent_settings::AgentSettings;
use op_editor_core::editor_ui_state::{EditorUiState, UpdateStatus};

const TITLE_H: f32 = 36.0;
const TOGGLE_CARD_H: f32 = 58.0;
const UPDATE_CARD_H: f32 = 116.0;
const CARD_GAP: f32 = 12.0;
const STATUS_DOT_RADIUS: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemHit {
    ToggleAutoUpdate,
    ToggleExperimental,
    /// Click on one of the pencil-cursor swatches.
    SelectPencilCursor(op_editor_core::PencilCursorStyle),
    None,
}

/// Pencil-cursor picker card metrics.
const CURSOR_CARD_H: f32 = 108.0;
const CURSOR_SWATCH: f32 = 52.0;
const CURSOR_SWATCH_GAP: f32 = 10.0;

pub(super) fn content_height() -> f32 {
    let update = if AUTO_UPDATE_AVAILABLE {
        UPDATE_CARD_H + CARD_GAP
    } else {
        0.0
    };
    12.0 + TITLE_H + update + TOGGLE_CARD_H + CARD_GAP + CURSOR_CARD_H + 24.0
}

fn cursor_card_rect(content: Rect) -> Rect {
    let prev = experimental_card_rect(content);
    Rect {
        origin: Point2D::new(content.origin.x, prev.origin.y + TOGGLE_CARD_H + CARD_GAP),
        size: Point2D::new(content.size.x, CURSOR_CARD_H),
    }
}

/// Swatch rect for style index `i` inside the cursor card. Shared by
/// paint + hit-test.
fn cursor_swatch_rect(card: Rect, i: usize) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + 16.0 + i as f32 * (CURSOR_SWATCH + CURSOR_SWATCH_GAP),
            card.origin.y + 42.0,
        ),
        size: Point2D::new(CURSOR_SWATCH, CURSOR_SWATCH),
    }
}

fn auto_update_card_rect(content: Rect) -> Rect {
    Rect {
        origin: Point2D::new(content.origin.x, content.origin.y + 12.0 + TITLE_H),
        size: Point2D::new(content.size.x, UPDATE_CARD_H),
    }
}

/// Host capability: the auto-update toggle drives the desktop
/// updater (`op-host-desktop/src/update_check.rs`). The web host has
/// no updater — hide the card there instead of painting a switch that
/// silently does nothing (same pattern as `GIT_BUTTON_AVAILABLE`).
pub(super) const AUTO_UPDATE_AVAILABLE: bool = !cfg!(target_arch = "wasm32");

fn experimental_card_rect(content: Rect) -> Rect {
    if !AUTO_UPDATE_AVAILABLE {
        // The auto-update card is hidden — the experimental card
        // moves up into its slot.
        return auto_update_card_rect(content);
    }
    Rect {
        origin: Point2D::new(
            content.origin.x,
            content.origin.y + 12.0 + TITLE_H + UPDATE_CARD_H + CARD_GAP,
        ),
        size: Point2D::new(content.size.x, TOGGLE_CARD_H),
    }
}

/// The switch hit/paint rect for a given card — right-aligned, vertically
/// centred. Shared by paint + hit-test so the two can't drift.
fn switch_rect_for(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 16.0 - SETTINGS_SWITCH_W,
            card.origin.y + (TOGGLE_CARD_H - SETTINGS_SWITCH_H) / 2.0,
        ),
        size: Point2D::new(SETTINGS_SWITCH_W, SETTINGS_SWITCH_H),
    }
}

pub fn hit_test(content: Rect, scrolled: Point2D) -> SystemHit {
    if AUTO_UPDATE_AVAILABLE && switch_rect_for(auto_update_card_rect(content)).contains(scrolled) {
        return SystemHit::ToggleAutoUpdate;
    }
    if switch_rect_for(experimental_card_rect(content)).contains(scrolled) {
        return SystemHit::ToggleExperimental;
    }
    let cursor_card = cursor_card_rect(content);
    for (i, style) in op_editor_core::PencilCursorStyle::ALL.iter().enumerate() {
        if cursor_swatch_rect(cursor_card, i).contains(scrolled) {
            return SystemHit::SelectPencilCursor(*style);
        }
    }
    SystemHit::None
}

pub(super) fn paint_system_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
) {
    let title = TextLayout::single_run(
        t_settings(ui, "settings.system.title"),
        "system-ui",
        15.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(content.origin.x, content.origin.y + 20.0),
    );

    // Auto-update card — desktop-only (see `AUTO_UPDATE_AVAILABLE`).
    if AUTO_UPDATE_AVAILABLE {
        paint_toggle_card(
            cx,
            theme,
            ui,
            auto_update_card_rect(content),
            "agents.autoUpdate",
            "settings.autoUpdateDesc",
            settings.auto_update_enabled,
        );
        paint_update_status(cx, theme, ui, auto_update_card_rect(content));
    }
    paint_toggle_card(
        cx,
        theme,
        ui,
        experimental_card_rect(content),
        "settings.experimental",
        "settings.experimentalDesc",
        settings.experimental_features_enabled,
    );
    paint_cursor_card(cx, theme, ui, cursor_card_rect(content));
}

/// Lower half of the auto-update card: current probe state and build version.
fn paint_update_status(cx: &mut PaintCx<'_>, theme: &Theme, ui: &EditorUiState, card: Rect) {
    let (color, status_key, description_key) = status_view(theme, &ui.update_status);
    let divider_y = card.origin.y + TOGGLE_CARD_H;
    cx.backend.stroke_line(
        Point2D::new(card.origin.x + 16.0, divider_y),
        Point2D::new(card.origin.x + card.size.x - 16.0, divider_y),
        theme.border,
        1.0,
    );

    let dot_center_y = divider_y + 20.0;
    cx.backend.fill_oval(
        Rect {
            origin: Point2D::new(card.origin.x + 16.0, dot_center_y - STATUS_DOT_RADIUS),
            size: Point2D::new(STATUS_DOT_RADIUS * 2.0, STATUS_DOT_RADIUS * 2.0),
        },
        color,
    );

    let status_text = match &ui.update_status {
        UpdateStatus::Available { version } => {
            format!("{} v{version}", t_settings(ui, status_key))
        }
        _ => t_settings(ui, status_key).to_string(),
    };
    let status = TextLayout::single_run(
        &status_text,
        "system-ui",
        12.0,
        color.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &status,
        Point2D::new(card.origin.x + 32.0, divider_y + 24.0),
    );

    let current_version = format!(
        "{}: v{}",
        t_settings(ui, "settings.system.currentVersion"),
        env!("CARGO_PKG_VERSION")
    );
    let current_version_w = cx.backend.measure_text(&current_version, 10.0);
    let version = TextLayout::single_run(
        &current_version,
        "system-ui",
        10.0,
        theme.muted_foreground.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &version,
        Point2D::new(
            card.origin.x + card.size.x - 16.0 - current_version_w,
            divider_y + 24.0,
        ),
    );

    let description = TextLayout::single_run(
        t_settings(ui, description_key),
        "system-ui",
        11.0,
        theme.muted_foreground.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &description,
        Point2D::new(card.origin.x + 16.0, divider_y + 46.0),
    );
}

fn status_view(theme: &Theme, status: &UpdateStatus) -> (Color, &'static str, &'static str) {
    match status {
        UpdateStatus::Idle => (
            theme.muted_foreground,
            "settings.system.idle",
            "settings.system.idleDescription",
        ),
        UpdateStatus::Checking => (
            theme.primary,
            "settings.system.checking",
            "settings.system.checkingDescription",
        ),
        UpdateStatus::UpToDate => (
            theme.status_success,
            "settings.system.upToDate",
            "settings.system.upToDateDescription",
        ),
        UpdateStatus::Available { .. } => (
            theme.status_warning,
            "settings.system.updateAvailable",
            "settings.system.updateAvailableDescription",
        ),
        UpdateStatus::Error => (
            theme.destructive,
            "settings.system.errorStatus",
            "settings.system.errorDescription",
        ),
    }
}

/// The pencil-cursor picker: a labelled card with one swatch per style,
/// each drawing its real silhouette at half scale; the active style gets
/// an accent ring.
fn paint_cursor_card(cx: &mut PaintCx<'_>, theme: &Theme, ui: &EditorUiState, card: Rect) {
    Card {
        fill: Some(theme.muted),
        border: Some(theme.border),
        radius: 10.0,
    }
    .paint(cx.backend, card, &tokens_from_theme(theme));

    let label = TextLayout::single_run(
        "Pencil cursor",
        "system-ui",
        13.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label,
        Point2D::new(card.origin.x + 16.0, card.origin.y + 24.0),
    );

    for (i, style) in op_editor_core::PencilCursorStyle::ALL.iter().enumerate() {
        let swatch = cursor_swatch_rect(card, i);
        let active = ui.pencil_cursor_style == *style;
        cx.backend.fill_round_rect(
            swatch,
            8.0,
            if active {
                theme.row_selected_primary
            } else {
                theme.button_hover
            },
        );
        if active {
            cx.backend
                .stroke_round_rect(swatch, 8.0, theme.primary, 1.5);
        }
        crate::widgets::canvas_agent_cursor::paint_cursor_swatch(
            cx,
            *style,
            Point2D::new(swatch.origin.x + 15.0, swatch.origin.y + 13.0),
            theme.primary,
        );
    }
}

/// Paint one labelled toggle card: rounded background, title row, muted
/// description row, and a right-aligned switch reflecting `enabled`.
fn paint_toggle_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    card: Rect,
    label_key: &'static str,
    desc_key: &'static str,
    enabled: bool,
) {
    Card {
        fill: Some(theme.muted),
        border: Some(theme.border),
        radius: 10.0,
    }
    .paint(cx.backend, card, &tokens_from_theme(theme));

    let label_layout = TextLayout::single_run(
        t_settings(ui, label_key),
        "system-ui",
        13.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label_layout,
        Point2D::new(card.origin.x + 16.0, card.origin.y + 24.0),
    );

    let desc_layout = TextLayout::single_run(
        t_settings(ui, desc_key),
        "system-ui",
        11.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &desc_layout,
        Point2D::new(card.origin.x + 16.0, card.origin.y + 46.0),
    );

    paint_settings_switch(cx, theme, switch_rect_for(card), enabled);
}
