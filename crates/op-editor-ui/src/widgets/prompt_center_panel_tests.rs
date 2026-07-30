use op_editor_core::prompt_center_catalog::PromptCategory;
use op_editor_core::{CustomPrompt, EditorState, Locale, PromptFilter};

use super::{PromptCenterHit, PromptCenterPanel, PROMPT_CENTER_PANEL_H, PROMPT_CENTER_PANEL_W};
use crate::{Point2D, Rect};

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
    state.editor_ui.prompt_center.filter = PromptFilter::Category(PromptCategory::Starter);
    let cards = PromptCenterPanel::for_editor(&state)
        .expect("open panel")
        .filtered();
    assert!(!cards.is_empty());
    assert!(cards
        .iter()
        .all(|card| card.category == PromptCategory::Starter));

    state.editor_ui.prompt_center.filter = PromptFilter::Category(PromptCategory::WebPage);
    assert!(PromptCenterPanel::for_editor(&state)
        .expect("open panel")
        .filtered()
        .is_empty());
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
    let rects = PromptCenterPanel::for_editor(&state)
        .expect("open panel")
        .card_rects(panel_rect());
    assert!(rects.len() >= 3);
    assert_eq!(rects[0].1.origin.y, rects[1].1.origin.y);
    assert!(rects[1].1.origin.x > rects[0].1.origin.x);
    assert_eq!(rects[0].1.origin.x, rects[2].1.origin.x);
    assert!(rects[2].1.origin.y > rects[0].1.origin.y);
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
fn max_scroll_tracks_filtered_grid_height() {
    let mut state = open_state(Locale::EnUs);
    let rect = panel_rect();
    assert!(
        PromptCenterPanel::for_editor(&state)
            .expect("open panel")
            .max_scroll(rect)
            > 0.0
    );

    state.editor_ui.prompt_center.filter = PromptFilter::Category(PromptCategory::Starter);
    assert_eq!(
        PromptCenterPanel::for_editor(&state)
            .expect("open panel")
            .max_scroll(rect),
        0.0
    );
}
