//! Platform-neutral Prompt Center geometry, filtering, and hit testing.
//!
//! Hosts provide the panel rect and route the returned hit through their
//! shared press flow. The widget itself only reads [`EditorState`], so it
//! remains usable by both native and wasm hosts.

use std::borrow::Cow;

use op_editor_core::prompt_center_catalog::{prompt_catalogue, PromptCategory};
use op_editor_core::{ButtonPressTarget, EditorState, Locale, PromptFilter};

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::{Point2D, Rect};

/// Prompt Center width in logical pixels.
pub const PROMPT_CENTER_PANEL_W: f32 = 720.0;
/// Prompt Center height in logical pixels.
pub const PROMPT_CENTER_PANEL_H: f32 = 520.0;

/// Reserved hover token for the close button.
pub const PROMPT_CENTER_CLOSE_HOVER: usize = usize::MAX;
/// Reserved hover token for the "save current input" header action.
pub const PROMPT_CENTER_OPEN_SAVE_HOVER: usize = usize::MAX - 1;
/// Reserved hover token for the inline save button.
pub const PROMPT_CENTER_SAVE_HOVER: usize = usize::MAX - 2;
/// Reserved hover token for the inline cancel button.
pub const PROMPT_CENTER_CANCEL_HOVER: usize = usize::MAX - 3;

const FILTER_HOVER_BASE: usize = usize::MAX - 32;
const SAVE_CATEGORY_HOVER_BASE: usize = usize::MAX - 64;
const DELETE_HOVER_BASE: usize = usize::MAX / 2;

const PAD: f32 = 16.0;
const HEADER_H: f32 = 46.0;
const SEARCH_ROW_H: f32 = 42.0;
const FILTER_ROW_H: f32 = 40.0;
const SAVE_FORM_H: f32 = 76.0;
const CLOSE_BTN: f32 = 26.0;
const HEADER_ACTION_H: f32 = 26.0;
const SEARCH_H: f32 = 30.0;
const CHIP_H: f32 = 24.0;
const CHIP_GAP: f32 = 6.0;
const CARD_COLS: usize = 2;
const CARD_GAP: f32 = 12.0;
const CARD_H: f32 = 262.0;
const CARD_PREVIEW_INSET: f32 = 8.0;
const CARD_PREVIEW_ASPECT: f32 = 16.0 / 10.0;
const CARD_FOOTER_MIN_H: f32 = 44.0;
const DELETE_BTN: f32 = 24.0;

const FILTERS: [PromptFilter; 8] = [
    PromptFilter::All,
    PromptFilter::Category(PromptCategory::Starter),
    PromptFilter::Category(PromptCategory::MobileApp),
    PromptFilter::Category(PromptCategory::WebPage),
    PromptFilter::Category(PromptCategory::Dashboard),
    PromptFilter::Category(PromptCategory::Component),
    PromptFilter::Category(PromptCategory::Modify),
    PromptFilter::Custom,
];

/// One filtered card ready for geometry, paint, and an unambiguous click.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptCenterCard<'a> {
    pub id: Cow<'a, str>,
    pub title: Cow<'a, str>,
    pub body: &'a str,
    pub category: PromptCategory,
    pub screens: Option<u8>,
    pub freeform: bool,
    pub custom: bool,
}

/// What a pointer press inside the Prompt Center resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptCenterHit {
    Close,
    OpenSave,
    FocusSearch(usize),
    SelectFilter(PromptFilter),
    FocusSaveTitle(usize),
    SelectSaveCategory(PromptCategory),
    SaveCurrent,
    CancelSave,
    SelectPrompt { id: String, body: String },
    DeleteCustom(String),
    Inside,
}

/// Floating Prompt Center view model.
pub struct PromptCenterPanel<'a> {
    pub(super) state: &'a EditorState,
    pub(super) theme: Theme,
    pub(super) locale: Locale,
    pub(super) now_ms: u64,
}

impl<'a> PromptCenterPanel<'a> {
    /// Build the panel when it is open.
    pub fn for_editor(state: &'a EditorState) -> Option<Self> {
        Self::for_editor_at(state, 0)
    }

    /// Build the panel with a frame clock for caret blinking.
    pub fn for_editor_at(state: &'a EditorState, now_ms: u64) -> Option<Self> {
        state.editor_ui.prompt_center.open.then(|| Self {
            state,
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
            now_ms,
        })
    }

    /// Built-in or custom entries after category and query filtering.
    pub fn filtered(&self) -> Vec<PromptCenterCard<'a>> {
        let prompt_state = &self.state.editor_ui.prompt_center;
        let query = prompt_state.search.text().trim();
        match prompt_state.filter {
            PromptFilter::Custom => prompt_state
                .custom_prompts
                .iter()
                .filter(|prompt| custom_matches(prompt.title.as_str(), &prompt.body, query))
                .map(|prompt| PromptCenterCard {
                    id: Cow::Borrowed(prompt.id.as_str()),
                    title: Cow::Borrowed(prompt.title.as_str()),
                    body: prompt.body.as_str(),
                    category: prompt.category,
                    screens: None,
                    freeform: false,
                    custom: true,
                })
                .collect(),
            PromptFilter::All | PromptFilter::Category(_) => prompt_catalogue()
                .iter()
                .filter(|prompt| match prompt_state.filter {
                    PromptFilter::All => true,
                    PromptFilter::Category(category) => prompt.category == category,
                    PromptFilter::Custom => false,
                })
                .filter(|prompt| prompt.matches_query(self.locale, query))
                .map(|prompt| PromptCenterCard {
                    id: Cow::Borrowed(prompt.id.as_str()),
                    title: Cow::Borrowed(prompt.title_for_locale(self.locale)),
                    body: prompt.body_for_locale(self.locale),
                    category: prompt.category,
                    screens: prompt.screens,
                    freeform: prompt.tags.iter().any(|tag| tag == "freeform"),
                    custom: false,
                })
                .collect(),
        }
    }

    /// Resolve a pointer to a hover token shared with paint.
    pub fn hover_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !panel.contains(point) {
            return None;
        }
        if Self::close_rect(panel).contains(point) {
            return Some(PROMPT_CENTER_CLOSE_HOVER);
        }
        if self
            .save_current_rect(panel)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(PROMPT_CENTER_OPEN_SAVE_HOVER);
        }
        for (index, (rect, _)) in self.filter_chip_rects(panel).into_iter().enumerate() {
            if rect.contains(point) {
                return Some(filter_hover_token(index));
            }
        }
        if self.state.editor_ui.prompt_center.save_open {
            for (index, (rect, _)) in self.save_category_rects(panel).into_iter().enumerate() {
                if rect.contains(point) {
                    return Some(save_category_hover_token(index));
                }
            }
            if Self::save_button_rect(panel).contains(point) {
                return Some(PROMPT_CENTER_SAVE_HOVER);
            }
            if Self::cancel_button_rect(panel).contains(point) {
                return Some(PROMPT_CENTER_CANCEL_HOVER);
            }
        }
        let cards = self.filtered();
        let viewport = self.cards_viewport(panel);
        for (index, rect) in self.card_rects_for_count(panel, cards.len()) {
            if !viewport.contains(point) {
                continue;
            }
            if cards[index].custom
                && self.state.editor_ui.prompt_center.custom_store_writable
                && Self::delete_rect(rect).contains(point)
            {
                return Some(delete_hover_token(index));
            }
            if rect.contains(point) {
                return Some(index);
            }
        }
        None
    }

    /// Hit-test panel chrome and cards. Outside presses return `None`.
    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<PromptCenterHit> {
        if !panel.contains(point) {
            return None;
        }
        if Self::close_rect(panel).contains(point) {
            return Some(PromptCenterHit::Close);
        }
        if self
            .save_current_rect(panel)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(PromptCenterHit::OpenSave);
        }
        if Self::search_rect(panel).contains(point) {
            let input = &self.state.editor_ui.prompt_center.search;
            let offset = crate::widgets::text_input::single_line_byte_offset_at(
                input,
                Self::search_rect(panel),
                point,
                12.0,
                32.0,
            )
            .unwrap_or(input.text().len());
            return Some(PromptCenterHit::FocusSearch(offset));
        }
        for (rect, filter) in self.filter_chip_rects(panel) {
            if rect.contains(point) {
                return Some(PromptCenterHit::SelectFilter(filter));
            }
        }
        if self.state.editor_ui.prompt_center.save_open {
            if Self::save_title_rect(panel).contains(point) {
                let input = &self.state.editor_ui.prompt_center.save_title;
                let offset = crate::widgets::text_input::single_line_byte_offset_at(
                    input,
                    Self::save_title_rect(panel),
                    point,
                    12.0,
                    10.0,
                )
                .unwrap_or(input.text().len());
                return Some(PromptCenterHit::FocusSaveTitle(offset));
            }
            for (rect, category) in self.save_category_rects(panel) {
                if rect.contains(point) {
                    return Some(PromptCenterHit::SelectSaveCategory(category));
                }
            }
            if Self::save_button_rect(panel).contains(point) {
                return Some(if self.can_commit_save() {
                    PromptCenterHit::SaveCurrent
                } else {
                    PromptCenterHit::Inside
                });
            }
            if Self::cancel_button_rect(panel).contains(point) {
                return Some(PromptCenterHit::CancelSave);
            }
        }
        let cards = self.filtered();
        for (index, rect) in self.card_rects_for_count(panel, cards.len()) {
            if !self.cards_viewport(panel).contains(point) {
                continue;
            }
            let card = &cards[index];
            if card.custom
                && self.state.editor_ui.prompt_center.custom_store_writable
                && Self::delete_rect(rect).contains(point)
            {
                return Some(PromptCenterHit::DeleteCustom(card.id.to_string()));
            }
            if rect.contains(point) {
                return Some(PromptCenterHit::SelectPrompt {
                    id: card.id.to_string(),
                    body: card.body.to_owned(),
                });
            }
        }
        Some(PromptCenterHit::Inside)
    }

    /// Card rectangles shifted by the current scroll offset.
    pub fn card_rects(&self, panel: Rect) -> Vec<(usize, Rect)> {
        self.card_rects_for_count(panel, self.filtered().len())
    }

    /// Maximum vertical grid scroll for the current filter and query.
    pub fn max_scroll(&self, panel: Rect) -> f32 {
        let count = self.filtered().len();
        let rows = count.div_ceil(CARD_COLS);
        let content_h = if rows == 0 {
            0.0
        } else {
            rows as f32 * CARD_H + (rows - 1) as f32 * CARD_GAP
        };
        (content_h - self.cards_viewport(panel).size.y).max(0.0)
    }

    pub fn close_rect(panel: Rect) -> Rect {
        Rect::xywh(
            panel.origin.x + panel.size.x - PAD - CLOSE_BTN,
            panel.origin.y + (HEADER_H - CLOSE_BTN) / 2.0,
            CLOSE_BTN,
            CLOSE_BTN,
        )
    }

    pub fn search_rect(panel: Rect) -> Rect {
        Rect::xywh(
            panel.origin.x + PAD,
            panel.origin.y + HEADER_H + (SEARCH_ROW_H - SEARCH_H) / 2.0,
            panel.size.x - PAD * 2.0,
            SEARCH_H,
        )
    }

    pub fn save_title_rect(panel: Rect) -> Rect {
        Rect::xywh(
            panel.origin.x + PAD,
            Self::save_form_top(panel) + 8.0,
            260.0,
            28.0,
        )
    }

    pub fn save_button_rect(panel: Rect) -> Rect {
        Rect::xywh(
            panel.origin.x + panel.size.x - PAD - 58.0,
            Self::save_form_top(panel) + 8.0,
            58.0,
            28.0,
        )
    }

    pub fn cancel_button_rect(panel: Rect) -> Rect {
        let save = Self::save_button_rect(panel);
        Rect::xywh(save.origin.x - 68.0, save.origin.y, 62.0, save.size.y)
    }

    pub fn delete_rect(card: Rect) -> Rect {
        Rect::xywh(
            card.origin.x + card.size.x - DELETE_BTN - 8.0,
            card.origin.y + 8.0,
            DELETE_BTN,
            DELETE_BTN,
        )
    }

    /// Preview region within a card, preserving a 16:10 thumbnail aspect ratio.
    pub fn card_preview_rect(card: Rect) -> Rect {
        let width = (card.size.x - CARD_PREVIEW_INSET * 2.0).max(0.0);
        let available_height =
            (card.size.y - CARD_PREVIEW_INSET * 2.0 - CARD_FOOTER_MIN_H).max(0.0);
        Rect::xywh(
            card.origin.x + CARD_PREVIEW_INSET,
            card.origin.y + CARD_PREVIEW_INSET,
            width,
            (width / CARD_PREVIEW_ASPECT).min(available_height),
        )
    }

    /// Whether `point` is over one of the panel's visible text inputs.
    pub fn text_input_at(&self, panel: Rect, point: Point2D) -> bool {
        Self::search_rect(panel).contains(point)
            || (self.state.editor_ui.prompt_center.save_open
                && Self::save_title_rect(panel).contains(point))
    }

    /// Caret rectangle for IME candidate-window anchoring.
    pub fn focused_input_caret_rect(&self, panel: Rect) -> Rect {
        let prompt_center = &self.state.editor_ui.prompt_center;
        match prompt_center.focus {
            op_editor_core::PromptCenterFocus::Search => {
                crate::widgets::text_input::single_line_caret_rect(
                    &prompt_center.search,
                    Self::search_rect(panel),
                    12.0,
                    32.0,
                )
            }
            op_editor_core::PromptCenterFocus::SaveTitle => {
                crate::widgets::text_input::single_line_caret_rect(
                    &prompt_center.save_title,
                    Self::save_title_rect(panel),
                    12.0,
                    10.0,
                )
            }
        }
    }

    pub fn filter_chip_rects(&self, panel: Rect) -> Vec<(Rect, PromptFilter)> {
        let labels = FILTERS.map(|filter| self.filter_label(filter));
        chip_rects(
            panel.origin.x + PAD,
            panel.origin.y + HEADER_H + SEARCH_ROW_H + 8.0,
            panel.size.x - PAD * 2.0,
            &labels,
        )
        .into_iter()
        .zip(FILTERS)
        .collect()
    }

    pub fn save_category_rects(&self, panel: Rect) -> Vec<(Rect, PromptCategory)> {
        let labels = PromptCategory::ALL.map(|category| self.category_label(category));
        chip_rects(
            panel.origin.x + PAD,
            Self::save_form_top(panel) + 43.0,
            panel.size.x - PAD * 2.0,
            &labels,
        )
        .into_iter()
        .zip(PromptCategory::ALL)
        .collect()
    }

    pub fn cards_viewport(&self, panel: Rect) -> Rect {
        let top = self.cards_top(panel);
        Rect::xywh(
            panel.origin.x + PAD,
            top,
            panel.size.x - PAD * 2.0,
            (panel.origin.y + panel.size.y - PAD - top).max(0.0),
        )
    }

    pub(super) fn save_current_rect(&self, panel: Rect) -> Option<Rect> {
        self.can_open_save().then(|| {
            let close = Self::close_rect(panel);
            Rect::xywh(
                close.origin.x - 170.0,
                panel.origin.y + (HEADER_H - HEADER_ACTION_H) / 2.0,
                162.0,
                HEADER_ACTION_H,
            )
        })
    }

    pub(super) fn can_open_save(&self) -> bool {
        self.state.editor_ui.prompt_center.custom_store_writable
            && !self.state.chat.input.text().trim().is_empty()
    }

    pub(super) fn can_commit_save(&self) -> bool {
        self.can_open_save()
            && !self
                .state
                .editor_ui
                .prompt_center
                .save_title
                .text()
                .trim()
                .is_empty()
    }

    pub(super) fn is_pressed(&self, token: usize) -> bool {
        matches!(
            self.state.editor_ui.pressed_button,
            Some(ButtonPressTarget::PromptCenter(pressed)) if pressed == token
        )
    }

    pub(super) fn filter_label(&self, filter: PromptFilter) -> &'static str {
        match filter {
            PromptFilter::All => self.t("promptCenter.category.all"),
            PromptFilter::Category(category) => self.category_label(category),
            PromptFilter::Custom => self.t("promptCenter.category.custom"),
        }
    }

    pub(super) fn category_label(&self, category: PromptCategory) -> &'static str {
        self.t(match category {
            PromptCategory::Starter => "promptCenter.category.starter",
            PromptCategory::MobileApp => "promptCenter.category.mobileApp",
            PromptCategory::WebPage => "promptCenter.category.webPage",
            PromptCategory::Dashboard => "promptCenter.category.dashboard",
            PromptCategory::Component => "promptCenter.category.component",
            PromptCategory::Modify => "promptCenter.category.modify",
        })
    }

    pub(super) fn t(&self, key: &'static str) -> &'static str {
        op_i18n::translate(self.locale, key)
    }

    pub(super) fn metadata(&self, card: &PromptCenterCard<'_>) -> String {
        let mut parts = Vec::new();
        if let Some(count) = card.screens {
            let count = count.to_string();
            parts.push(op_i18n::translate_with(
                self.locale,
                "promptCenter.screens",
                &[("count", &count)],
            ));
        }
        if card.freeform {
            parts.push(self.t("promptCenter.freeform").to_owned());
        }
        parts.join(" · ")
    }

    fn save_form_top(panel: Rect) -> f32 {
        panel.origin.y + HEADER_H + SEARCH_ROW_H + FILTER_ROW_H
    }

    fn cards_top(&self, panel: Rect) -> f32 {
        Self::save_form_top(panel)
            + if self.state.editor_ui.prompt_center.save_open {
                SAVE_FORM_H
            } else {
                0.0
            }
            + 8.0
    }

    fn card_rects_for_count(&self, panel: Rect, count: usize) -> Vec<(usize, Rect)> {
        let viewport = self.cards_viewport(panel);
        let card_w = (viewport.size.x - CARD_GAP) / CARD_COLS as f32;
        let scroll = self
            .state
            .editor_ui
            .prompt_center
            .scroll
            .offset
            .clamp(0.0, self.max_scroll_for_count(panel, count));
        (0..count)
            .map(|index| {
                let row = index / CARD_COLS;
                let column = index % CARD_COLS;
                (
                    index,
                    Rect::xywh(
                        viewport.origin.x + column as f32 * (card_w + CARD_GAP),
                        viewport.origin.y + row as f32 * (CARD_H + CARD_GAP) - scroll,
                        card_w,
                        CARD_H,
                    ),
                )
            })
            .collect()
    }

    fn max_scroll_for_count(&self, panel: Rect, count: usize) -> f32 {
        let rows = count.div_ceil(CARD_COLS);
        let content_h = if rows == 0 {
            0.0
        } else {
            rows as f32 * CARD_H + (rows - 1) as f32 * CARD_GAP
        };
        (content_h - self.cards_viewport(panel).size.y).max(0.0)
    }
}

fn custom_matches(title: &str, body: &str, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty()
        || title.to_lowercase().contains(&query)
        || body.to_lowercase().contains(&query)
}

fn chip_rects(x: f32, y: f32, available_w: f32, labels: &[&str]) -> Vec<Rect> {
    let natural: Vec<f32> = labels
        .iter()
        .map(|label| (estimated_text_width(label, 11.0) + 20.0).max(48.0))
        .collect();
    let gaps = CHIP_GAP * labels.len().saturating_sub(1) as f32;
    let natural_total = natural.iter().sum::<f32>();
    let scale = if natural_total + gaps > available_w && natural_total > 0.0 {
        ((available_w - gaps) / natural_total).max(0.0)
    } else {
        1.0
    };
    let mut cursor = x;
    natural
        .into_iter()
        .map(|width| {
            let rect = Rect::xywh(cursor, y, width * scale, CHIP_H);
            cursor += rect.size.x + CHIP_GAP;
            rect
        })
        .collect()
}

pub(super) fn estimated_text_width(text: &str, size: f32) -> f32 {
    text.chars()
        .map(|ch| if ch.is_ascii() { size * 0.56 } else { size })
        .sum()
}

pub(super) fn filter_hover_token(index: usize) -> usize {
    FILTER_HOVER_BASE - index
}

pub(super) fn save_category_hover_token(index: usize) -> usize {
    SAVE_CATEGORY_HOVER_BASE - index
}

pub(super) fn delete_hover_token(index: usize) -> usize {
    DELETE_HOVER_BASE + index
}

#[path = "prompt_center_panel_paint.rs"]
mod paint;

#[cfg(test)]
#[path = "prompt_center_panel_tests.rs"]
mod tests;
