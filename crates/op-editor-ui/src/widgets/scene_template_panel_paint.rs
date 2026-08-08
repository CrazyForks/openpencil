//! Scene Template Center painting, split from geometry to keep both compact.

use op_editor_core::scene_template_catalog::SceneTemplateDefinition;

use op_editor_core::SceneTemplateFocus;

use super::scene_template_card_actions::{
    card_action_rects, card_add_hover_token, card_generate_hover_token, ACTION_LABEL_SIZE,
    ACTION_RADIUS, BASIS_CHIP_DISMISS_W, BASIS_CHIP_H, BASIS_CHIP_LABEL_SIZE, BASIS_CHIP_PAD_X,
    SCENE_TEMPLATE_BASIS_CHIP_HOVER,
};
use super::scene_template_panel::{
    filter_hover_token, preview_height, tab_hover_token, SceneTemplatePanel, CARD_PREVIEW_INSET,
    CHIP_H, CHIP_LABEL_SIZE, CLOSE_BTN, GENERATE_HINT_SIZE, GENERATE_INPUT_PAD_X,
    GENERATE_TEXT_SIZE, HEADER_H, SCENE_TEMPLATE_CLOSE_HOVER, SCENE_TEMPLATE_GENERATE_HOVER,
    SEARCH_PAD_X, SEARCH_TEXT_SIZE, TITLE_SIZE,
};
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::canvas_viewport_image::{
    has_cached_image_bytes, note_pending_decode, required_raster_edge, store_remote_image_bytes,
};
use crate::widgets::prompt_center_panel::estimated_text_width;
use crate::widgets::property_panel_text_input::paint_text_input_view;
use crate::widgets::scene_template_previews::scene_template_preview;
use crate::widgets::{draw_icon, Icon, PaintCx};
use crate::{Color, ImageDrawMode, Point2D, Rect, TextLayout};

const CARD_RADIUS: f32 = 12.0;
const CARD_TITLE_SIZE: f32 = 14.0;
const SUMMARY_SIZE: f32 = 11.5;
const META_SIZE: f32 = 11.0;
/// Corner radius of the gallery frame itself. Larger than a dropdown's
/// because the shape is read at canvas scale, not at menu scale.
const PANEL_RADIUS: f32 = 16.0;

impl SceneTemplatePanel<'_> {
    /// Paint the complete gallery.
    pub fn paint(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        cx.backend
            .fill_round_rect(panel, PANEL_RADIUS, self.theme.popover);
        cx.backend
            .stroke_round_rect(panel, PANEL_RADIUS, self.theme.border, 1.0);
        self.paint_header(cx, panel);
        self.paint_tab_chips(cx, panel);
        self.paint_search(cx, panel);
        self.paint_filter_chips(cx, panel);
        self.paint_generate_row(cx, panel);
        match self.tab() {
            op_editor_core::AssetCenterTab::Templates => self.paint_cards(cx, panel),
            op_editor_core::AssetCenterTab::Styles => self.paint_style_cards(cx, panel),
        }
        // Last, over everything: the paste box is the panel's topmost layer,
        // matching the press ladder that gives it every click inside the panel.
        self.paint_style_import(cx, panel);
    }

    /// The tab row. Chips rather than underlined labels: the filter row
    /// directly below already reads as chips, and two chip rows with
    /// different selection colours is a smaller vocabulary than two
    /// different selection idioms stacked on each other.
    fn paint_tab_chips(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        for (index, (rect, tab)) in self.tab_chip_rects(panel).into_iter().enumerate() {
            let active = self.tab() == tab;
            let (fill, foreground) = if active {
                (self.theme.primary, self.theme.primary_foreground)
            } else {
                (self.theme.muted, self.theme.muted_foreground)
            };
            cx.backend.fill_round_rect(rect, CHIP_H / 2.0, fill);
            paint_button_feedback_wash(
                cx.backend,
                &self.theme,
                rect,
                CHIP_H / 2.0,
                self.state.editor_ui.scene_template_center.hover == Some(tab_hover_token(index)),
                self.is_pressed(tab_hover_token(index)),
            );
            let label = self.tab_label(tab);
            let label_w = estimated_text_width(label, CHIP_LABEL_SIZE);
            self.paint_text(
                cx,
                label,
                Point2D::new(
                    rect.origin.x + ((rect.size.x - label_w) / 2.0).max(5.0),
                    jian_widgets::centered_text_baseline_y(rect, CHIP_LABEL_SIZE),
                ),
                CHIP_LABEL_SIZE,
                foreground,
            );
        }
    }

    fn paint_header(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let content = Self::content_rect(panel);
        self.paint_text(
            cx,
            self.t("assetCenter.title", "资产中心"),
            Point2D::new(
                content.origin.x,
                jian_widgets::centered_text_baseline_y(
                    Rect::xywh(content.origin.x, panel.origin.y, content.size.x, HEADER_H),
                    TITLE_SIZE,
                ),
            ),
            TITLE_SIZE,
            self.theme.foreground,
        );

        let close = Self::close_rect(panel);
        jian_widgets::components::icon_button::IconButton {
            icon_paths: Icon::Close.paths(),
            hovered: self.state.editor_ui.scene_template_center.hover
                == Some(SCENE_TEMPLATE_CLOSE_HOVER),
            pressed: self.is_pressed(SCENE_TEMPLATE_CLOSE_HOVER),
            active: false,
            enabled: true,
            icon_size: CLOSE_BTN - 14.0,
            stroke_width: 1.5,
        }
        .paint(
            cx.backend,
            close,
            &crate::widgets::button::tokens_from_theme(&self.theme),
        );

        cx.backend.fill_rect(
            Rect::xywh(panel.origin.x, panel.origin.y + HEADER_H, panel.size.x, 1.0),
            self.theme.border,
        );
    }

    fn paint_search(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let rect = Self::search_rect(panel);
        cx.backend.fill_round_rect(rect, 9.0, self.theme.muted);
        cx.backend
            .stroke_round_rect(rect, 9.0, self.theme.border, 1.0);
        draw_icon(
            cx.backend,
            Icon::Search,
            Point2D::new(rect.origin.x + 10.0, rect.origin.y + 11.0),
            16.0,
            self.theme.muted_foreground,
            1.4,
        );
        paint_text_input_view(
            cx,
            &self.theme,
            &self.state.editor_ui.scene_template_center.search,
            rect,
            SEARCH_TEXT_SIZE,
            SEARCH_PAD_X,
            jian_widgets::centered_text_baseline_y(rect, SEARCH_TEXT_SIZE),
            self.now_ms(),
            match self.tab() {
                op_editor_core::AssetCenterTab::Templates => {
                    self.t("sceneTemplate.searchPlaceholder", "搜索场景或模板")
                }
                op_editor_core::AssetCenterTab::Styles => {
                    self.t("assetCenter.style.searchPlaceholder", "搜索风格或标签")
                }
            },
            self.field_focused(SceneTemplateFocus::Search),
        );
    }

    /// The prompt-to-deck row: a topic field, a generate button, and one line
    /// saying what pressing it does. The sentence is not decoration — the row
    /// replaces the document, and a control that quietly discards the canvas
    /// has to say so before it is pressed, not after.
    fn paint_generate_row(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let Some(input) = self.generate_input_rect(panel) else {
            return;
        };
        let button = self
            .generate_button_rect(panel)
            .expect("the button rect exists whenever the input does");

        self.paint_basis_chip(cx, panel);

        cx.backend.fill_round_rect(input, 9.0, self.theme.muted);
        cx.backend
            .stroke_round_rect(input, 9.0, self.theme.border, 1.0);
        draw_icon(
            cx.backend,
            Icon::Sparkles,
            Point2D::new(input.origin.x + 10.0, input.origin.y + 11.0),
            16.0,
            self.theme.muted_foreground,
            1.4,
        );
        paint_text_input_view(
            cx,
            &self.theme,
            &self.state.editor_ui.scene_template_center.generate,
            input,
            GENERATE_TEXT_SIZE,
            GENERATE_INPUT_PAD_X,
            jian_widgets::centered_text_baseline_y(input, GENERATE_TEXT_SIZE),
            self.now_ms(),
            self.t(
                "sceneTemplate.generate.placeholder",
                "描述主题，AI 直接生成整副演示文稿",
            ),
            self.field_focused(SceneTemplateFocus::Generate),
        );

        cx.backend.fill_round_rect(button, 9.0, self.theme.primary);
        paint_button_feedback_wash(
            cx.backend,
            &self.theme,
            button,
            7.0,
            self.state.editor_ui.scene_template_center.hover == Some(SCENE_TEMPLATE_GENERATE_HOVER),
            self.is_pressed(SCENE_TEMPLATE_GENERATE_HOVER),
        );
        let label = self.t("sceneTemplate.generate.button", "生成");
        let label_w = estimated_text_width(label, GENERATE_TEXT_SIZE);
        self.paint_text(
            cx,
            label,
            Point2D::new(
                button.origin.x + ((button.size.x - label_w) / 2.0).max(6.0),
                jian_widgets::centered_text_baseline_y(button, GENERATE_TEXT_SIZE),
            ),
            GENERATE_TEXT_SIZE,
            self.theme.primary_foreground,
        );

        self.paint_text(
            cx,
            // The deck-specific wording belongs to the Templates tab, where
            // the row is gated to the slides scene. On the Styles tab the row
            // means "generate anything, in this aesthetic".
            match self.tab() {
                op_editor_core::AssetCenterTab::Templates => self.t(
                    "sceneTemplate.generate.hint",
                    "新建一个文档，按主题直接生成整副演示文稿。",
                ),
                op_editor_core::AssetCenterTab::Styles => self.t(
                    "assetCenter.style.generateHint",
                    "新建一个文档，按主题生成；已钉住的风格会被直接采用。",
                ),
            },
            Point2D::new(input.origin.x + 2.0, input.origin.y + input.size.y + 18.0),
            GENERATE_HINT_SIZE,
            self.theme.muted_foreground,
        );
    }

    fn paint_filter_chips(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        for (index, (rect, filter)) in self.filter_chip_rects(panel).into_iter().enumerate() {
            let active = self.state.editor_ui.scene_template_center.filter == filter;
            let (fill, foreground) = if active {
                (self.theme.primary, self.theme.primary_foreground)
            } else {
                (self.theme.muted, self.theme.muted_foreground)
            };
            cx.backend.fill_round_rect(rect, CHIP_H / 2.0, fill);
            paint_button_feedback_wash(
                cx.backend,
                &self.theme,
                rect,
                CHIP_H / 2.0,
                self.state.editor_ui.scene_template_center.hover == Some(filter_hover_token(index)),
                self.is_pressed(filter_hover_token(index)),
            );
            let label = self.filter_label(filter);
            let label_w = estimated_text_width(label, CHIP_LABEL_SIZE);
            self.paint_text(
                cx,
                label,
                Point2D::new(
                    rect.origin.x + ((rect.size.x - label_w) / 2.0).max(5.0),
                    jian_widgets::centered_text_baseline_y(rect, CHIP_LABEL_SIZE),
                ),
                CHIP_LABEL_SIZE,
                foreground,
            );
        }
    }

    fn paint_cards(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let viewport = self.cards_viewport(panel);
        let templates = self.filtered();
        if templates.is_empty() {
            self.paint_text(
                cx,
                self.t("sceneTemplate.empty", "没有匹配的模板"),
                Point2D::new(viewport.origin.x + 4.0, viewport.origin.y + 28.0),
                12.0,
                self.theme.muted_foreground,
            );
            return;
        }

        cx.backend.save();
        cx.backend.clip_rect(viewport);
        for (index, rect) in self.card_rects_for_count(panel, templates.len()) {
            // Cheap reject for rows scrolled out of view: their rects are
            // still computed so hover and paint agree on indices.
            if rect.origin.y > viewport.origin.y + viewport.size.y
                || rect.origin.y + rect.size.y < viewport.origin.y
            {
                continue;
            }
            self.paint_card(cx, rect, templates[index], index);
        }
        cx.backend.restore();
    }

    fn paint_card(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        template: &'static SceneTemplateDefinition,
        index: usize,
    ) {
        cx.backend
            .fill_round_rect(rect, CARD_RADIUS, self.theme.card);
        cx.backend
            .stroke_round_rect(rect, CARD_RADIUS, self.theme.border, 1.0);
        paint_button_feedback_wash(
            cx.backend,
            &self.theme,
            rect,
            CARD_RADIUS,
            self.state.editor_ui.scene_template_center.hover == Some(index),
            self.is_pressed(index),
        );

        let preview = Self::card_preview_rect(rect);
        self.paint_card_preview(cx, preview, template);

        let text_x = rect.origin.x + CARD_PREVIEW_INSET + 4.0;
        let text_w = rect.size.x - (CARD_PREVIEW_INSET + 4.0) * 2.0;
        self.paint_text(
            cx,
            &truncate_to_width(
                template.title_for_locale(self.locale),
                text_w,
                CARD_TITLE_SIZE,
            ),
            Point2D::new(text_x, preview.origin.y + preview.size.y + 26.0),
            CARD_TITLE_SIZE,
            self.theme.foreground,
        );

        // Two summary lines, wrapped on the same width estimate the rest of
        // the panel uses.
        let summary = template.summary_for_locale(self.locale);
        let mut y = preview.origin.y + preview.size.y + 46.0;
        for line in wrap_to_width(summary, text_w, SUMMARY_SIZE, 2) {
            self.paint_text(
                cx,
                &line,
                Point2D::new(text_x, y),
                SUMMARY_SIZE,
                self.theme.muted_foreground,
            );
            y += 17.0;
        }

        self.paint_text(
            cx,
            &self.metadata(template),
            Point2D::new(text_x, rect.origin.y + rect.size.y - 14.0),
            META_SIZE,
            self.theme.muted_foreground,
        );

        // Last, so the strip sits over the picture rather than under it.
        if self.card_actions_visible(index) {
            self.paint_card_actions(cx, rect, template, index);
        }
    }

    /// The hover strip: what this card can do, spelled out.
    ///
    /// Only on hover, because the answer is the same for every card and a
    /// grid that repeated it forty times would be reading its own manual.
    /// The primary is filled and the secondary is a bordered surface, which
    /// is the same weight relationship the panel's other button pairs use —
    /// pressing the card does what the filled one says.
    fn paint_card_actions(
        &self,
        cx: &mut PaintCx<'_>,
        card: Rect,
        template: &'static SceneTemplateDefinition,
        index: usize,
    ) {
        let (add, generate) = card_action_rects(card, self.card_offers_generate(template));
        self.paint_card_action(
            cx,
            add,
            self.t("sceneTemplate.card.addToCanvas", "加入画布"),
            true,
            card_add_hover_token(index),
        );
        if let Some(rect) = generate {
            self.paint_card_action(
                cx,
                rect,
                self.t("sceneTemplate.card.generateFrom", "以此生成"),
                false,
                card_generate_hover_token(index),
            );
        }
    }

    fn paint_card_action(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        label: &str,
        primary: bool,
        token: usize,
    ) {
        let (fill, foreground) = if primary {
            (self.theme.primary, self.theme.primary_foreground)
        } else {
            (self.theme.card, self.theme.foreground)
        };
        cx.backend.fill_round_rect(rect, ACTION_RADIUS, fill);
        if !primary {
            cx.backend
                .stroke_round_rect(rect, ACTION_RADIUS, self.theme.border, 1.0);
        }
        paint_button_feedback_wash(
            cx.backend,
            &self.theme,
            rect,
            ACTION_RADIUS,
            self.state.editor_ui.scene_template_center.hover == Some(token),
            self.is_pressed(token),
        );
        let label = truncate_to_width(label, (rect.size.x - 12.0).max(0.0), ACTION_LABEL_SIZE);
        let label_w = estimated_text_width(&label, ACTION_LABEL_SIZE);
        self.paint_text(
            cx,
            &label,
            Point2D::new(
                rect.origin.x + ((rect.size.x - label_w) / 2.0).max(4.0),
                jian_widgets::centered_text_baseline_y(rect, ACTION_LABEL_SIZE),
            ),
            ACTION_LABEL_SIZE,
            foreground,
        );
    }

    /// "基于：极简 Keynote ×" — the standing answer to "in what style?".
    ///
    /// It reads as a chip rather than as a line of help text because it is
    /// removable, and the × has to look like the thing that removes it: the
    /// pin behind this chip steers every generation until it is cleared.
    fn paint_basis_chip(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let (Some(chip), Some(label)) = (self.basis_chip_rect(panel), self.basis_chip_label())
        else {
            return;
        };
        let radius = BASIS_CHIP_H / 2.0;
        cx.backend.fill_round_rect(chip, radius, self.theme.muted);
        cx.backend
            .stroke_round_rect(chip, radius, self.theme.border, 1.0);

        let text_w = (chip.size.x - BASIS_CHIP_PAD_X - BASIS_CHIP_DISMISS_W).max(0.0);
        self.paint_text(
            cx,
            &truncate_to_width(&label, text_w, BASIS_CHIP_LABEL_SIZE),
            Point2D::new(
                chip.origin.x + BASIS_CHIP_PAD_X,
                jian_widgets::centered_text_baseline_y(chip, BASIS_CHIP_LABEL_SIZE),
            ),
            BASIS_CHIP_LABEL_SIZE,
            self.theme.foreground,
        );

        let Some(dismiss) = self.basis_chip_dismiss_rect(panel) else {
            return;
        };
        paint_button_feedback_wash(
            cx.backend,
            &self.theme,
            dismiss,
            radius,
            self.state.editor_ui.scene_template_center.hover
                == Some(SCENE_TEMPLATE_BASIS_CHIP_HOVER),
            self.is_pressed(SCENE_TEMPLATE_BASIS_CHIP_HOVER),
        );
        const GLYPH: f32 = 11.0;
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(
                dismiss.origin.x + (dismiss.size.x - GLYPH) / 2.0,
                dismiss.origin.y + (dismiss.size.y - GLYPH) / 2.0,
            ),
            GLYPH,
            self.theme.muted_foreground,
            1.5,
        );
    }

    fn paint_card_preview(
        &self,
        cx: &mut PaintCx<'_>,
        preview: Rect,
        template: &'static SceneTemplateDefinition,
    ) {
        let Some(asset) = scene_template_preview(&template.id) else {
            cx.backend.fill_round_rect(preview, 9.0, self.theme.muted);
            return;
        };
        let image_id = asset.image_id;
        let Some(encoded) = asset.bytes else {
            // Web only: not in the bundle and not fetched yet. Ask the host and
            // paint the plain block meanwhile — the card's title and metadata
            // are already readable, so a slow or failed fetch costs the picture
            // and nothing else.
            op_editor_core::web_assets::request(asset.route);
            cx.backend.fill_round_rect(preview, 9.0, self.theme.muted);
            return;
        };
        // Same decode handshake the Prompt Center uses: register the bytes,
        // request the raster at the size actually painted, and fall back to a
        // plain block until the decode lands so the first frame never blocks.
        if !has_cached_image_bytes(image_id) {
            store_remote_image_bytes(image_id, encoded.to_vec());
        }
        let max_edge_px = required_raster_edge(preview, cx.backend.dpi_scale());
        let sharp = cx.backend.image_decoded(image_id, encoded, max_edge_px);
        if !sharp {
            note_pending_decode(image_id, max_edge_px);
        }
        if !sharp && !cx.backend.image_resident(image_id) {
            cx.backend.fill_round_rect(preview, 9.0, self.theme.muted);
            return;
        }
        cx.backend.save();
        cx.backend.clip_round_rect(preview, 9.0);
        cx.backend
            .draw_image_with_mode(preview, image_id, encoded, ImageDrawMode::Fill);
        cx.backend.restore();
    }

    /// The preview's rect inside its card.
    ///
    /// No clamp: the card height is derived from exactly this height (see
    /// `template_card_height`), so the preview always gets its full aspect and
    /// the text block below it always gets the rest.
    pub(super) fn card_preview_rect(card: Rect) -> Rect {
        Rect::xywh(
            card.origin.x + CARD_PREVIEW_INSET,
            card.origin.y + CARD_PREVIEW_INSET,
            (card.size.x - CARD_PREVIEW_INSET * 2.0).max(0.0),
            preview_height(card.size.x),
        )
    }

    /// "6 页 · 1920×1080" — what distinguishes a deck from a poster at a
    /// glance, which is the decision the card exists to support.
    fn metadata(&self, template: &SceneTemplateDefinition) -> String {
        let count = template.frames.to_string();
        let pages =
            op_i18n::translate_with(self.locale, "sceneTemplate.frames", &[("count", &count)]);
        let pages = if pages == "sceneTemplate.frames" {
            format!("{count} 页")
        } else {
            pages
        };
        format!(
            "{pages} · {}×{}",
            template.frame_width, template.frame_height
        )
    }

    pub(super) fn t(&self, key: &'static str, fallback: &'static str) -> &'static str {
        let translated = op_i18n::translate(self.locale, key);
        if translated == key {
            fallback
        } else {
            translated
        }
    }

    pub(super) fn paint_text(
        &self,
        cx: &mut PaintCx<'_>,
        text: &str,
        position: Point2D,
        size: f32,
        color: Color,
    ) {
        let layout = TextLayout::single_run(
            text,
            "system-ui",
            size,
            color.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(&layout, position);
    }
}

pub(super) fn truncate_to_width(text: &str, max_width: f32, size: f32) -> String {
    if estimated_text_width(text, size) <= max_width {
        return text.to_string();
    }
    let ellipsis_w = estimated_text_width("…", size);
    let mut out = String::new();
    let mut width = 0.0;
    for character in text.chars() {
        let advance = estimated_text_width(&character.to_string(), size);
        if width + advance + ellipsis_w > max_width {
            break;
        }
        out.push(character);
        width += advance;
    }
    out.push('…');
    out
}

/// Greedy wrap to at most `max_lines`, ellipsising the last line when the
/// text does not fit. Breaks per character, which is right for the CJK the
/// summaries are written in and acceptable for the Latin ones at this size.
fn wrap_to_width(text: &str, max_width: f32, size: f32, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut width = 0.0;
    for character in text.chars() {
        let advance = estimated_text_width(&character.to_string(), size);
        if width + advance > max_width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            width = 0.0;
            if lines.len() == max_lines {
                break;
            }
        }
        current.push(character);
        width += advance;
    }
    if lines.len() < max_lines && !current.is_empty() {
        lines.push(current);
    }
    // Anything past the cap is dropped, so the final line must show that.
    if lines.len() == max_lines {
        let consumed: usize = lines.iter().map(|line| line.chars().count()).sum();
        if text.chars().count() > consumed {
            let last = lines.pop().unwrap_or_default();
            lines.push(truncate_to_width(&last, max_width, size));
        }
    }
    lines
}
