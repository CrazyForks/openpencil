use super::*;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::EditorState;

use super::test_rects::{MEDIUM as PANEL, NARROW, WIDE};

fn open_state() -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.open_scene_template_center(0);
    state
}

#[test]
fn the_panel_only_builds_while_open() {
    let mut state = EditorState::default();
    assert!(SceneTemplatePanel::for_editor(&state).is_none());
    state.editor_ui.open_scene_template_center(0);
    assert!(SceneTemplatePanel::for_editor(&state).is_some());
}

#[test]
fn every_scene_chip_is_reachable_and_none_overlap() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let chips = panel.filter_chip_rects(PANEL);
    assert_eq!(chips.len(), TemplateScene::ALL.len() + 1, "scenes plus All");

    // Chips are laid out by accumulating widths; an off-by-one there would
    // silently make the last chip unclickable or overlap its neighbour.
    for pair in chips.windows(2) {
        let (left, _) = pair[0];
        let (right, _) = pair[1];
        assert!(
            left.origin.x + left.size.x <= right.origin.x,
            "chips overlap: {left:?} then {right:?}"
        );
    }
    let (last, _) = chips.last().copied().expect("chips");
    assert!(
        last.origin.x + last.size.x <= PANEL.origin.x + PANEL.size.x,
        "the last chip runs past the panel edge"
    );
    for (rect, filter) in chips {
        assert_eq!(
            panel.hit_test(
                PANEL,
                Point2D::new(rect.origin.x + 2.0, rect.origin.y + 2.0)
            ),
            Some(SceneTemplateHit::SelectFilter(filter))
        );
    }
}

/// The filter row is one line with no wrap or scroll, so it has to fit in
/// every locale — and the locale with the longest labels is not the default
/// one the test above runs in. Adding a scene adds a chip, which is exactly
/// when the row runs off the panel edge for languages that spell the scene
/// names out.
#[test]
fn the_filter_row_fits_the_panel_in_every_locale() {
    for locale in op_editor_core::Locale::ALL {
        let mut state = open_state();
        state.editor_ui.locale = locale;
        let panel = SceneTemplatePanel::for_editor(&state).expect("open");
        let chips = panel.filter_chip_rects(PANEL);
        let (last, _) = chips.last().copied().expect("chips");
        let overflow = last.origin.x + last.size.x - (PANEL.origin.x + PANEL.size.x - PAD);
        assert!(
            overflow <= 0.0,
            "{locale:?}: the filter row overflows the panel by {overflow:.1}px"
        );
    }
}

#[test]
fn cards_fill_their_row_and_wrap_inside_the_viewport() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let viewport = panel.cards_viewport(PANEL);
    let (columns, _, _) = panel.grid_metrics(PANEL);
    let rects = panel.card_rects(PANEL);
    assert!(
        rects.len() > columns,
        "the catalogue ships more than one row of templates"
    );

    let (_, first) = rects[0];
    let (_, second) = rects[1];
    assert_eq!(first.origin.y, second.origin.y, "first two share a row");
    assert!(second.origin.x > first.origin.x);
    assert!(
        rects[columns].1.origin.y > first.origin.y,
        "the card past the last column wraps to the next row"
    );

    for (_, rect) in &rects {
        assert!(rect.origin.x >= viewport.origin.x - 0.5);
        assert!(
            rect.origin.x + rect.size.x <= viewport.origin.x + viewport.size.x + 0.5,
            "card {rect:?} runs past the viewport"
        );
    }
}

/// The gallery has no fixed column count: a wider window buys more cards per
/// row, and a narrow one falls back rather than shrinking cards to slivers.
#[test]
fn the_column_count_follows_the_panel_width() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let columns_at = |rect| panel.grid_metrics(rect).0;
    assert_eq!(columns_at(NARROW), 2);
    assert_eq!(columns_at(PANEL), 3);
    assert_eq!(columns_at(WIDE), 4);
}

/// The whole point of going full-screen: previews get bigger, not just more
/// numerous. A card must also be exactly as tall as its own preview plus the
/// fixed text block, or the derived height and the painted preview disagree.
#[test]
fn card_height_is_derived_from_the_preview_it_holds() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    for rect in [NARROW, PANEL, WIDE] {
        let (_, card_w, card_h) = panel.grid_metrics(rect);
        assert!(
            card_w - CARD_PREVIEW_INSET * 2.0 > 320.0,
            "a {card_w}px card leaves a preview no bigger than the old dialog's"
        );
        assert!(
            (card_h - (CARD_PREVIEW_INSET + preview_height(card_w) + CARD_TEXT_H)).abs() < 0.01,
            "card height must leave the text block exactly {CARD_TEXT_H}px"
        );
    }
}

/// Everything inside the gallery reads off one centred column, so a 32"
/// display gets a bigger gallery rather than a search field a metre wide.
#[test]
fn the_content_column_is_capped_and_centred() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let content = SceneTemplatePanel::content_rect(WIDE);
    assert_eq!(content.size.x, SCENE_TEMPLATE_CONTENT_MAX_W);
    let left = content.origin.x - WIDE.origin.x;
    let right = (WIDE.origin.x + WIDE.size.x) - (content.origin.x + content.size.x);
    assert!((left - right).abs() < 0.5, "margins {left} vs {right}");

    // Every row hangs off the same column, so none of them can drift out of
    // alignment with the grid as the cap changes.
    assert_eq!(
        SceneTemplatePanel::search_rect(WIDE).origin.x,
        content.origin.x
    );
    assert_eq!(panel.cards_viewport(WIDE).origin.x, content.origin.x);
    assert_eq!(panel.tab_chip_rects(WIDE)[0].0.origin.x, content.origin.x);
    assert_eq!(
        panel.filter_chip_rects(WIDE)[0].0.origin.x,
        content.origin.x
    );
}

#[test]
fn a_press_on_a_card_resolves_to_that_template() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let templates = panel.filtered();
    let (index, rect) = panel.card_rects(PANEL).into_iter().next().expect("a card");
    let hit = panel.hit_test(
        PANEL,
        Point2D::new(rect.origin.x + 10.0, rect.origin.y + 10.0),
    );
    assert_eq!(
        hit,
        Some(SceneTemplateHit::AddTemplateToCanvas(
            templates[index].id.clone()
        ))
    );
}

#[test]
fn search_narrows_the_grid_and_an_empty_result_is_representable() {
    let mut state = open_state();
    state.editor_ui.scene_template_center.search.set_text("PPT");
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let hits = panel.filtered();
    assert!(!hits.is_empty());
    assert!(hits.iter().all(|t| t.scene == TemplateScene::Slides));

    state
        .editor_ui
        .scene_template_center
        .search
        .set_text("no-such-template-anywhere");
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    assert!(panel.filtered().is_empty());
    assert_eq!(panel.max_scroll(PANEL), 0.0, "an empty grid cannot scroll");
}

#[test]
fn a_pointer_below_the_viewport_does_not_hover_a_scrolled_card() {
    // Card rects are computed for every entry, including ones scrolled out of
    // view — paint clips them. Without the viewport gate, a pointer under the
    // panel would light up a row nobody can see.
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let viewport = panel.cards_viewport(PANEL);
    let below = Point2D::new(
        viewport.origin.x + 10.0,
        viewport.origin.y + viewport.size.y + 4.0,
    );
    assert_eq!(panel.hover_at(PANEL, below), None);
}

#[test]
fn the_close_button_sits_inside_the_header_and_hit_tests_first() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let close = SceneTemplatePanel::close_rect(PANEL);
    assert!(close.origin.y >= PANEL.origin.y);
    assert!(close.origin.x + close.size.x <= PANEL.origin.x + PANEL.size.x);
    let centre = Point2D::new(
        close.origin.x + close.size.x / 2.0,
        close.origin.y + close.size.y / 2.0,
    );
    assert_eq!(panel.hit_test(PANEL, centre), Some(SceneTemplateHit::Close));
    assert_eq!(
        panel.hover_at(PANEL, centre),
        Some(SCENE_TEMPLATE_CLOSE_HOVER)
    );
}

/// The widget answers only for its own rect. What happens to a press on the
/// scrim beside it is the press flow's call, not this layer's.
#[test]
fn presses_outside_the_panel_are_not_claimed() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let outside = Point2D::new(PANEL.origin.x - 1.0, PANEL.origin.y + 10.0);
    assert_eq!(panel.hit_test(PANEL, outside), None);
    assert_eq!(panel.hover_at(PANEL, outside), None);
}
