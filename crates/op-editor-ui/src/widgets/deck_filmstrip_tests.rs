use super::*;

/// A generous canvas region — wide enough for six chips end to end.
fn canvas() -> Rect {
    Rect {
        origin: Point2D::new(240.0, 48.0),
        size: Point2D::new(1000.0, 700.0),
    }
}

fn layout(count: usize, active: Option<usize>) -> FilmstripLayout {
    FilmstripLayout::new(canvas(), count, active).expect("the canvas has room for a strip")
}

fn midpoint(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn the_strip_is_centred_on_the_bottom_edge_of_the_canvas() {
    let layout = layout(4, Some(0));
    let canvas = canvas();

    let strip_centre = layout.strip.origin.x + layout.strip.size.x / 2.0;
    let canvas_centre = canvas.origin.x + canvas.size.x / 2.0;
    assert!(
        (strip_centre - canvas_centre).abs() < 0.5,
        "strip centre {strip_centre} vs canvas centre {canvas_centre}"
    );
    assert_eq!(layout.strip.size.y, FILMSTRIP_HEIGHT);
    let bottom = layout.strip.origin.y + layout.strip.size.y;
    assert!((bottom - (canvas.origin.y + canvas.size.y - BOTTOM_MARGIN)).abs() < 0.5);
}

#[test]
fn a_short_deck_sizes_the_strip_to_its_chips() {
    let three = layout(3, Some(0));
    assert!((three.strip.size.x - (content_width(3) + STRIP_PAD * 2.0)).abs() < 0.5);
    // Every chip fits, so the row never scrolls.
    assert_eq!(three.offset, 0.0);
    assert_eq!(three.visible_chips().len(), 3);
}

#[test]
fn chips_sit_in_page_order_left_to_right() {
    let layout = layout(4, Some(0));
    let xs: Vec<f32> = (0..4).map(|i| layout.chip_rect(i).origin.x).collect();
    for pair in xs.windows(2) {
        assert!(
            pair[1] > pair[0],
            "chip order must read left to right: {xs:?}"
        );
        assert!((pair[1] - pair[0] - (CHIP_W + CHIP_GAP)).abs() < 0.01);
    }
}

#[test]
fn a_click_on_a_chip_resolves_to_its_page_index() {
    let layout = layout(5, Some(0));
    for index in 0..5 {
        assert_eq!(
            layout.chip_at(midpoint(layout.chip_rect(index))),
            Some(index)
        );
    }
    // The gap between two chips is the strip's, not a chip's: it must
    // swallow the press without navigating anywhere.
    let gap_x = layout.chip_rect(0).origin.x + CHIP_W + CHIP_GAP / 2.0;
    let gap = Point2D::new(gap_x, midpoint(layout.inner).y);
    assert_eq!(layout.chip_at(gap), None);
    assert!(layout.contains_point(gap));
}

#[test]
fn a_press_off_the_strip_is_not_the_strips() {
    let layout = layout(3, Some(0));
    let above = Point2D::new(midpoint(layout.strip).x, layout.strip.origin.y - 12.0);
    assert!(!layout.contains_point(above));
    assert_eq!(layout.chip_at(above), None);
}

#[test]
fn a_long_deck_scrolls_to_keep_the_current_slide_on_screen() {
    // 40 chips is far wider than any canvas here.
    let count = 40;
    assert!(content_width(count) > canvas().size.x);

    let first = layout(count, Some(0));
    assert_eq!(first.offset, 0.0, "slide 1 pins the row to its left end");
    assert_eq!(first.chip_at(midpoint(first.chip_rect(0))), Some(0));

    let last = layout(count, Some(count - 1));
    let max_offset = content_width(count) - last.inner.size.x;
    assert!(
        (last.offset - max_offset).abs() < 0.5,
        "offset {}",
        last.offset
    );
    assert_eq!(
        last.chip_at(midpoint(last.chip_rect(count - 1))),
        Some(count - 1),
        "the last slide's chip must be reachable"
    );

    let middle = layout(count, Some(20));
    let chip = middle.chip_rect(20);
    assert!(chip.origin.x >= middle.inner.origin.x);
    assert!(chip.origin.x + chip.size.x <= middle.inner.origin.x + middle.inner.size.x + 0.5);
}

#[test]
fn a_chip_scrolled_out_of_the_band_is_not_clickable() {
    let layout = layout(40, Some(20));
    // Slide 1 is far off the left edge at this offset.
    let off_screen = layout.chip_rect(0);
    assert!(off_screen.origin.x + off_screen.size.x < layout.inner.origin.x);
    assert_eq!(layout.chip_at(midpoint(off_screen)), None);
}

#[test]
fn a_canvas_too_narrow_for_one_chip_shows_no_strip() {
    let narrow = Rect {
        origin: Point2D::ZERO,
        size: Point2D::new(CHIP_W, 600.0),
    };
    assert_eq!(FilmstripLayout::new(narrow, 3, Some(0)), None);
    // And a deck with no boards has nothing to lay out.
    assert_eq!(FilmstripLayout::new(canvas(), 0, None), None);
}

#[test]
fn the_drop_slot_flips_at_each_chip_centre() {
    let layout = layout(4, Some(0));
    // Left of chip 0's centre → before chip 0.
    assert_eq!(layout.insertion_slot(layout.chip_rect(0).origin.x + 1.0), 0);
    // Just past chip 0's centre → between 0 and 1.
    assert_eq!(
        layout.insertion_slot(midpoint(layout.chip_rect(0)).x + 1.0),
        1
    );
    assert_eq!(
        layout.insertion_slot(midpoint(layout.chip_rect(2)).x + 1.0),
        3
    );
    // Past the last chip → the end of the deck.
    let far_right = layout.chip_rect(3).origin.x + CHIP_W + 200.0;
    assert_eq!(layout.insertion_slot(far_right), 4);
}

#[test]
fn dropping_either_side_of_the_dragged_chip_changes_nothing() {
    // Slot 2 is the gap the chip already sits in front of, slot 3 the one
    // behind it: both leave slide 3 exactly where it was.
    assert_eq!(reorder_target_index(2, 2), None);
    assert_eq!(reorder_target_index(2, 3), None);
}

#[test]
fn a_drop_converts_to_the_index_the_board_ends_up_at() {
    // Dragging slide 4 (index 3) to the very front.
    assert_eq!(reorder_target_index(3, 0), Some(0));
    // Dragging slide 1 to the end of a five-slide deck: after removing it
    // the array is one shorter, so the last index is 4.
    assert_eq!(reorder_target_index(0, 5), Some(4));
    // A short hop backwards keeps the slot as-is.
    assert_eq!(reorder_target_index(4, 1), Some(1));
}

#[test]
fn a_press_that_barely_moves_is_still_a_click() {
    let steady = FilmstripDrag {
        from: 1,
        press_x: 400.0,
        pointer_x: 402.0,
    };
    assert!(!drag_is_live(&steady));
    let dragged = FilmstripDrag {
        from: 1,
        press_x: 400.0,
        pointer_x: 460.0,
    };
    assert!(drag_is_live(&dragged));
}

/// Names are cut to the chip, and a CJK name is cut by its real width —
/// four CJK glyphs are roughly twice as wide as four Latin letters, so a
/// character-count budget would spill them past the chip's edge.
#[test]
fn a_long_name_is_ellipsized_to_the_chip_by_measured_width() {
    // Advance model standing in for the backend: CJK glyphs are square,
    // Latin ones roughly half that.
    let measure = |text: &str| -> f32 {
        text.chars()
            .map(|ch| {
                if (ch as u32) >= 0x2E80 {
                    FONT_SIZE
                } else {
                    FONT_SIZE * 0.55
                }
            })
            .sum()
    };
    let budget = CHIP_W - CHIP_PAD_X * 2.0 - 10.0;

    let cjk = crate::widgets::file_menu::truncate_to_width_measured(
        "产品路线图与季度目标回顾",
        budget,
        measure,
    );
    assert!(cjk.ends_with('…'), "a cut name must say so: {cjk}");
    assert!(
        measure(&cjk) <= budget,
        "the cut name still overflows the chip: {cjk}"
    );
    assert!(
        cjk.chars().count() < "产品路线图与季度目标回顾".chars().count(),
        "characters must actually be dropped"
    );

    // A name that fits is left alone.
    let short = crate::widgets::file_menu::truncate_to_width_measured("封面", budget, measure);
    assert_eq!(short, "封面");
}
