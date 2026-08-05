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
//! The panel is a gallery, not a dialog: it fills the canvas region inset by
//! [`SCENE_TEMPLATE_GALLERY_INSET`], over a scrim that dims the editor behind
//! it. Nothing here has a fixed size — the column count, the card width, and
//! the card height all fall out of how much room the panel got, so the same
//! layout serves a laptop and a 32" display without a second code path.
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
use super::scene_template_card_actions::{
    basis_chip_reserved_width, SCENE_TEMPLATE_BASIS_CHIP_HOVER,
};
use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::{Color, Point2D, Rect};

/// Margin between the canvas region and the gallery on every side.
///
/// The Asset Center is a full-canvas gallery, not a dialog: it has no
/// intrinsic size, only this inset. The margin exists so the rounded frame
/// and its scrim still read as a layer above the editor rather than as a new
/// window that replaced it.
pub const SCENE_TEMPLATE_GALLERY_INSET: f32 = 24.0;

/// Widest the centred content column ever gets, whatever the panel does.
///
/// The backdrop goes edge to edge; the rows inside do not. A search field or
/// a grid row stretched across a 32" display reads as a spreadsheet, and a
/// four-card row is already as much as the eye scans at once.
pub const SCENE_TEMPLATE_CONTENT_MAX_W: f32 = 1680.0;

/// Dimming laid over the editor behind the gallery.
///
/// Slightly heavier than the modal scrims (0.45) because the gallery is the
/// only overlay whose own surface is opaque edge to edge — a lighter wash
/// would show only in the thin margin, where it reads as a drop shadow rather
/// than as the editor stepping back.
pub const SCENE_TEMPLATE_SCRIM: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.5,
};

/// Hover token for the close button.
pub const SCENE_TEMPLATE_CLOSE_HOVER: usize = usize::MAX;

/// Hover token for the generate button.
pub const SCENE_TEMPLATE_GENERATE_HOVER: usize = usize::MAX - 64;

const FILTER_HOVER_BASE: usize = usize::MAX - 32;
/// Tab chips reserve their own token band. It sits below the filter band
/// (which reaches `FILTER_HOVER_BASE + scene count`) so the two can never
/// collide as either row grows.
const TAB_HOVER_BASE: usize = usize::MAX - 96;

pub(super) const PAD: f32 = 24.0;
pub(super) const HEADER_H: f32 = 72.0;
pub(super) const TITLE_SIZE: f32 = 24.0;
pub(super) const TAB_ROW_H: f32 = 44.0;
pub(super) const SEARCH_ROW_H: f32 = 52.0;
pub(super) const FILTER_ROW_H: f32 = 44.0;
pub(super) const CLOSE_BTN: f32 = 32.0;
const SEARCH_H: f32 = 38.0;
pub(super) const SEARCH_TEXT_SIZE: f32 = 13.0;
/// Left inset of the search text, clearing the magnifier glyph. Shared by
/// paint and the caret hit-test so a click lands where the glyph is drawn.
pub(super) const SEARCH_PAD_X: f32 = 34.0;
pub(super) const CHIP_H: f32 = 28.0;
const CHIP_GAP: f32 = 8.0;
const CARD_GAP: f32 = 20.0;
pub(super) const GENERATE_ROW_H: f32 = 72.0;
pub(super) const GENERATE_INPUT_H: f32 = 38.0;
pub(super) const GENERATE_BUTTON_W: f32 = 108.0;
pub(super) const GENERATE_GAP: f32 = 10.0;
pub(super) const GENERATE_TEXT_SIZE: f32 = 13.0;
pub(super) const GENERATE_HINT_SIZE: f32 = 11.0;
/// Left inset of the topic text, clearing the sparkle glyph. Shared by paint
/// and the caret hit-test, for the same reason [`SEARCH_PAD_X`] is.
pub(super) const GENERATE_INPUT_PAD_X: f32 = 34.0;
pub(super) const CARD_PREVIEW_INSET: f32 = 10.0;
pub(super) const CARD_PREVIEW_ASPECT: f32 = 16.0 / 10.0;
/// Title, two summary lines, and the metadata row under a template preview.
/// Fixed while the preview above it flows, so a wider column buys picture
/// rather than whitespace.
pub(super) const CARD_TEXT_H: f32 = 100.0;
/// Style cards carry no preview image yet (M2 bakes one), so they are a
/// name, a colour band, and a line of tags — a third the height of a
/// template card.
pub(super) const STYLE_CARD_H: f32 = 108.0;
pub(super) const STYLE_SWATCH_H: f32 = 20.0;

/// Column count for a card viewport of `width`.
///
/// Wide layouts buy more cards per row rather than ever-wider cards: past
/// roughly 440 px a 16:10 preview stops gaining legibility and the row starts
/// reading as a banner strip.
pub(super) fn grid_columns(width: f32) -> usize {
    if width >= 1400.0 {
        4
    } else if width >= 1000.0 {
        3
    } else {
        2
    }
}

/// Card width for `columns` cards sharing a viewport of `viewport_w`.
pub(super) fn card_width(viewport_w: f32, columns: usize) -> f32 {
    ((viewport_w - CARD_GAP * (columns - 1) as f32) / columns as f32).max(1.0)
}

/// Preview height for a template card of `card_w`, at the card aspect.
pub(super) fn preview_height(card_w: f32) -> f32 {
    (card_w - CARD_PREVIEW_INSET * 2.0).max(0.0) / CARD_PREVIEW_ASPECT
}

/// Template-card height derived from its width.
///
/// Derived rather than a constant: the panel is now the size of the canvas,
/// so a fixed height written for a 720 px dialog would clamp every preview to
/// a letterbox no matter how much room the row has.
pub(super) fn template_card_height(card_w: f32) -> f32 {
    CARD_PREVIEW_INSET + preview_height(card_w) + CARD_TEXT_H
}

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
    /// Bring this template's boards into the editor.
    ///
    /// What that means is the host's call, and it turns on what is already
    /// open: an untouched starter is replaced (so the template simply *is*
    /// the document), anything else gets the boards appended beside its
    /// existing content. The widget deliberately does not decide — it cannot
    /// see unsaved-work prompts or the recent-files list.
    AddTemplateToCanvas(String),
    /// Aim the generate row at this template: pin its style guide, narrow
    /// the grid to its scene, and focus the topic field. Touches no document.
    GenerateFromTemplate(String),
    /// Dismiss the generate row's basis chip, unpinning the style it set.
    ClearGenerateBasis,
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

    /// The centred content column every row inside the gallery measures from.
    ///
    /// The panel fills the canvas, but its contents do not: everything from
    /// the title to the card grid lives in a column capped at
    /// [`SCENE_TEMPLATE_CONTENT_MAX_W`] and centred in the panel. Only the
    /// backdrop and the header rule run edge to edge.
    ///
    /// Every rect below derives its x from here, so widening the cap moves
    /// paint and hit-testing together by construction.
    pub fn content_rect(panel: Rect) -> Rect {
        let width = (panel.size.x - PAD * 2.0).clamp(0.0, SCENE_TEMPLATE_CONTENT_MAX_W);
        Rect::xywh(
            panel.origin.x + ((panel.size.x - width) / 2.0).max(0.0),
            panel.origin.y,
            width,
            panel.size.y,
        )
    }

    pub fn close_rect(panel: Rect) -> Rect {
        let content = Self::content_rect(panel);
        Rect::xywh(
            content.origin.x + content.size.x - CLOSE_BTN,
            panel.origin.y + (HEADER_H - CLOSE_BTN) / 2.0,
            CLOSE_BTN,
            CLOSE_BTN,
        )
    }

    pub fn search_rect(panel: Rect) -> Rect {
        let content = Self::content_rect(panel);
        Rect::xywh(
            content.origin.x,
            panel.origin.y + HEADER_H + TAB_ROW_H + (SEARCH_ROW_H - SEARCH_H) / 2.0,
            content.size.x,
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
        let mut x = Self::content_rect(panel).origin.x;
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
        let mut x = Self::content_rect(panel).origin.x;
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
    ///
    /// The basis chip, when there is one, eats into the field from the left
    /// rather than sitting above it: the chip is a modifier on the topic
    /// about to be typed, and a row reads as one sentence only while the two
    /// stay on one line.
    pub fn generate_input_rect(&self, panel: Rect) -> Option<Rect> {
        if !self.generate_row_visible() {
            return None;
        }
        let content = Self::content_rect(panel);
        let reserved = basis_chip_reserved_width(self.basis_chip_rect(panel), GENERATE_GAP);
        Some(Rect::xywh(
            content.origin.x + reserved,
            self.generate_row_top(panel) + 6.0,
            (content.size.x - reserved - GENERATE_BUTTON_W - GENERATE_GAP).max(0.0),
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
        let content = Self::content_rect(panel);
        let top = self.cards_top(panel);
        Rect::xywh(
            content.origin.x,
            top,
            content.size.x,
            (panel.origin.y + panel.size.y - PAD - top).max(0.0),
        )
    }

    /// Column count, card width, and row height of the grid the active tab
    /// paints. One walker serves both tabs; only these three numbers differ.
    ///
    /// All three flow from the panel: the columns from how much room the card
    /// viewport has, and — for templates — the height from the width, so the
    /// preview keeps its aspect at every breakpoint. Style cards keep a fixed
    /// height because they have no picture to scale, but they share the column
    /// count so the two tabs do not disagree about how wide a card is.
    pub(super) fn grid_metrics(&self, panel: Rect) -> (usize, f32, f32) {
        let viewport_w = self.cards_viewport(panel).size.x;
        let columns = grid_columns(viewport_w);
        let card_w = card_width(viewport_w, columns);
        let card_h = match self.tab() {
            AssetCenterTab::Templates => template_card_height(card_w),
            AssetCenterTab::Styles => STYLE_CARD_H,
        };
        (columns, card_w, card_h)
    }

    pub(super) fn content_height_for_count(&self, panel: Rect, count: usize) -> f32 {
        let (columns, _, card_h) = self.grid_metrics(panel);
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
        (self.content_height_for_count(panel, count) - viewport.size.y).max(0.0)
    }

    pub(super) fn card_rects_for_count(&self, panel: Rect, count: usize) -> Vec<(usize, Rect)> {
        let viewport = self.cards_viewport(panel);
        let (columns, card_w, card_h) = self.grid_metrics(panel);
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
            .basis_chip_dismiss_rect(panel)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(SCENE_TEMPLATE_BASIS_CHIP_HOVER);
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
        let (index, card) = self
            .card_rects(panel)
            .into_iter()
            .find(|(_, rect)| rect.contains(point))?;
        Some(self.card_hover_token(index, card, point))
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
        // Above the topic field: the chip sits in the same row and its
        // dismiss target must not read as a click into the text.
        if self
            .basis_chip_dismiss_rect(panel)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(SceneTemplateHit::ClearGenerateBasis);
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
                            return Some(self.template_card_hit(cards[index], index, rect, point));
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
pub(super) const CHIP_LABEL_SIZE: f32 = 12.0;

fn chip_width(label: &str) -> f32 {
    // Reuses the Prompt Center's estimate on purpose: the tab row and the
    // filter row use the same chip shape, and a second width model would
    // drift them apart for the same label.
    estimated_text_width(label, CHIP_LABEL_SIZE) + 26.0
}

/// Gallery rects the hosts would hand the widget, for tests.
///
/// The panel has no intrinsic size any more, so a test cannot name one; it
/// names the canvas region it is standing in instead. The three widths
/// straddle both [`grid_columns`] breakpoints, which is the whole reason more
/// than one exists.
#[cfg(test)]
pub(super) mod test_rects {
    use crate::{Point2D, Rect};

    /// A 1440x900 canvas region at (40, 60), inset — the laptop case, and the
    /// default fixture. Three columns.
    pub(in crate::widgets) const MEDIUM: Rect = Rect {
        origin: Point2D { x: 64.0, y: 84.0 },
        size: Point2D {
            x: 1392.0,
            y: 852.0,
        },
    };

    /// A 900x700 canvas region at the origin, inset. Two columns.
    pub(in crate::widgets) const NARROW: Rect = Rect {
        origin: Point2D { x: 24.0, y: 24.0 },
        size: Point2D { x: 852.0, y: 652.0 },
    };

    /// A 2200x1200 canvas region at the origin, inset — wide enough that the
    /// content column hits its cap. Four columns.
    pub(in crate::widgets) const WIDE: Rect = Rect {
        origin: Point2D { x: 24.0, y: 24.0 },
        size: Point2D {
            x: 2152.0,
            y: 1152.0,
        },
    };
}

#[cfg(test)]
#[path = "scene_template_panel_tests.rs"]
mod scene_template_panel_tests;

#[cfg(test)]
#[path = "scene_template_generate_tests.rs"]
mod scene_template_generate_tests;
