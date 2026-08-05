//! Geometry tests for the slides panel: tab row, row stack, thumbnail
//! boxes, scrolling, and the reorder drop arithmetic.

use super::*;
use crate::widgets::deck_boards::BoardChip;
use op_editor_core::LeftPanelTab;

/// Real shipped tab labels, not synthetic strings: whether the row
/// compacts is a fact about the catalogue, and a made-up label would let
/// a test pass while the product never reached the branch. `Layers` /
/// `Slides` is the English pair, `Lớp` / `Trang chiếu` the Vietnamese.
const EN: (&str, &str) = ("Layers", "Slides");
const VI: (&str, &str) = ("Lớp", "Trang chiếu");

const PANEL: Rect = Rect {
    origin: Point2D { x: 0.0, y: 48.0 },
    size: Point2D { x: 240.0, y: 700.0 },
};

/// A uniform 16:9 deck of `count` boards.
fn layout(count: usize, offset: f32) -> SlidesPanelLayout {
    layout_of(&vec![DEFAULT_BOARD_ASPECT; count], offset)
}

fn layout_of(aspects: &[f32], offset: f32) -> SlidesPanelLayout {
    SlidesPanelLayout::new(
        PANEL,
        SlidesPanelTabs::new(PANEL, LeftPanelTab::Slides, EN.0, EN.1),
        aspects,
        offset,
    )
    .expect("a 240x700 rail fits the slides list")
}

#[test]
fn the_tab_row_splits_the_rail_top_in_two() {
    let tabs = SlidesPanelTabs::new(PANEL, LeftPanelTab::Slides, EN.0, EN.1);
    assert_eq!(tabs.row.size.y, SLIDES_TAB_ROW_HEIGHT);
    assert_eq!(tabs.layers.size.x, tabs.slides.size.x);
    assert!(
        (tabs.layers.origin.x + tabs.layers.size.x - tabs.slides.origin.x).abs() < f32::EPSILON,
        "the two tabs abut"
    );
    assert_eq!(
        tabs.hit(Point2D::new(30.0, 60.0)),
        Some(SlidesPanelTarget::LayersTab)
    );
    assert_eq!(
        tabs.hit(Point2D::new(200.0, 60.0)),
        Some(SlidesPanelTarget::SlidesTab)
    );
    // Below the row is nobody's.
    assert_eq!(tabs.hit(Point2D::new(120.0, 200.0)), None);
}

#[test]
fn the_layers_tree_gets_the_rail_below_the_tab_row() {
    let tabs = SlidesPanelTabs::new(PANEL, LeftPanelTab::Slides, EN.0, EN.1);
    let content = tabs.content_rect(PANEL);
    assert_eq!(content.origin.y, PANEL.origin.y + SLIDES_TAB_ROW_HEIGHT);
    assert_eq!(content.size.y, PANEL.size.y - SLIDES_TAB_ROW_HEIGHT);
    assert_eq!(content.size.x, PANEL.size.x);
}

#[test]
fn every_row_is_the_same_height_whatever_shape_its_board_is() {
    // A phone screen, a 16:9 deck board and a square card on one page.
    let mixed = layout_of(&[9.0 / 19.5, DEFAULT_BOARD_ASPECT, 1.0], 0.0);
    let heights: Vec<f32> = (0..3).map(|i| mixed.row_rect(i).size.y).collect();
    assert!(
        heights.windows(2).all(|w| (w[0] - w[1]).abs() < 0.01),
        "rows differ in height: {heights:?}"
    );
    assert!(
        (mixed.row_height() - (THUMB_BOX_H + 4.0 + 16.0)).abs() < 0.01,
        "row height {} is the box plus the name line",
        mixed.row_height()
    );
    // The same three rows on a deck of one uniform aspect.
    let uniform = layout(3, 0.0);
    assert!((uniform.row_height() - mixed.row_height()).abs() < 0.01);
}

#[test]
fn each_board_is_fitted_into_the_row_box_on_its_own_aspect() {
    let mixed = layout_of(&[9.0 / 19.5, DEFAULT_BOARD_ASPECT, 1.0], 0.0);
    // 240 - 10*2 - 20 - 6 = 194 wide box, THUMB_BOX_H tall.
    assert!((mixed.thumb_box.x - 194.0).abs() < 0.01);
    assert!((mixed.thumb_box.y - THUMB_BOX_H).abs() < 0.01);

    for (index, aspect) in [(0, 9.0 / 19.5), (1, DEFAULT_BOARD_ASPECT), (2, 1.0)] {
        let thumb = mixed.thumb_rect(index);
        let boxed = mixed.thumb_box_rect(index);
        assert!(
            (thumb.size.x / thumb.size.y - aspect).abs() < 0.01,
            "row {index} paints at {:?}, not its own {aspect}:1",
            thumb.size
        );
        assert!(
            thumb.size.x <= boxed.size.x + 0.01 && thumb.size.y <= boxed.size.y + 0.01,
            "row {index} overflows its box"
        );
        // Centred: the letterbox / pillarbox bars are equal.
        assert!(
            ((thumb.origin.x - boxed.origin.x)
                - (boxed.origin.x + boxed.size.x - thumb.origin.x - thumb.size.x))
                .abs()
                < 0.01
        );
        assert!(
            ((thumb.origin.y - boxed.origin.y)
                - (boxed.origin.y + boxed.size.y - thumb.origin.y - thumb.size.y))
                .abs()
                < 0.01
        );
    }
    // A tall board is pillarboxed (narrower than the box), a wide one
    // letterboxed (shorter than it).
    assert!(mixed.thumb_rect(0).size.x < mixed.thumb_box.x - 1.0);
    assert!(mixed.thumb_rect(1).size.y < mixed.thumb_box.y - 1.0);
}

#[test]
fn rows_stack_in_page_order_and_the_thumb_sits_in_its_row() {
    let l = layout(5, 0.0);
    let first = l.row_rect(0);
    let second = l.row_rect(1);
    assert!(second.origin.y > first.origin.y, "row 1 is under row 0");
    assert!(
        (second.origin.y - first.origin.y - l.row_height() - 8.0).abs() < 0.01,
        "rows are one stride apart"
    );
    let thumb = l.thumb_rect(2);
    let row = l.row_rect(2);
    assert!(thumb.origin.y >= row.origin.y);
    assert!(
        thumb.origin.x > row.origin.x,
        "the number gutter is left of it"
    );
    assert!(thumb.origin.y + thumb.size.y <= row.origin.y + row.size.y);
}

#[test]
fn a_row_is_clickable_exactly_where_it_paints() {
    let l = layout(5, 0.0);
    let row = l.row_rect(1);
    let inside = Point2D::new(row.origin.x + 40.0, row.origin.y + 10.0);
    assert_eq!(l.row_at(inside), Some(1));
    assert_eq!(l.hit(inside), Some(SlidesPanelTarget::Slide(1)));
    // The gap between rows belongs to no row.
    let gap = Point2D::new(row.origin.x + 40.0, row.origin.y + row.size.y + 3.0);
    assert_eq!(l.row_at(gap), None);
}

#[test]
fn rows_scrolled_past_the_band_are_neither_visible_nor_clickable() {
    let l = layout(20, 0.0);
    let visible: Vec<usize> = l.visible_rows().into_iter().map(|(i, _)| i).collect();
    assert!(visible.contains(&0));
    assert!(
        !visible.contains(&19),
        "a 20-slide deck overflows a 700px rail"
    );
    // The last row's own rect exists off-screen (the drag needs it) but
    // is not hittable.
    let last = l.row_rect(19);
    assert!(last.origin.y > l.list.origin.y + l.list.size.y);
    assert_eq!(
        l.row_at(Point2D::new(100.0, last.origin.y + 5.0)),
        None,
        "outside the band nothing is hit"
    );
}

#[test]
fn scrolling_moves_the_stack_and_clamps_to_the_content() {
    let short = layout(2, 0.0);
    assert_eq!(short.max_scroll(), 0.0, "a deck that fits never scrolls");
    let long = layout(20, 0.0);
    assert!(long.max_scroll() > 0.0);
    let scrolled = layout(20, 10_000.0);
    assert_eq!(
        scrolled.offset,
        long.max_scroll(),
        "an over-scroll clamps to the end"
    );
    assert!(
        scrolled.row_rect(0).origin.y < long.row_rect(0).origin.y,
        "scrolling lifts the stack"
    );
}

#[test]
fn the_footer_holds_the_present_button_under_the_list() {
    let l = layout(3, 0.0);
    assert!(l.footer.origin.y >= l.list.origin.y + l.list.size.y);
    assert_eq!(
        l.footer.origin.y + l.footer.size.y,
        PANEL.origin.y + PANEL.size.y
    );
    let centre = Point2D::new(
        l.present.origin.x + l.present.size.x / 2.0,
        l.present.origin.y + l.present.size.y / 2.0,
    );
    assert_eq!(l.hit(centre), Some(SlidesPanelTarget::Present));
}

#[test]
fn the_drop_slot_flips_at_row_centres() {
    let l = layout(5, 0.0);
    let row1 = l.row_rect(1);
    // Just above row 1's centre still drops before it.
    assert_eq!(l.insertion_slot(row1.origin.y + row1.size.y * 0.25), 1);
    // Past its centre drops after it.
    assert_eq!(l.insertion_slot(row1.origin.y + row1.size.y * 0.75), 2);
    // Far above everything drops at the head, far below at the tail.
    assert_eq!(l.insertion_slot(-10_000.0), 0);
    assert_eq!(l.insertion_slot(10_000.0), 5);
}

#[test]
fn a_drag_only_counts_once_it_has_travelled() {
    let mut drag = SlidesDrag {
        from: 0,
        press_y: 100.0,
        pointer_y: 101.0,
    };
    assert!(!drag_is_live(&drag), "a 1px wobble is still a click");
    drag.pointer_y = 140.0;
    assert!(drag_is_live(&drag));
}

#[test]
fn a_rail_too_short_for_a_row_lays_nothing_out() {
    let squeezed = Rect {
        origin: Point2D::new(0.0, 48.0),
        size: Point2D::new(240.0, SLIDES_TAB_ROW_HEIGHT + FOOTER_HEIGHT),
    };
    assert!(SlidesPanelLayout::new(
        squeezed,
        SlidesPanelTabs::new(squeezed, LeftPanelTab::Slides, EN.0, EN.1),
        &[DEFAULT_BOARD_ASPECT; 3],
        0.0
    )
    .is_none());
    let narrow = Rect {
        origin: Point2D::new(0.0, 48.0),
        size: Point2D::new(30.0, 700.0),
    };
    assert!(SlidesPanelLayout::new(
        narrow,
        SlidesPanelTabs::new(narrow, LeftPanelTab::Slides, EN.0, EN.1),
        &[DEFAULT_BOARD_ASPECT; 3],
        0.0
    )
    .is_none());
}

#[test]
fn a_non_finite_aspect_falls_back_to_sixteen_by_nine() {
    let l = layout_of(&[f32::NAN, 0.0, -2.0], 0.0);
    for index in 0..3 {
        let thumb = l.thumb_rect(index);
        assert!(
            (thumb.size.x / thumb.size.y - DEFAULT_BOARD_ASPECT).abs() < 0.01,
            "row {index} falls back to 16:9"
        );
    }
}

/// The rect the widget paints its placeholder into IS the rect the host
/// blits its rendered board into. If these two ever disagree a thumbnail
/// lands off its own row, so the contract is asserted rather than
/// assumed.
#[test]
fn the_placeholder_paints_exactly_where_the_host_blits() {
    use crate::widgets::test_capture_backend::CaptureBackend;
    let chips: Vec<BoardChip> = (0..3)
        .map(|i| BoardChip {
            id: format!("slide-{i}"),
            name: format!("Slide {i}"),
        })
        .collect();
    let l = layout(3, 0.0);
    let panel = SlidesPanel {
        chips: &chips,
        active: Some(1),
        hover: None,
        drag: None,
        thumbnails_supported: true,
        layers_label: "Layers",
        slides_label: "Slides",
        present_label: "Present",
    };
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    panel.paint(&mut cx, &l, &Theme::dark());

    for index in 0..3 {
        let expected = l.thumb_rect(index);
        assert!(
            backend
                .round_fills
                .iter()
                .any(
                    |(rect, _, _)| (rect.origin.x - expected.origin.x).abs() < 0.01
                        && (rect.origin.y - expected.origin.y).abs() < 0.01
                        && (rect.size.x - expected.size.x).abs() < 0.01
                        && (rect.size.y - expected.size.y).abs() < 0.01
                ),
            "row {index}'s placeholder is painted at its thumb_rect {expected:?}"
        );
    }
    // And the footer button, so the press target is a painted button.
    assert!(
        backend
            .round_fills
            .iter()
            .any(
                |(rect, _, _)| (rect.origin.x - l.present.origin.x).abs() < 0.01
                    && (rect.origin.y - l.present.origin.y).abs() < 0.01
            ),
        "the present button paints where it hit-tests"
    );
}

#[test]
fn a_thumbnail_blit_is_clipped_to_the_list_band() {
    let unscrolled = layout(20, 0.0);
    // A fully visible row's clip IS its picture.
    let first = unscrolled
        .visible_thumb_rect(0)
        .expect("row 0 is on screen");
    assert_eq!(first, unscrolled.thumb_rect(0));

    // Scroll until one row's picture is cut in half by the footer edge.
    // It must be clipped there, never past it — an unclipped blit would
    // paint over the present button.
    let straddling = 4;
    let band_bottom = unscrolled.list.origin.y + unscrolled.list.size.y;
    let probe = unscrolled.thumb_rect(straddling);
    let l = layout(20, probe.origin.y + probe.size.y / 2.0 - band_bottom);
    assert!(
        l.thumb_rect(straddling).origin.y < band_bottom
            && l.thumb_rect(straddling).origin.y + l.thumb_rect(straddling).size.y > band_bottom,
        "row {straddling} was meant to straddle the band edge"
    );
    let clipped = l
        .visible_thumb_rect(straddling)
        .expect("part of it is visible");
    assert_eq!(clipped.origin.y + clipped.size.y, band_bottom);
    assert!(clipped.size.y < l.thumb_rect(straddling).size.y);
    // A row entirely below the band has nothing to blit.
    let below = (0..20)
        .find(|i| l.thumb_rect(*i).origin.y >= band_bottom)
        .expect("some row is fully below");
    assert_eq!(l.visible_thumb_rect(below), None);
}

// ---- The tab row's text / icon modes -----------------------------------

fn rail(width: f32) -> Rect {
    Rect {
        origin: PANEL.origin,
        size: Point2D::new(width, PANEL.size.y),
    }
}

fn tabs_of(width: f32, active: LeftPanelTab, labels: (&str, &str)) -> SlidesPanelTabs {
    SlidesPanelTabs::new(rail(width), active, labels.0, labels.1)
}

#[test]
fn a_rail_with_room_for_its_labels_keeps_them() {
    let tabs = tabs_of(240.0, LeftPanelTab::Slides, EN);
    assert!(!tabs.compact, "English at 240 has room for both words");
    assert_eq!(
        tabs.layers.size.x, tabs.slides.size.x,
        "text mode splits the row in equal halves"
    );
    assert!(
        (tabs.layers.origin.x + tabs.layers.size.x - tabs.slides.origin.x).abs() < f32::EPSILON,
        "the two tabs abut"
    );
    // The mode change must not cost the row its click targets.
    assert_eq!(
        tabs.hit(Point2D::new(30.0, 60.0)),
        Some(SlidesPanelTarget::LayersTab)
    );
    assert_eq!(
        tabs.hit(Point2D::new(200.0, 60.0)),
        Some(SlidesPanelTarget::SlidesTab)
    );
}

#[test]
fn a_rail_too_narrow_for_its_labels_falls_back_to_icons() {
    let tabs = tabs_of(180.0, LeftPanelTab::Slides, VI);
    assert!(
        tabs.compact,
        "Vietnamese labels do not fit two ways across a 180 px rail"
    );
    // The active tab keeps `[icon label]`; the other shrinks to its glyph.
    assert!(
        tabs.slides.size.x > tabs.layers.size.x,
        "the active tab is the one that keeps its label: {:?} vs {:?}",
        tabs.slides.size,
        tabs.layers.size
    );
    let inner_w = 180.0 - 8.0 * 2.0;
    assert!(
        tabs.layers.size.x + tabs.slides.size.x <= inner_w + 0.01,
        "the tabs must not overhang the row"
    );
    assert!(
        (tabs.layers.origin.x + tabs.layers.size.x - tabs.slides.origin.x).abs() < 0.01,
        "they still abut, so no dead gap eats clicks between them"
    );
    // Both are still clickable, exactly where they paint.
    let mid_y = tabs.row.origin.y + tabs.row.size.y / 2.0;
    assert_eq!(
        tabs.hit(Point2D::new(tabs.layers.origin.x + 2.0, mid_y)),
        Some(SlidesPanelTarget::LayersTab)
    );
    assert_eq!(
        tabs.hit(Point2D::new(tabs.slides.origin.x + 2.0, mid_y)),
        Some(SlidesPanelTarget::SlidesTab)
    );
}

#[test]
fn the_labelled_pill_follows_whichever_tab_is_active() {
    let on_slides = tabs_of(180.0, LeftPanelTab::Slides, VI);
    let on_layers = tabs_of(180.0, LeftPanelTab::Layers, VI);
    assert!(on_slides.compact && on_layers.compact);
    assert!(on_slides.slides.size.x > on_slides.layers.size.x);
    assert!(on_layers.layers.size.x > on_layers.slides.size.x);
    // The un-labelled tab is the same square either way — it holds one
    // glyph and nothing else, whichever side it happens to be on.
    assert!((on_slides.layers.size.x - on_layers.slides.size.x).abs() < 0.01);
}

/// Dragging the rail switches modes at the measured width, and never
/// switches back the wrong way: once the labels stop fitting they stay
/// not-fitting as the rail keeps narrowing.
#[test]
fn resizing_the_rail_switches_modes_monotonically() {
    let compact_at = |w: f32| tabs_of(w, LeftPanelTab::Slides, VI).compact;
    assert!(compact_at(180.0), "at the minimum rail width, icons");
    assert!(!compact_at(480.0), "at the maximum, words");
    let mut seen_text = false;
    for step in 0..=60 {
        let width = 180.0 + step as f32 * 5.0;
        let compact = compact_at(width);
        if !compact {
            seen_text = true;
        }
        assert!(
            !(compact && seen_text),
            "the row went back to icons at {width} after already fitting its labels"
        );
    }
    assert!(seen_text);
}

/// English is short enough that two tabs fit at every rail width we
/// allow, so the fallback is inert for it today — that is the measured
/// answer, not an oversight, and it is what stops us shrinking a row
/// that has room. The moment a third tab lands the same rule compacts
/// the row without another line of code.
#[test]
fn the_fit_rule_scales_to_more_tabs_than_the_two_we_ship() {
    let inner_w = 240.0 - 8.0 * 2.0;
    let slides_w = crate::widgets::top_bar_geometry::estimated_text_width("Slides", 12.0);
    assert!(
        text_tabs_fit(inner_w, 2, slides_w),
        "two English tabs fit the default rail"
    );
    assert!(
        !text_tabs_fit(inner_w, 5, slides_w),
        "five would not — the row compacts on tab count, not only on rail width"
    );
    // And the two we ship never compact anywhere in the resize range.
    for step in 0..=60 {
        let width = 180.0 + step as f32 * 5.0;
        assert!(
            !tabs_of(width, LeftPanelTab::Slides, EN).compact,
            "English compacted at {width}, which means the estimate drifted"
        );
    }
}
