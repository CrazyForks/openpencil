//! Images tab of the settings modal.

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_images_parts::{
    ellipsize, paint_profile_field, paint_profile_test_button, paint_provider_field,
    paint_provider_menu, paint_search_input_row,
};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{
    AgentSettings, ImageGenField, ImageGenProfile, ImageGenProvider, ImageSearchField,
    ImageTestStatus, SettingsFocus,
};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::{AgentSettingsButton, ButtonPressTarget};

mod hit_test;
mod paint;

pub use hit_test::*;
pub(super) use paint::paint_images_tab;

const TITLE_H: f32 = 36.0;
const ADVANCED_ROW_H: f32 = 24.0;
const SECTION_GAP: f32 = 28.0;
const SECTION_TITLE_H: f32 = 36.0;
const SUBTITLE_H: f32 = 22.0;
const ROW_H: f32 = 36.0;
const ROW_VGAP: f32 = 10.0;
const LABEL_W: f32 = 110.0;
const TEST_BTN_W: f32 = 56.0;
const ADD_BTN_W: f32 = 72.0;
const BTN_H: f32 = 28.0;
const BODY_GAP: f32 = 14.0;
const REGISTER_ROW_H: f32 = 36.0;
// Fixed hit-rect width for the "Register at Openverse" link. Covers the
// link text + trailing chevron across all locales without reaching the
// right-aligned Test button.
const REGISTER_LINK_W: f32 = 220.0;
const PROFILE_ROW_H: f32 = 32.0;
const PROFILE_ROW_GAP: f32 = 6.0;
const PROFILE_ROW_INSET_X: f32 = 8.0;
const PROFILE_LIST_TOP_GAP: f32 = 8.0;
const ACTIVE_DOT: f32 = 14.0;
const DELETE_W: f32 = 32.0;
const DELETE_HOVER_INSET: f32 = 2.0;
const DELETE_ICON: f32 = 12.0;
const CHEVRON_W: f32 = 24.0;
const PROFILE_FORM_TOP: f32 = 40.0;
const PROFILE_FIELD_H: f32 = 24.0;
const PROFILE_TEST_BTN_W: f32 = 56.0;
const PROFILE_TEST_GAP: f32 = 8.0;
const PROVIDER_OPTION_H: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagesHit {
    ToggleAdvanced,
    FocusSearchField(ImageSearchField),
    OpenRegisterLink,
    TestSearch,
    AddGenConfig,
    ToggleGenConfigEditor(usize),
    SetActiveGenConfig(usize),
    RemoveGenConfig(usize),
    TestGenConfig(usize),
    ToggleGenProviderMenu(usize),
    SelectGenProvider {
        index: usize,
        provider: ImageGenProvider,
    },
    FocusGenConfig {
        index: usize,
        field: ImageGenField,
    },
    None,
}

fn advanced_body_h() -> f32 {
    SUBTITLE_H + ROW_H + ROW_VGAP + ROW_H + BODY_GAP + REGISTER_ROW_H
}

fn image_gen_section_top(content: Rect, settings: &AgentSettings) -> f32 {
    let mut y = content.origin.y + TITLE_H + ADVANCED_ROW_H;
    if settings.images_advanced_open {
        y += advanced_body_h();
    }
    y + SECTION_GAP
}

pub(super) fn content_height(settings: &AgentSettings) -> f32 {
    let mut h = TITLE_H + ADVANCED_ROW_H;
    if settings.images_advanced_open {
        h += advanced_body_h();
    }
    h + SECTION_GAP + SECTION_TITLE_H + PROFILE_LIST_TOP_GAP + profile_list_h(settings) + 24.0
}

fn profile_list_h(settings: &AgentSettings) -> f32 {
    if settings.image_gen_profiles.is_empty() {
        80.0
    } else {
        settings
            .image_gen_profiles
            .iter()
            .enumerate()
            .map(|(index, _)| profile_row_h(settings, index))
            .sum::<f32>()
            + settings.image_gen_profiles.len().saturating_sub(1) as f32 * PROFILE_ROW_GAP
    }
}

fn profile_row_h(settings: &AgentSettings, index: usize) -> f32 {
    if is_editing_profile(settings, index) {
        PROFILE_ROW_H + 8.0 + 5.0 * ROW_H
    } else {
        PROFILE_ROW_H
    }
}

#[rustfmt::skip]
fn advanced_toggle_rect(content: Rect) -> Rect {
    Rect::xywh(content.origin.x, content.origin.y + TITLE_H, 140.0, ADVANCED_ROW_H)
}

fn register_link_y(content: Rect) -> f32 {
    content.origin.y + TITLE_H + ADVANCED_ROW_H + SUBTITLE_H + ROW_H + ROW_VGAP + ROW_H + BODY_GAP
}

/// Click target for the "Register at Openverse" link (text + chevron).
/// A fixed `REGISTER_LINK_W` width covers every locale's link label
/// without reaching the right-aligned Test button.
pub(super) fn register_link_rect(content: Rect) -> Rect {
    Rect::xywh(
        content.origin.x,
        register_link_y(content),
        REGISTER_LINK_W,
        REGISTER_ROW_H,
    )
}

#[rustfmt::skip]
fn search_field_rect(content: Rect, index: usize) -> Rect {
    let y = content.origin.y
        + TITLE_H
        + ADVANCED_ROW_H
        + SUBTITLE_H
        + if index == 0 { 0.0 } else { ROW_H + ROW_VGAP };
    Rect::xywh(content.origin.x + LABEL_W, y, content.size.x - LABEL_W, ROW_H)
}

fn has_search_credentials(settings: &AgentSettings) -> bool {
    !settings.openverse_client_id.trim().is_empty()
        || !settings.openverse_client_secret.trim().is_empty()
}

fn search_test_enabled(settings: &AgentSettings) -> bool {
    has_search_credentials(settings)
        && settings.images_search_test_status != ImageTestStatus::Testing
}

fn profile_test_enabled(profile: &ImageGenProfile) -> bool {
    !profile.api_key.trim().is_empty() && profile.test_status != ImageTestStatus::Testing
}

#[rustfmt::skip]
fn test_btn_rect(content: Rect, settings: &AgentSettings) -> Rect {
    if !settings.images_advanced_open {
        return Rect::xywh(0.0, 0.0, 0.0, 0.0);
    }
    let y = register_link_y(content) + (REGISTER_ROW_H - BTN_H) / 2.0;
    Rect::xywh(content.origin.x + content.size.x - TEST_BTN_W, y, TEST_BTN_W, BTN_H)
}

#[rustfmt::skip]
fn add_btn_rect(content: Rect, settings: &AgentSettings) -> Rect {
    let top = image_gen_section_top(content, settings);
    Rect::xywh(content.origin.x + content.size.x - ADD_BTN_W, top + (SECTION_TITLE_H - BTN_H) / 2.0, ADD_BTN_W, BTN_H)
}

fn profile_row_rect(content: Rect, settings: &AgentSettings, index: usize) -> Rect {
    let top = image_gen_section_top(content, settings) + SECTION_TITLE_H + PROFILE_LIST_TOP_GAP;
    let y = settings
        .image_gen_profiles
        .iter()
        .enumerate()
        .take(index)
        .fold(top, |acc, (i, _)| {
            acc + profile_row_h(settings, i) + PROFILE_ROW_GAP
        });
    Rect::xywh(
        content.origin.x + PROFILE_ROW_INSET_X,
        y,
        (content.size.x - PROFILE_ROW_INSET_X * 2.0).max(0.0),
        profile_row_h(settings, index),
    )
}

#[rustfmt::skip]
fn profile_active_rect(row: Rect) -> Rect {
    Rect::xywh(row.origin.x + 8.0, row.origin.y + (PROFILE_ROW_H - ACTIVE_DOT) / 2.0, ACTIVE_DOT, ACTIVE_DOT)
}

#[rustfmt::skip]
fn profile_remove_rect(row: Rect) -> Rect {
    Rect::xywh(row.origin.x + row.size.x - DELETE_W, row.origin.y, DELETE_W, PROFILE_ROW_H)
}

#[rustfmt::skip]
fn profile_remove_hover_rect(row: Rect) -> Rect {
    let target = profile_remove_rect(row);
    Rect::xywh(target.origin.x + DELETE_HOVER_INSET, target.origin.y + DELETE_HOVER_INSET, target.size.x - DELETE_HOVER_INSET * 2.0, target.size.y - DELETE_HOVER_INSET * 2.0)
}

#[rustfmt::skip]
fn profile_chevron_rect(row: Rect) -> Rect {
    Rect::xywh(row.origin.x + row.size.x - DELETE_W - CHEVRON_W, row.origin.y + (PROFILE_ROW_H - CHEVRON_W) / 2.0, CHEVRON_W, CHEVRON_W)
}

#[rustfmt::skip]
fn profile_header_rect(row: Rect) -> Rect {
    Rect::xywh(row.origin.x, row.origin.y, row.size.x, PROFILE_ROW_H)
}

#[rustfmt::skip]
fn profile_field_rect(row: Rect, field_index: usize) -> Rect {
    Rect::xywh(row.origin.x + LABEL_W, row.origin.y + PROFILE_FORM_TOP + field_index as f32 * ROW_H, row.size.x - LABEL_W - 12.0, PROFILE_FIELD_H)
}

fn profile_input_rect(row: Rect, field: ImageGenField) -> Rect {
    let mut input = profile_field_rect(row, profile_field_index(field));
    if matches!(field, ImageGenField::ApiKey) {
        input.size.x = (input.size.x - PROFILE_TEST_GAP - PROFILE_TEST_BTN_W).max(48.0);
    }
    input
}

#[rustfmt::skip]
fn profile_test_btn_rect(row: Rect) -> Rect {
    let input = profile_field_rect(row, profile_field_index(ImageGenField::ApiKey));
    Rect::xywh(input.origin.x + input.size.x - PROFILE_TEST_BTN_W, input.origin.y, PROFILE_TEST_BTN_W, PROFILE_FIELD_H)
}

fn profile_provider_rect(row: Rect) -> Rect {
    profile_field_rect(row, 1)
}

#[rustfmt::skip]
fn profile_provider_option_rect(row: Rect, option_index: usize) -> Rect {
    let provider = profile_provider_rect(row);
    Rect::xywh(provider.origin.x, provider.origin.y + provider.size.y + option_index as f32 * PROVIDER_OPTION_H, provider.size.x, PROVIDER_OPTION_H)
}

fn profile_field_index(field: ImageGenField) -> usize {
    match field {
        ImageGenField::Name => 0,
        ImageGenField::ApiKey => 2,
        ImageGenField::Model => 3,
        ImageGenField::BaseUrl => 4,
    }
}

fn image_gen_fields() -> [ImageGenField; 4] {
    use ImageGenField::*;
    [Name, ApiKey, Model, BaseUrl]
}

fn is_editing_profile(settings: &AgentSettings, index: usize) -> bool {
    matches!(
        settings.focus,
        Some(SettingsFocus::ImageGenProfile { index: i, .. }) if i == index
    )
}
