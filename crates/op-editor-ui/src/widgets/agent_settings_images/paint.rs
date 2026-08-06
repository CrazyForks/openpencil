//! Paint pass for the Images settings tab — the search-credential
//! section, its test-status row, and the image-generation profile list.
//! Carved off `agent_settings_images.rs` to keep every file under the
//! 800-line cap; all rect maths lives on the spine.

use super::*;
use crate::widgets::text_metrics;

pub(in crate::widgets) fn paint_images_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
    now_ms: u64,
) {
    let title_str = t_settings(ui, "settings.images.search");
    let title = TextLayout::single_run(
        title_str,
        "system-ui",
        15.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(content.origin.x, content.origin.y + 20.0),
    );
    let title_w = text_metrics::measure_chrome(cx.backend, title_str, 15.0);
    let ready = settings.images_search_ready;
    let dot_color = if ready {
        theme.status_success
    } else {
        theme.muted_foreground
    };
    // Dot vertically aligned with the status text optical centre,
    // not the title baseline — keeps "● Ready" reading as one
    // horizontal pill instead of the dot drifting downward.
    let dot = Rect {
        origin: Point2D::new(content.origin.x + title_w + 14.0, content.origin.y + 11.0),
        size: Point2D::new(8.0, 8.0),
    };
    cx.backend.fill_oval(dot, dot_color);
    let status_text = if ready {
        t_settings(ui, "settings.images.ready")
    } else {
        t_settings(ui, "settings.images.notConfigured")
    };
    let status = TextLayout::single_run(
        status_text,
        "system-ui",
        12.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &status,
        Point2D::new(content.origin.x + title_w + 30.0, content.origin.y + 20.0),
    );

    // Advanced collapsible row.
    let toggle = advanced_toggle_rect(content);
    let chev_icon = if settings.images_advanced_open {
        Icon::ChevronDown
    } else {
        Icon::ChevronRight
    };
    draw_icon(
        cx.backend,
        chev_icon,
        Point2D::new(
            toggle.origin.x,
            toggle.origin.y + (ADVANCED_ROW_H - 14.0) / 2.0,
        ),
        14.0,
        theme.muted_foreground,
        1.8,
    );
    let advanced_label = TextLayout::single_run(
        t_settings(ui, "settings.images.advanced"),
        "system-ui",
        13.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &advanced_label,
        Point2D::new(toggle.origin.x + 22.0, toggle.origin.y + 17.0),
    );

    if settings.images_advanced_open {
        let mut y = toggle.origin.y + ADVANCED_ROW_H;
        let sub = TextLayout::single_run(
            t_settings(ui, "settings.images.oauthLabel"),
            "system-ui",
            12.0,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&sub, Point2D::new(content.origin.x, y + 14.0));
        y += SUBTITLE_H;
        paint_search_input_row(
            cx,
            theme,
            settings,
            ui,
            ImageSearchField::ClientId,
            t_settings(ui, "settings.images.clientId"),
            t_settings(ui, "settings.images.clientIdPlaceholder"),
            content.origin.x,
            y,
            content.size.x,
            now_ms,
        );
        y += ROW_H + ROW_VGAP;
        paint_search_input_row(
            cx,
            theme,
            settings,
            ui,
            ImageSearchField::ClientSecret,
            t_settings(ui, "settings.images.clientSecret"),
            t_settings(ui, "settings.images.clientSecretPlaceholder"),
            content.origin.x,
            y,
            content.size.x,
            now_ms,
        );
        y += ROW_H + BODY_GAP;
        let link_text = t_settings(ui, "settings.images.registerLink");
        let link = TextLayout::single_run(
            link_text,
            "system-ui",
            12.0,
            (theme.primary).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&link, Point2D::new(content.origin.x, y + 22.0));
        let link_w = text_metrics::measure_chrome(cx.backend, link_text, 12.0);
        // Underline the link + arrow on hover (link affordance).
        if settings.hover_image_search_register_link {
            let underline = Rect::xywh(content.origin.x, y + 26.0, link_w + 20.0, 1.0);
            cx.backend.fill_rect(underline, theme.primary);
        }
        draw_icon(
            cx.backend,
            Icon::ArrowUpRight,
            Point2D::new(content.origin.x + link_w + 6.0, y + 10.0),
            14.0,
            theme.primary,
            1.6,
        );
        let test_btn = test_btn_rect(content, settings);
        paint_search_test_status(cx, theme, settings, test_btn);
        cx.backend.fill_round_rect(test_btn, 6.0, theme.muted);
        crate::widgets::button::paint_ghost_button_feedback(
            cx.backend,
            theme,
            test_btn,
            settings.hover_image_search_test_button,
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::ImageSearchTest,
            )),
        );
        cx.backend
            .stroke_round_rect(test_btn, 6.0, theme.border, 1.0);
        let test_label = t_settings(ui, "settings.images.test");
        let test_w = text_metrics::measure_chrome(cx.backend, test_label, 13.0);
        let test_lay = TextLayout::single_run(
            test_label,
            "system-ui",
            13.0,
            (if search_test_enabled(settings) {
                theme.foreground
            } else {
                theme.muted_foreground
            })
            .to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &test_lay,
            Point2D::new(
                test_btn.origin.x + (TEST_BTN_W - test_w) / 2.0,
                test_btn.origin.y + BTN_H / 2.0 + 5.0,
            ),
        );
    }

    // Image Generation section.
    let gen_top = image_gen_section_top(content, settings);
    let gen_title = TextLayout::single_run(
        t_settings(ui, "settings.images.generation"),
        "system-ui",
        15.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&gen_title, Point2D::new(content.origin.x, gen_top + 20.0));
    let add_btn = add_btn_rect(content, settings);
    cx.backend.fill_round_rect(add_btn, 6.0, theme.muted);
    crate::widgets::button::paint_ghost_button_feedback(
        cx.backend,
        theme,
        add_btn,
        settings.hover_image_gen_add_button,
        ui.button_pressed(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::ImageGenAdd,
        )),
    );
    cx.backend
        .stroke_round_rect(add_btn, 6.0, theme.border, 1.0);
    // Fit the label to the fixed-width button before centring it. The
    // button sits flush against the content column's right edge, so a long
    // translation ("+ Ajouter") centred on its untrimmed width hangs off
    // both sides — and the right-hand overhang leaves the modal.
    let add_label = ellipsize(
        cx,
        t_settings(ui, "settings.images.add"),
        ADD_BTN_W - 8.0,
        13.0,
    );
    let add_w = text_metrics::measure_chrome(cx.backend, &add_label, 13.0);
    let add_lay = TextLayout::single_run(
        &add_label,
        "system-ui",
        13.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &add_lay,
        Point2D::new(
            add_btn.origin.x + (ADD_BTN_W - add_w) / 2.0,
            add_btn.origin.y + BTN_H / 2.0 + 5.0,
        ),
    );

    if settings.image_gen_profiles.is_empty() {
        let hint = t_settings(ui, "settings.images.empty");
        let hint_w = text_metrics::measure_chrome(cx.backend, hint, 13.0);
        let hint_lay = TextLayout::single_run(
            hint,
            "system-ui",
            13.0,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &hint_lay,
            Point2D::new(
                content.origin.x + (content.size.x - hint_w) / 2.0,
                gen_top + SECTION_TITLE_H + PROFILE_LIST_TOP_GAP + 48.0,
            ),
        );
    } else {
        for (index, profile) in settings.image_gen_profiles.iter().enumerate() {
            let row = profile_row_rect(content, settings, index);
            paint_profile_row(cx, theme, settings, ui, profile, index, row, now_ms);
        }
    }
}

fn paint_search_test_status(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    test_btn: Rect,
) {
    match settings.images_search_test_status {
        ImageTestStatus::Idle => {}
        ImageTestStatus::Testing => draw_icon(
            cx.backend,
            Icon::Loader,
            Point2D::new(test_btn.origin.x - 20.0, test_btn.origin.y + 8.5),
            11.0,
            theme.muted_foreground,
            1.5,
        ),
        ImageTestStatus::Valid => draw_icon(
            cx.backend,
            Icon::Check,
            Point2D::new(test_btn.origin.x - 20.0, test_btn.origin.y + 8.5),
            11.0,
            theme.primary,
            1.8,
        ),
        ImageTestStatus::Invalid => {
            let label = TextLayout::single_run(
                "Invalid",
                "system-ui",
                10.0,
                (theme.destructive).to_jian(),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &label,
                Point2D::new(test_btn.origin.x - 44.0, test_btn.origin.y + 17.0),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_profile_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    profile: &ImageGenProfile,
    index: usize,
    row: Rect,
    now_ms: u64,
) {
    let active = settings.active_image_gen_profile_id.as_deref() == Some(profile.id.as_str());
    let editing = is_editing_profile(settings, index);
    if active || editing {
        cx.backend.fill_round_rect(row, 6.0, theme.muted);
        cx.backend.stroke_round_rect(
            row,
            6.0,
            if active { theme.primary } else { theme.border },
            1.0,
        );
    } else {
        cx.backend.stroke_round_rect(row, 6.0, theme.border, 1.0);
    }
    crate::widgets::button::paint_ghost_button_feedback(
        cx.backend,
        theme,
        profile_header_rect(row),
        settings.hover_image_gen_profile_header == Some(index),
        ui.button_pressed(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::ImageProfileHeader(index),
        )),
    );
    let dot = profile_active_rect(row);
    if active {
        cx.backend.fill_oval(dot, theme.primary);
        draw_icon(
            cx.backend,
            Icon::Check,
            Point2D::new(dot.origin.x + 3.0, dot.origin.y + 3.0),
            8.0,
            theme.primary_foreground,
            2.0,
        );
    } else {
        cx.backend.stroke_oval(dot, theme.muted_foreground, 1.5);
    }

    let name = if profile.name.trim().is_empty() {
        profile.provider.label()
    } else {
        profile.name.as_str()
    };
    let name = ellipsize(cx, name, row.size.x - 180.0, 12.0);
    let name_lay = TextLayout::single_run(
        &name,
        "system-ui",
        12.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &name_lay,
        Point2D::new(row.origin.x + 32.0, row.origin.y + 20.0),
    );

    let provider = profile.provider.label();
    let provider_w = text_metrics::measure_chrome(cx.backend, provider, 10.0);
    let provider_lay = TextLayout::single_run(
        provider,
        "system-ui",
        10.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &provider_lay,
        Point2D::new(
            row.origin.x + row.size.x - DELETE_W - CHEVRON_W - 12.0 - provider_w,
            row.origin.y + 20.0,
        ),
    );

    let chevron = profile_chevron_rect(row);
    draw_icon(
        cx.backend,
        if editing {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        },
        Point2D::new(chevron.origin.x + 4.0, chevron.origin.y + 6.0),
        12.0,
        theme.muted_foreground,
        1.5,
    );

    let remove_hover = profile_remove_hover_rect(row);
    crate::widgets::button::paint_ghost_button_feedback(
        cx.backend,
        theme,
        remove_hover,
        settings.hover_image_gen_profile_remove == Some(index),
        ui.button_pressed(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::ImageProfileRemove(index),
        )),
    );
    draw_icon(
        cx.backend,
        Icon::Trash,
        Point2D::new(
            remove_hover.origin.x + (remove_hover.size.x - DELETE_ICON) / 2.0,
            remove_hover.origin.y + (remove_hover.size.y - DELETE_ICON) / 2.0,
        ),
        DELETE_ICON,
        theme.muted_foreground,
        1.5,
    );

    if editing {
        for field in image_gen_fields() {
            paint_profile_field(
                cx,
                theme,
                settings,
                ui,
                profile,
                index,
                field,
                profile_input_rect(row, field),
                row.origin.x + 12.0,
                now_ms,
            );
        }
        paint_profile_test_button(
            cx,
            theme,
            ui,
            profile,
            profile_test_btn_rect(row),
            settings.hover_image_gen_profile_test == Some(index),
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::ImageProfileTest(index),
            )),
        );
        let provider_rect = profile_provider_rect(row);
        paint_provider_field(
            cx,
            theme,
            profile,
            provider_rect,
            row.origin.x + 12.0,
            settings.hover_image_gen_profile_provider == Some(index),
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::ImageProfileProvider(index),
            )),
        );
        if settings.image_gen_provider_menu_open == Some(index) {
            let hovered = settings
                .hover_image_gen_provider_option
                .and_then(|(hover_index, provider)| (hover_index == index).then_some(provider));
            let pressed = match ui.pressed_button {
                Some(ButtonPressTarget::AgentSettings(
                    AgentSettingsButton::ImageProviderOption {
                        index: pressed_index,
                        provider,
                    },
                )) if pressed_index == index => Some(provider),
                _ => None,
            };
            paint_provider_menu(cx, theme, provider_rect, profile.provider, hovered, pressed);
        }
    }
}
