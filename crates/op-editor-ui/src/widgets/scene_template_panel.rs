//! Platform-neutral Asset Center geometry, filtering, and hit testing.
//!
//! Same contract as the Prompt Center: hosts supply the panel rect and route
//! the returned hit through their shared press flow, and the widget reads only
//! [`EditorState`] so both the native and wasm hosts can use it.
//!
//! The two panels look alike on purpose — a user who has met one should not
//! have to learn the other — but they answer different questions. A prompt
//! ends up in the chat input; a template opens as a document. That is why the
//! only card action here is "open", and why the panel carries no save form.
//!
//! The panel is tabbed: Templates is the original card grid, Styles lists the
//! style-guide catalogue. The tab is an enum threaded through every geometry
//! helper rather than a pair of hard-coded layouts, because the tab row is
//! meant to grow (Design Systems, Scripts) without the panel being rewritten
//! each time.

use op_editor_core::scene_template_catalog::{
    scene_template_catalogue, SceneTemplateDefinition, TemplateScene,
};
use op_editor_core::{
    AssetCenterTab, ButtonPressTarget, EditorState, Locale, SceneFilter, SceneTemplateFocus,
};

use super::asset_center_style_cards::{filtered_style_guide_cards, StyleGuideCard};
use super::prompt_center_panel::estimated_text_width;
use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::{Point2D, Rect};

/// Scene Template Center width in logical pixels.
pub const SCENE_TEMPLATE_PANEL_W: f32 = 720.0;
/// Scene Template Center height in logical pixels. The tab row is additive:
/// it must not eat a row out of the grid it sits above.
pub const SCENE_TEMPLATE_PANEL_H: f32 = 554.0;

/// Hover token for the close button.
pub const SCENE_TEMPLATE_CLOSE_HOVER: usize = usize::MAX;

/// Hover token for the generate button.
pub const SCENE_TEMPLATE_GENERATE_HOVER: usize = usize::MAX - 64;

const FILTER_HOVER_BASE: usize = usize::MAX - 32;
/// Tab chips reserve their own token band. It sits below the filter band
/// (which reaches `FILTER_HOVER_BASE + scene count`) so the two can never
/// collide as either row grows.
const TAB_HOVER_BASE: usize = usize::MAX - 96;

pub(super) const PAD: f32 = 16.0;
pub(super) const HEADER_H: f32 = 46.0;
pub(super) const TAB_ROW_H: f32 = 34.0;
pub(super) const SEARCH_ROW_H: f32 = 42.0;
pub(super) const FILTER_ROW_H: f32 = 40.0;
pub(super) const CLOSE_BTN: f32 = 26.0;
const SEARCH_H: f32 = 30.0;
pub(super) const SEARCH_TEXT_SIZE: f32 = 12.0;
/// Left inset of the search text, clearing the magnifier glyph. Shared by
/// paint and the caret hit-test so a click lands where the glyph is drawn.
pub(super) const SEARCH_PAD_X: f32 = 32.0;
pub(super) const CHIP_H: f32 = 24.0;
const CHIP_GAP: f32 = 6.0;
const CARD_COLS: usize = 2;
const CARD_GAP: f32 = 12.0;
pub(super) const GENERATE_ROW_H: f32 = 64.0;
pub(super) const GENERATE_INPUT_H: f32 = 32.0;
pub(super) const GENERATE_BUTTON_W: f32 = 92.0;
pub(super) const GENERATE_GAP: f32 = 8.0;
pub(super) const GENERATE_TEXT_SIZE: f32 = 12.0;
pub(super) const GENERATE_HINT_SIZE: f32 = 10.5;
/// Left inset of the topic text, clearing the sparkle glyph. Shared by paint
/// and the caret hit-test, for the same reason [`SEARCH_PAD_X`] is.
pub(super) const GENERATE_INPUT_PAD_X: f32 = 31.0;
pub(super) const CARD_H: f32 = 262.0;
pub(super) const CARD_PREVIEW_INSET: f32 = 8.0;
pub(super) const CARD_PREVIEW_ASPECT: f32 = 16.0 / 10.0;
/// Style cards carry no preview image yet (M2 bakes one), so they are a
/// name, a colour band, and a line of tags — a third the height of a
/// template card.
pub(super) const STYLE_CARD_H: f32 = 92.0;
pub(super) const STYLE_CARD_COLS: usize = 2;
pub(super) const STYLE_SWATCH_H: f32 = 16.0;

/// A hover token for the filter chip at `index`.
pub(super) fn filter_hover_token(index: usize) -> usize {
    FILTER_HOVER_BASE + index
}

/// A hover token for the tab chip at `index`.
pub(super) fn tab_hover_token(index: usize) -> usize {
    TAB_HOVER_BASE + index
}

/// What a press inside the panel resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneTemplateHit {
    Close,
    FocusSearch(usize),
    /// Put the caret in the generate row's topic field.
    FocusGenerate(usize),
    /// Submit the typed topic as a generation request.
    Generate,
    SelectFilter(SceneFilter),
    /// Switch which asset family the panel is showing.
    SelectTab(AssetCenterTab),
    /// Open this template as a new document.
    SelectTemplate(String),
    /// Pin this style guide, or unpin it when it is already the pinned one.
    ToggleStyleGuide(String),
    /// Inside the panel but not on a control — swallows the press so it
    /// cannot fall through to the canvas underneath.
    Inside,
}

/// Floating Scene Template Center view model.
pub struct SceneTemplatePanel<'a> {
    pub(super) state: &'a EditorState,
    pub(super) theme: Theme,
    pub(super) locale: Locale,
    pub(super) now_ms: u64,
}

impl<'a> SceneTemplatePanel<'a> {
    /// Build the panel when it is open.
    pub fn for_editor(state: &'a EditorState) -> Option<Self> {
        Self::for_editor_at(state, 0)
    }

    /// Build the panel with a frame clock for caret blinking.
    pub fn for_editor_at(state: &'a EditorState, now_ms: u64) -> Option<Self> {
        state.editor_ui.scene_template_center.open.then(|| Self {
            state,
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
            now_ms,
        })
    }

    pub(super) fn now_ms(&self) -> u64 {
        self.now_ms
    }

    pub(super) fn is_pressed(&self, token: usize) -> bool {
        matches!(
            self.state.editor_ui.pressed_button,
            Some(ButtonPressTarget::SceneTemplate(pressed)) if pressed == token
        )
    }

    /// Which asset family the panel is showing.
    pub fn tab(&self) -> AssetCenterTab {
        self.state.editor_ui.scene_template_center.tab
    }

    /// The pinned style guide's name, if any.
    pub fn pinned_style_guide(&self) -> Option<&str> {
        self.state.editor_ui.pinned_style_guide.as_deref()
    }

    /// Style guides surviving the search query, in registry order.
    pub fn style_cards(&self) -> Vec<StyleGuideCard> {
        filtered_style_guide_cards(self.state.editor_ui.scene_template_center.search.text())
    }

    /// Templates surviving the scene filter and the search query.
    pub fn filtered(&self) -> Vec<&'static SceneTemplateDefinition> {
        let center = &self.state.editor_ui.scene_template_center;
        let query = center.search.text().trim();
        scene_template_catalogue()
            .iter()
            .filter(|template| match center.filter {
                SceneFilter::All => true,
                SceneFilter::Scene(scene) => template.scene == scene,
            })
            .filter(|template| template.matches_query(self.locale, query))
            .collect()
    }

    /// The chip row: "All" plus every scene, in catalogue order.
    pub(super) fn filters(&self) -> Vec<SceneFilter> {
        let mut filters = vec![SceneFilter::All];
        filters.extend(TemplateScene::ALL.map(SceneFilter::Scene));
        filters
    }

    /// Label for one chip.
    pub(super) fn filter_label(&self, filter: SceneFilter) -> &'static str {
        match filter {
            SceneFilter::All => {
                let translated = op_i18n::translate(self.locale, "sceneTemplate.filter.all");
                if translated == "sceneTemplate.filter.all" {
                    "全部"
                } else {
                    translated
                }
            }
            SceneFilter::Scene(scene) => {
                let translated = op_i18n::translate(self.locale, scene.title_key());
                if translated == scene.title_key() {
                    scene.title_fallback()
                } else {
                    translated
                }
            }
        }
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
            panel.origin.y + HEADER_H + TAB_ROW_H + (SEARCH_ROW_H - SEARCH_H) / 2.0,
            panel.size.x - PAD * 2.0,
            SEARCH_H,
        )
    }

    /// Label for one tab chip.
    pub(super) fn tab_label(&self, tab: AssetCenterTab) -> &'static str {
        let translated = op_i18n::translate(self.locale, tab.title_key());
        if translated == tab.title_key() {
            tab.title_fallback()
        } else {
            translated
        }
    }

    pub(super) fn tab_chip_rects(&self, panel: Rect) -> Vec<(Rect, AssetCenterTab)> {
        let top = panel.origin.y + HEADER_H + (TAB_ROW_H - CHIP_H) / 2.0;
        let mut x = panel.origin.x + PAD;
        AssetCenterTab::ALL
            .into_iter()
            .map(|tab| {
                let width = chip_width(self.tab_label(tab));
                let rect = Rect::xywh(x, top, width, CHIP_H);
                x += width + CHIP_GAP;
                (rect, tab)
            })
            .collect()
    }

    /// The scene-filter row belongs to the template catalogue; the style
    /// catalogue has its own vocabulary (tags) and is searched, not filtered.
    /// Collapsing the row to zero height rather than hiding it in paint keeps
    /// every rect below it in one place.
    fn filter_row_height(&self) -> f32 {
        match self.tab() {
            AssetCenterTab::Templates => FILTER_ROW_H,
            AssetCenterTab::Styles => 0.0,
        }
    }

    pub(super) fn filter_chip_rects(&self, panel: Rect) -> Vec<(Rect, SceneFilter)> {
        if self.tab() != AssetCenterTab::Templates {
            return Vec::new();
        }
        let top =
            panel.origin.y + HEADER_H + TAB_ROW_H + SEARCH_ROW_H + (FILTER_ROW_H - CHIP_H) / 2.0;
        let mut x = panel.origin.x + PAD;
        self.filters()
            .into_iter()
            .map(|filter| {
                let width = chip_width(self.filter_label(filter));
                let rect = Rect::xywh(x, top, width, CHIP_H);
                x += width + CHIP_GAP;
                (rect, filter)
            })
            .collect()
    }

    /// Whether the prompt-to-deck row paints.
    ///
    /// Two gates, and they answer different questions. The filter gate is
    /// about relevance: the row generates a deck, so it belongs to the slides
    /// scene and to the unfiltered view that contains it — offering it under
    /// "Cards" would promise a deck where the user asked for a card. The
    /// capability gate is about honesty: a host that cannot both replace the
    /// document and launch a chat turn would paint a button whose press goes
    /// nowhere, so it gets no row at all rather than a dead one.
    pub fn generate_row_visible(&self) -> bool {
        if !self.state.editor_ui.scene_template_generate_supported {
            return false;
        }
        // The Styles tab has no scene filter to be relevant to, and the row
        // is the whole point of pinning: pick an aesthetic, type a topic,
        // get a document in that aesthetic without a second trip.
        if self.tab() == AssetCenterTab::Styles {
            return true;
        }
        matches!(
            self.state.editor_ui.scene_template_center.filter,
            SceneFilter::All | SceneFilter::Scene(TemplateScene::Slides)
        )
    }

    fn generate_row_height(&self) -> f32 {
        if self.generate_row_visible() {
            GENERATE_ROW_H
        } else {
            0.0
        }
    }

    pub(super) fn generate_row_top(&self, panel: Rect) -> f32 {
        panel.origin.y + HEADER_H + TAB_ROW_H + SEARCH_ROW_H + self.filter_row_height()
    }

    /// Topic field rect, or `None` when the row does not paint.
    pub fn generate_input_rect(&self, panel: Rect) -> Option<Rect> {
        if !self.generate_row_visible() {
            return None;
        }
        Some(Rect::xywh(
            panel.origin.x + PAD,
            self.generate_row_top(panel) + 4.0,
            (panel.size.x - PAD * 2.0 - GENERATE_BUTTON_W - GENERATE_GAP).max(0.0),
            GENERATE_INPUT_H,
        ))
    }

    /// Generate button rect, or `None` when the row does not paint.
    pub fn generate_button_rect(&self, panel: Rect) -> Option<Rect> {
        let input = self.generate_input_rect(panel)?;
        Some(Rect::xywh(
            input.origin.x + input.size.x + GENERATE_GAP,
            input.origin.y,
            GENERATE_BUTTON_W,
            GENERATE_INPUT_H,
        ))
    }

    pub(super) fn cards_top(&self, panel: Rect) -> f32 {
        self.generate_row_top(panel) + self.generate_row_height()
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

    /// Column count and row height of the grid the active tab paints. One
    /// walker serves both tabs; only these two numbers differ.
    fn grid_metrics(&self) -> (usize, f32) {
        match self.tab() {
            AssetCenterTab::Templates => (CARD_COLS, CARD_H),
            AssetCenterTab::Styles => (STYLE_CARD_COLS, STYLE_CARD_H),
        }
    }

    pub(super) fn content_height_for_count(&self, count: usize) -> f32 {
        let (columns, card_h) = self.grid_metrics();
        let rows = count.div_ceil(columns);
        if rows == 0 {
            0.0
        } else {
            rows as f32 * card_h + (rows - 1) as f32 * CARD_GAP
        }
    }

    /// How many cards the active tab is showing.
    fn visible_card_count(&self) -> usize {
        match self.tab() {
            AssetCenterTab::Templates => self.filtered().len(),
            AssetCenterTab::Styles => self.style_cards().len(),
        }
    }

    /// Largest legal scroll offset for the current result set.
    pub fn max_scroll(&self, panel: Rect) -> f32 {
        self.max_scroll_for_count(panel, self.visible_card_count())
    }

    pub(super) fn max_scroll_for_count(&self, panel: Rect, count: usize) -> f32 {
        let viewport = self.cards_viewport(panel);
        (self.content_height_for_count(count) - viewport.size.y).max(0.0)
    }

    pub(super) fn card_rects_for_count(&self, panel: Rect, count: usize) -> Vec<(usize, Rect)> {
        let viewport = self.cards_viewport(panel);
        let (columns, card_h) = self.grid_metrics();
        let card_w = (viewport.size.x - CARD_GAP * (columns - 1) as f32) / columns as f32;
        let scroll = self
            .state
            .editor_ui
            .scene_template_center
            .scroll
            .offset
            .clamp(0.0, self.max_scroll_for_count(panel, count));
        (0..count)
            .map(|index| {
                let row = index / columns;
                let column = index % columns;
                let rect = Rect::xywh(
                    viewport.origin.x + column as f32 * (card_w + CARD_GAP),
                    viewport.origin.y + row as f32 * (card_h + CARD_GAP) - scroll,
                    card_w,
                    card_h,
                );
                (index, rect)
            })
            .collect()
    }

    pub(super) fn card_rects(&self, panel: Rect) -> Vec<(usize, Rect)> {
        self.card_rects_for_count(panel, self.visible_card_count())
    }

    /// Resolve a pointer to a hover token shared with paint.
    pub fn hover_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !panel.contains(point) {
            return None;
        }
        if Self::close_rect(panel).contains(point) {
            return Some(SCENE_TEMPLATE_CLOSE_HOVER);
        }
        for (index, (rect, _)) in self.tab_chip_rects(panel).into_iter().enumerate() {
            if rect.contains(point) {
                return Some(tab_hover_token(index));
            }
        }
        for (index, (rect, _)) in self.filter_chip_rects(panel).into_iter().enumerate() {
            if rect.contains(point) {
                return Some(filter_hover_token(index));
            }
        }
        if self
            .generate_button_rect(panel)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(SCENE_TEMPLATE_GENERATE_HOVER);
        }
        // A card scrolled out of the viewport must not hover: its rect is
        // still computed (paint clips it), so the viewport check is what
        // keeps a pointer below the panel from lighting up a hidden row.
        let viewport = self.cards_viewport(panel);
        if !viewport.contains(point) {
            return None;
        }
        self.card_rects(panel)
            .into_iter()
            .find(|(_, rect)| rect.contains(point))
            .map(|(index, _)| index)
    }

    /// Hit-test panel chrome and cards. Outside presses return `None` so the
    /// caller can treat them as dismiss.
    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<SceneTemplateHit> {
        if !panel.contains(point) {
            return None;
        }
        if Self::close_rect(panel).contains(point) {
            return Some(SceneTemplateHit::Close);
        }
        for (rect, tab) in self.tab_chip_rects(panel) {
            if rect.contains(point) {
                return Some(SceneTemplateHit::SelectTab(tab));
            }
        }
        let search = Self::search_rect(panel);
        if search.contains(point) {
            let caret = self.caret_at(
                &self.state.editor_ui.scene_template_center.search,
                search,
                SEARCH_PAD_X,
                SEARCH_TEXT_SIZE,
                point,
            );
            return Some(SceneTemplateHit::FocusSearch(caret));
        }
        for (rect, filter) in self.filter_chip_rects(panel) {
            if rect.contains(point) {
                return Some(SceneTemplateHit::SelectFilter(filter));
            }
        }
        if let Some(input) = self.generate_input_rect(panel) {
            if input.contains(point) {
                let caret = self.caret_at(
                    &self.state.editor_ui.scene_template_center.generate,
                    input,
                    GENERATE_INPUT_PAD_X,
                    GENERATE_TEXT_SIZE,
                    point,
                );
                return Some(SceneTemplateHit::FocusGenerate(caret));
            }
        }
        if self
            .generate_button_rect(panel)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(SceneTemplateHit::Generate);
        }
        let viewport = self.cards_viewport(panel);
        if viewport.contains(point) {
            match self.tab() {
                AssetCenterTab::Templates => {
                    let cards = self.filtered();
                    for (index, rect) in self.card_rects_for_count(panel, cards.len()) {
                        if rect.contains(point) {
                            return Some(SceneTemplateHit::SelectTemplate(cards[index].id.clone()));
                        }
                    }
                }
                AssetCenterTab::Styles => {
                    let cards = self.style_cards();
                    for (index, rect) in self.card_rects_for_count(panel, cards.len()) {
                        if rect.contains(point) {
                            return Some(SceneTemplateHit::ToggleStyleGuide(
                                cards[index].name.to_string(),
                            ));
                        }
                    }
                }
            }
        }
        Some(SceneTemplateHit::Inside)
    }

    /// Caret index for a press inside a text field of this panel.
    fn caret_at(
        &self,
        input: &jian_core::text_input::TextInputState,
        rect: Rect,
        pad_x: f32,
        size: f32,
        point: Point2D,
    ) -> usize {
        let text = input.text();
        let relative = (point.x - (rect.origin.x + pad_x)).max(0.0);
        let mut width = 0.0;
        for (index, character) in text.char_indices() {
            let advance = estimated_text_width(&character.to_string(), size);
            if relative < width + advance / 2.0 {
                return index;
            }
            width += advance;
        }
        text.len()
    }

    /// Whether `field` is the one the caret paints in.
    pub(super) fn field_focused(&self, field: SceneTemplateFocus) -> bool {
        let center = &self.state.editor_ui.scene_template_center;
        // A hidden row cannot hold focus: the filter can change under a
        // focused topic field, and a caret blinking in an unpainted input
        // would leave the panel with no visible focus at all.
        if field == SceneTemplateFocus::Generate && !self.generate_row_visible() {
            return false;
        }
        if center.focus == SceneTemplateFocus::Generate && !self.generate_row_visible() {
            return field == SceneTemplateFocus::Search;
        }
        center.focus == field
    }
}

/// Chip label size, shared by the rect math here and the paint pass.
pub(super) const CHIP_LABEL_SIZE: f32 = 11.0;

fn chip_width(label: &str) -> f32 {
    // Reuses the Prompt Center's estimate on purpose: the two chip rows sit
    // in identically sized panels, and a second width model would drift them
    // apart for the same label.
    estimated_text_width(label, CHIP_LABEL_SIZE) + 20.0
}

#[cfg(test)]
#[path = "scene_template_panel_tests.rs"]
mod scene_template_panel_tests;

#[cfg(test)]
#[path = "scene_template_generate_tests.rs"]
mod scene_template_generate_tests;
