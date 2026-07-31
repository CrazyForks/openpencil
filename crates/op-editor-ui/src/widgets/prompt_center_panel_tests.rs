use op_editor_core::prompt_center_catalog::PromptCategory;
use op_editor_core::{CustomPrompt, EditorState, Locale, PromptFilter};

use super::{PromptCenterHit, PromptCenterPanel, PROMPT_CENTER_PANEL_H, PROMPT_CENTER_PANEL_W};
use crate::widgets::canvas_viewport_image::{
    lock_decode_registry_for_tests, mark_decode_done, take_pending_decodes,
};
use crate::widgets::property_panel_test_support::CountingBackend;
use crate::widgets::PaintCx;
use crate::{ImageDrawMode, Point2D, Rect};

fn open_state(locale: Locale) -> EditorState {
    let mut state = EditorState::new();
    state.editor_ui.locale = locale;
    state.editor_ui.open_prompt_center(1);
    state
}

fn panel_rect() -> Rect {
    Rect::xywh(30.0, 40.0, PROMPT_CENTER_PANEL_W, PROMPT_CENTER_PANEL_H)
}

fn filtered_ids(state: &EditorState) -> Vec<String> {
    PromptCenterPanel::for_editor(state)
        .expect("open panel")
        .filtered()
        .into_iter()
        .map(|card| card.id.into_owned())
        .collect()
}

fn visible_card_count(panel: &PromptCenterPanel<'_>, rect: Rect) -> usize {
    let viewport = panel.cards_viewport(rect);
    let bottom = viewport.origin.y + viewport.size.y;
    panel
        .card_rects(rect)
        .into_iter()
        .filter(|(_, card)| {
            card.origin.y + card.size.y > viewport.origin.y && card.origin.y < bottom
        })
        .count()
}

#[test]
fn travel_search_matches_both_chinese_and_english() {
    let mut state = open_state(Locale::ZhCn);
    state.editor_ui.prompt_center.search.set_text("旅行");
    assert!(filtered_ids(&state).iter().any(|id| id == "gallery-wander"));

    state.editor_ui.prompt_center.search.set_text("travel");
    assert!(filtered_ids(&state).iter().any(|id| id == "gallery-wander"));
}

#[test]
fn category_filter_only_returns_that_category() {
    let mut state = open_state(Locale::EnUs);
    let expected = [
        (PromptCategory::Starter, None),
        (PromptCategory::WebPage, Some(["web-orbit", "web-atelier"])),
        (
            PromptCategory::Dashboard,
            Some(["dashboard-pulse", "dashboard-sentinel"]),
        ),
        (
            PromptCategory::Component,
            Some(["component-data-grid", "component-form-lab"]),
        ),
        (
            PromptCategory::Modify,
            Some(["modify-polish-current", "modify-complete-states"]),
        ),
    ];

    for (category, expected_ids) in expected {
        state.editor_ui.prompt_center.filter = PromptFilter::Category(category);
        let cards = PromptCenterPanel::for_editor(&state)
            .expect("open panel")
            .filtered();
        assert!(
            !cards.is_empty(),
            "{category:?} should have built-in prompts"
        );
        assert!(
            cards.iter().all(|card| card.category == category),
            "{category:?} filter leaked another category"
        );
        if let Some(expected_ids) = expected_ids {
            assert_eq!(
                cards
                    .iter()
                    .map(|card| card.id.as_ref())
                    .collect::<Vec<_>>(),
                expected_ids,
                "{category:?} prompt order changed"
            );
        }
    }
}

#[test]
fn unmatched_query_produces_empty_result() {
    let mut state = open_state(Locale::EnUs);
    state
        .editor_ui
        .prompt_center
        .search
        .set_text("no-such-prompt-9d79e9");
    assert!(PromptCenterPanel::for_editor(&state)
        .expect("open panel")
        .filtered()
        .is_empty());
}

#[test]
fn grid_places_cards_in_two_columns() {
    let state = open_state(Locale::EnUs);
    let panel = PromptCenterPanel::for_editor(&state).expect("open panel");
    let rects = panel.card_rects(panel_rect());
    assert!(rects.len() >= 3);
    assert_eq!(rects[0].1.origin.y, rects[1].1.origin.y);
    assert!(rects[1].1.origin.x > rects[0].1.origin.x);
    assert_eq!(rects[0].1.origin.x, rects[2].1.origin.x);
    assert!(rects[2].1.origin.y > rects[0].1.origin.y);
    assert_eq!(rects[0].1.size.y, 262.0);

    let preview = PromptCenterPanel::card_preview_rect(rects[0].1);
    assert!(rects[0].1.contains(preview.origin));
    assert!(
        (preview.size.x / preview.size.y - 16.0 / 10.0).abs() < 0.001,
        "preview must remain 16:10"
    );
    assert!(preview.origin.y + preview.size.y < rects[0].1.origin.y + rects[0].1.size.y);
}

#[test]
fn body_language_follows_cjk_locale_boundary() {
    let zh = open_state(Locale::ZhCn);
    let zh_body = PromptCenterPanel::for_editor(&zh)
        .expect("open panel")
        .filtered()
        .into_iter()
        .find(|card| card.id == "gallery-wander")
        .expect("wander")
        .body
        .to_owned();
    assert!(zh_body.contains("旅行"));

    let en = open_state(Locale::EnUs);
    let en_body = PromptCenterPanel::for_editor(&en)
        .expect("open panel")
        .filtered()
        .into_iter()
        .find(|card| card.id == "gallery-wander")
        .expect("wander")
        .body
        .to_owned();
    assert!(en_body.contains("travel itinerary"));
    assert!(!en_body.contains("旅行"));

    let ja = open_state(Locale::Ja);
    let ja_body = PromptCenterPanel::for_editor(&ja)
        .expect("open panel")
        .filtered()
        .into_iter()
        .find(|card| card.id == "gallery-wander")
        .expect("wander")
        .body;
    assert!(ja_body.contains("旅行"));
}

#[test]
fn outside_inside_and_card_hits_are_distinct() {
    let state = open_state(Locale::ZhCn);
    let panel = PromptCenterPanel::for_editor(&state).expect("open panel");
    let rect = panel_rect();
    assert_eq!(
        panel.hit_test(rect, Point2D::new(rect.origin.x - 1.0, rect.origin.y)),
        None
    );
    assert_eq!(
        panel.hit_test(
            rect,
            Point2D::new(rect.origin.x + rect.size.x / 2.0, rect.origin.y + 10.0)
        ),
        Some(PromptCenterHit::Inside)
    );

    let first = panel.card_rects(rect)[0].1;
    let hit = panel
        .hit_test(
            rect,
            Point2D::new(first.origin.x + 12.0, first.origin.y + 12.0),
        )
        .expect("card hit");
    match hit {
        PromptCenterHit::SelectPrompt { id, body } => {
            assert_eq!(id, "gallery-wander");
            assert!(body.contains("旅行"));
        }
        other => panic!("unexpected hit: {other:?}"),
    }
}

#[test]
fn scrolled_grid_hits_the_card_visible_at_the_viewport_top() {
    let mut state = open_state(Locale::EnUs);
    let rect = panel_rect();
    let row_step = {
        let panel = PromptCenterPanel::for_editor(&state).expect("open panel");
        let cards = panel.card_rects(rect);
        cards[2].1.origin.y - cards[0].1.origin.y
    };
    state.editor_ui.prompt_center.scroll.offset = row_step;

    let panel = PromptCenterPanel::for_editor(&state).expect("open panel");
    let cards = panel.card_rects(rect);
    let viewport = panel.cards_viewport(rect);
    assert_eq!(cards[2].1.origin.y, viewport.origin.y);
    let expected_id = panel.filtered()[2].id.to_string();
    assert!(matches!(
        panel.hit_test(
            rect,
            Point2D::new(cards[2].1.origin.x + 12.0, cards[2].1.origin.y + 12.0),
        ),
        Some(PromptCenterHit::SelectPrompt { id, .. }) if id == expected_id
    ));
}

#[test]
fn custom_delete_has_its_own_hit_target() {
    let mut state = open_state(Locale::EnUs);
    state.editor_ui.prompt_center.install_custom_prompts(
        vec![CustomPrompt {
            id: "custom-1".to_owned(),
            title: "Reusable".to_owned(),
            body: "Reusable prompt body".to_owned(),
            category: PromptCategory::Modify,
            created_at: 1,
        }],
        true,
    );
    state.editor_ui.prompt_center.filter = PromptFilter::Custom;
    let panel = PromptCenterPanel::for_editor(&state).expect("open panel");
    let rect = panel_rect();
    let card = panel.card_rects(rect)[0].1;
    let delete = PromptCenterPanel::delete_rect(card);
    assert_eq!(
        panel.hit_test(
            rect,
            Point2D::new(
                delete.origin.x + delete.size.x / 2.0,
                delete.origin.y + delete.size.y / 2.0,
            ),
        ),
        Some(PromptCenterHit::DeleteCustom("custom-1".to_owned()))
    );
}

#[test]
fn read_only_custom_card_does_not_expose_delete() {
    let mut state = open_state(Locale::EnUs);
    state.editor_ui.prompt_center.install_custom_prompts(
        vec![CustomPrompt {
            id: "custom-1".to_owned(),
            title: "Reusable".to_owned(),
            body: "Reusable prompt body".to_owned(),
            category: PromptCategory::Modify,
            created_at: 1,
        }],
        false,
    );
    state.editor_ui.prompt_center.filter = PromptFilter::Custom;
    let panel = PromptCenterPanel::for_editor(&state).expect("open panel");
    let rect = panel_rect();
    let card = panel.card_rects(rect)[0].1;
    let delete = PromptCenterPanel::delete_rect(card);
    assert!(matches!(
        panel.hit_test(
            rect,
            Point2D::new(
                delete.origin.x + delete.size.x / 2.0,
                delete.origin.y + delete.size.y / 2.0,
            ),
        ),
        Some(PromptCenterHit::SelectPrompt { .. })
    ));
}

#[test]
fn clipped_custom_delete_does_not_steal_hover_from_panel_chrome() {
    let mut state = open_state(Locale::EnUs);
    state.editor_ui.prompt_center.install_custom_prompts(
        (0..4)
            .map(|index| CustomPrompt {
                id: format!("custom-{index}"),
                title: format!("Reusable {index}"),
                body: "Reusable prompt body".to_owned(),
                category: PromptCategory::Modify,
                created_at: index,
            })
            .collect(),
        true,
    );
    state.editor_ui.prompt_center.filter = PromptFilter::Custom;
    let rect = panel_rect();
    let panel = PromptCenterPanel::for_editor(&state).expect("open panel");
    let delete = PromptCenterPanel::delete_rect(panel.card_rects(rect)[0].1);
    let search = PromptCenterPanel::search_rect(rect);
    state.editor_ui.prompt_center.scroll.offset =
        delete.origin.y + delete.size.y / 2.0 - (search.origin.y + search.size.y / 2.0);

    let panel = PromptCenterPanel::for_editor(&state).expect("open panel");
    let hidden_delete = PromptCenterPanel::delete_rect(panel.card_rects(rect)[0].1);
    let point = Point2D::new(
        hidden_delete.origin.x + hidden_delete.size.x / 2.0,
        hidden_delete.origin.y + hidden_delete.size.y / 2.0,
    );
    assert!(search.contains(point));
    assert_eq!(panel.hover_at(rect, point), None);
}

#[test]
fn max_scroll_tracks_filtered_grid_height() {
    let mut state = open_state(Locale::EnUs);
    let rect = panel_rect();
    let panel = PromptCenterPanel::for_editor(&state).expect("open panel");
    let viewport = panel.cards_viewport(rect);
    let last = panel.card_rects(rect).last().expect("catalogue card").1;
    assert_eq!(
        panel.max_scroll(rect),
        last.origin.y + last.size.y - (viewport.origin.y + viewport.size.y)
    );

    state.editor_ui.prompt_center.filter = PromptFilter::Category(PromptCategory::Starter);
    assert!(
        PromptCenterPanel::for_editor(&state)
            .expect("open panel")
            .max_scroll(rect)
            > 0.0,
        "two thumbnail rows must overflow the card viewport"
    );

    state.editor_ui.prompt_center.install_custom_prompts(
        vec![CustomPrompt {
            id: "custom-only".to_owned(),
            title: "Reusable".to_owned(),
            body: "Reusable prompt body".to_owned(),
            category: PromptCategory::Modify,
            created_at: 1,
        }],
        true,
    );
    state.editor_ui.prompt_center.filter = PromptFilter::Custom;
    assert_eq!(
        PromptCenterPanel::for_editor(&state)
            .expect("open panel")
            .max_scroll(rect),
        0.0,
        "a single thumbnail card must not scroll"
    );
}

#[test]
fn paint_decodes_only_generated_previews_visible_in_the_viewport() {
    let state = open_state(Locale::EnUs);
    let panel = PromptCenterPanel::for_editor(&state).expect("open panel");
    let expected = visible_card_count(&panel, panel_rect());
    let mut backend = CountingBackend::default();
    panel.paint(
        &mut PaintCx {
            backend: &mut backend,
        },
        panel_rect(),
    );

    assert_eq!(backend.images.len(), expected);
    assert_eq!(backend.image_modes.len(), backend.images.len());
    assert!(backend
        .image_modes
        .iter()
        .all(|mode| *mode == ImageDrawMode::Fill));
}

#[test]
fn first_paint_queues_only_visible_previews_and_uses_fallbacks() {
    let _guard = lock_decode_registry_for_tests();
    let state = open_state(Locale::EnUs);
    let panel = PromptCenterPanel::for_editor(&state).expect("open panel");
    let expected = visible_card_count(&panel, panel_rect());
    let mut backend = CountingBackend {
        image_decode_ready: Some(false),
        image_resident_ready: Some(false),
        ..Default::default()
    };
    panel.paint(
        &mut PaintCx {
            backend: &mut backend,
        },
        panel_rect(),
    );

    assert!(backend.images.is_empty());
    assert!(backend.linear_gradients >= 2);
    let pending = take_pending_decodes(usize::MAX);
    assert_eq!(pending.len(), expected);
    for entry in pending {
        mark_decode_done(entry.id);
    }
}

#[test]
fn starter_quick_actions_use_generated_previews() {
    let mut state = open_state(Locale::EnUs);
    state.editor_ui.prompt_center.filter = PromptFilter::Category(PromptCategory::Starter);
    let panel = PromptCenterPanel::for_editor(&state).expect("open panel");
    let mut backend = CountingBackend::default();
    panel.paint(
        &mut PaintCx {
            backend: &mut backend,
        },
        panel_rect(),
    );

    assert_eq!(backend.images.len(), 4);
    assert_eq!(backend.image_decode_edges.len(), 4);
    assert_eq!(backend.image_modes.len(), 4);
    assert!(backend
        .image_modes
        .iter()
        .all(|mode| *mode == ImageDrawMode::Fill));
    assert_eq!(backend.linear_gradients, 0);
}
