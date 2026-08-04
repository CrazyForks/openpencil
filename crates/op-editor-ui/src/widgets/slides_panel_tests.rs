//! Geometry tests for the slides panel: tab row, row stack, thumbnail
//! boxes, scrolling, and the reorder drop arithmetic.

use super::*;
use crate::widgets::deck_filmstrip::FilmstripChip;

const PANEL: Rect = Rect {
    origin: Point2D { x: 0.0, y: 48.0 },
    size: Point2D { x: 240.0, y: 700.0 },
};

fn layout(count: usize, offset: f32) -> SlidesPanelLayout {
    SlidesPanelLayout::new(PANEL, count, DEFAULT_BOARD_ASPECT, offset)
        .expect("a 240x700 rail fits the slides list")
}

#[test]
fn the_tab_row_splits_the_rail_top_in_two() {
    let tabs = SlidesPanelTabs::new(PANEL);
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
    let tabs = SlidesPanelTabs::new(PANEL);
    let content = tabs.content_rect(PANEL);
    assert_eq!(content.origin.y, PANEL.origin.y + SLIDES_TAB_ROW_HEIGHT);
    assert_eq!(content.size.y, PANEL.size.y - SLIDES_TAB_ROW_HEIGHT);
    assert_eq!(content.size.x, PANEL.size.x);
}

#[test]
fn thumbnails_take_the_board_aspect_at_the_rail_width() {
    let l = layout(4, 0.0);
    // 240 - 10*2 - 20 - 6 = 194 wide, 16:9 -> ~109 tall.
    assert!(
        (l.thumb.x - 194.0).abs() < 0.01,
        "thumb width {}",
        l.thumb.x
    );
    assert!(
        (l.thumb.y - 194.0 / DEFAULT_BOARD_ASPECT).abs() < 0.01,
        "thumb height {}",
        l.thumb.y
    );
    // A 3:4 card deck gets tall thumbnails at the same width.
    let card = SlidesPanelLayout::new(PANEL, 4, 0.75, 0.0).expect("card layout");
    assert!(card.thumb.y > card.thumb.x, "a 3:4 card is portrait");
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
    assert!(SlidesPanelLayout::new(squeezed, 3, DEFAULT_BOARD_ASPECT, 0.0).is_none());
    let narrow = Rect {
        origin: Point2D::new(0.0, 48.0),
        size: Point2D::new(30.0, 700.0),
    };
    assert!(SlidesPanelLayout::new(narrow, 3, DEFAULT_BOARD_ASPECT, 0.0).is_none());
}

#[test]
fn a_non_finite_aspect_falls_back_to_sixteen_by_nine() {
    let l = SlidesPanelLayout::new(PANEL, 3, f32::NAN, 0.0).expect("layout");
    assert!((l.thumb.x / l.thumb.y - DEFAULT_BOARD_ASPECT).abs() < 0.01);
}

/// The rect the widget paints its placeholder into IS the rect the host
/// blits its rendered board into. If these two ever disagree a thumbnail
/// lands off its own row, so the contract is asserted rather than
/// assumed.
#[test]
fn the_placeholder_paints_exactly_where_the_host_blits() {
    use crate::widgets::test_capture_backend::CaptureBackend;
    let chips: Vec<FilmstripChip> = (0..3)
        .map(|i| FilmstripChip {
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
    let l = layout(20, 0.0);
    // A fully visible row's clip IS its box.
    let first = l.visible_thumb_rect(0).expect("row 0 is on screen");
    assert_eq!(first, l.thumb_rect(0));
    // The row straddling the footer edge is cut at the band, never past
    // it — an unclipped blit would paint over the present button.
    let band_bottom = l.list.origin.y + l.list.size.y;
    let straddling = (0..20)
        .find(|i| {
            let t = l.thumb_rect(*i);
            t.origin.y < band_bottom && t.origin.y + t.size.y > band_bottom
        })
        .expect("some row straddles the band edge");
    let clipped = l
        .visible_thumb_rect(straddling)
        .expect("part of it is visible");
    assert_eq!(clipped.origin.y + clipped.size.y, band_bottom);
    assert!(clipped.size.y < l.thumb.y);
    // A row entirely below the band has nothing to blit.
    let below = (0..20)
        .find(|i| l.thumb_rect(*i).origin.y >= band_bottom)
        .expect("some row is fully below");
    assert_eq!(l.visible_thumb_rect(below), None);
}
