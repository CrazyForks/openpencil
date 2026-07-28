//! Tests for the mobile bottom-breathing-room echo. Every case runs the real
//! jian layout through `geometry_diagnostics`, so the assertions are about
//! RESOLVED geometry, not authored numbers.
//!
//! The bottom-nav exception is deliberately name-blind: the negative cases
//! below include a nav bar named nothing nav-ish, and the positive cases
//! include a trailing section that a name heuristic would have mistaken for
//! chrome.

use crate::geometry_validation::geometry_diagnostics;
use serde_json::json;

const ECHO: &str = "mobile-bottom-flush";

fn diagnostics_for(root: serde_json::Value) -> Vec<String> {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "children": [root]
    }))
    .expect("valid document");
    let state = op_editor_core::EditorState::from_document(doc);
    geometry_diagnostics(&state)
}

fn echoed(root: serde_json::Value) -> bool {
    diagnostics_for(root).iter().any(|d| d.contains(ECHO))
}

/// A movie-detail screen: no bottom nav, last section ends exactly at the
/// screen bottom. This is the reported failure (2026-07-28 screenshot).
fn detail_screen(trailing_bottom_padding: f64) -> serde_json::Value {
    json!({
        "type": "frame", "id": "root", "name": "Movie Detail",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "children": [
            { "type": "frame", "id": "hero", "name": "Hero",
              "width": "fill_container", "height": 520 },
            { "type": "frame", "id": "cast", "name": "Cast",
              "width": "fill_container", "height": 324,
              "layout": "vertical", "padding": [0, 24, trailing_bottom_padding, 24] }
        ]
    })
}

#[test]
fn detail_screen_without_a_bottom_nav_echoes_the_flush_edge() {
    let issues = diagnostics_for(detail_screen(0.0));
    let echo = issues
        .iter()
        .find(|d| d.contains(ECHO))
        .unwrap_or_else(|| panic!("expected the flush-bottom echo, got {issues:?}"));
    assert!(echo.contains("Movie Detail"), "{echo}");
    assert!(
        echo.contains("no bottom navigation bar") && echo.contains("24-32px"),
        "the echo must state the fact and the remedy: {echo}"
    );
}

#[test]
fn a_screen_with_bottom_breathing_room_is_silent() {
    // Same screen, 32px of trailing room — the shorter last section leaves the
    // gap the corpus rule asks for.
    let root = json!({
        "type": "frame", "id": "root", "name": "Movie Detail",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "children": [
            { "type": "frame", "id": "hero", "name": "Hero",
              "width": "fill_container", "height": 520 },
            { "type": "frame", "id": "cast", "name": "Cast",
              "width": "fill_container", "height": 292 }
        ]
    });
    assert!(!echoed(root), "32px of room must not be reported");
}

#[test]
fn a_screen_closed_by_a_bottom_nav_shape_is_silent_without_reading_its_name() {
    // No `role`, and a name a keyword heuristic would never match — the nav is
    // recognised purely from its resolved shape: full-width trailing band in
    // the nav height range with five evenly-sized tap targets.
    let tab = |id: &str| {
        json!({ "type": "frame", "id": id, "width": "fill_container", "height": 68,
                "layout": "vertical" })
    };
    let root = json!({
        "type": "frame", "id": "root", "name": "Home",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "children": [
            { "type": "frame", "id": "feed", "name": "Feed",
              "width": "fill_container", "height": 776 },
            { "type": "frame", "id": "chrome", "name": "Chrome Strip",
              "width": "fill_container", "height": 68, "layout": "horizontal",
              "children": [tab("t1"), tab("t2"), tab("t3"), tab("t4"), tab("t5")] }
        ]
    });
    assert!(
        !echoed(root),
        "a nav-shaped trailing band closes the screen legitimately"
    );
}

#[test]
fn a_role_tagged_bottom_nav_is_silent() {
    let root = json!({
        "type": "frame", "id": "root", "name": "Home",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "children": [
            { "type": "frame", "id": "feed", "name": "Feed",
              "width": "fill_container", "height": 774 },
            { "type": "frame", "id": "nav", "name": "Nav",
              "role": "bottom-tab-bar", "width": "fill_container", "height": 70 }
        ]
    });
    assert!(!echoed(root), "the authored role closes the screen");
}

#[test]
fn a_trailing_band_that_is_not_nav_shaped_still_echoes() {
    // A full-width 68px trailing band with only TWO children is a footer CTA
    // row, not a tab bar — the screen still ends flush and still gets the echo.
    // A name heuristic ("Bottom Bar") would have silenced this one.
    let half = |id: &str| {
        json!({ "type": "frame", "id": id, "width": "fill_container", "height": 68,
                "layout": "vertical" })
    };
    let root = json!({
        "type": "frame", "id": "root", "name": "Checkout",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "children": [
            { "type": "frame", "id": "summary", "name": "Summary",
              "width": "fill_container", "height": 776 },
            { "type": "frame", "id": "bar", "name": "Bottom Bar",
              "width": "fill_container", "height": 68, "layout": "horizontal",
              "children": [half("a"), half("b")] }
        ]
    });
    assert!(echoed(root), "a two-target footer row is not a tab bar");
}

#[test]
fn an_uneven_trailing_row_is_not_treated_as_a_tab_bar() {
    // A 20/80 split is a price + CTA row, not evenly-distributed tab targets.
    let root = json!({
        "type": "frame", "id": "root", "name": "Product",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "children": [
            { "type": "frame", "id": "body", "name": "Body",
              "width": "fill_container", "height": 776 },
            { "type": "frame", "id": "bar", "name": "Action Bar",
              "width": "fill_container", "height": 68, "layout": "horizontal",
              "children": [
                  { "type": "frame", "id": "price", "width": 78, "height": 68 },
                  { "type": "frame", "id": "cta", "width": 312, "height": 68 },
                  { "type": "frame", "id": "pad", "width": 0, "height": 68 }
              ] }
        ]
    });
    assert!(echoed(root), "unequal targets are not a tab bar");
}

#[test]
fn a_desktop_width_root_is_out_of_scope() {
    let root = json!({
        "type": "frame", "id": "root", "name": "Dashboard",
        "width": 1440, "height": 900, "layout": "vertical", "gap": 0,
        "children": [{ "type": "frame", "id": "body", "name": "Body",
                       "width": "fill_container", "height": 900 }]
    });
    assert!(!echoed(root), "the rule is mobile-only");
}

#[test]
fn a_short_mobile_width_component_is_out_of_scope() {
    // 390×320 is a card/component, not a screen — it is supposed to end at its
    // content.
    let root = json!({
        "type": "frame", "id": "root", "name": "Card",
        "width": 390, "height": 320, "layout": "vertical", "gap": 0,
        "children": [{ "type": "frame", "id": "body", "name": "Body",
                       "width": "fill_container", "height": 320 }]
    });
    assert!(!echoed(root), "a component is not a screen");
}

#[test]
fn content_overflowing_the_root_is_left_to_the_spill_diagnostics() {
    // A negative gap is a different fact; this echo must not claim it.
    let root = json!({
        "type": "frame", "id": "root", "name": "Overflowing Screen",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "clipContent": true,
        "children": [{ "type": "frame", "id": "body", "name": "Body",
                       "width": "fill_container", "height": 1200 }]
    });
    assert!(!echoed(root), "overflow is not a flush-bottom report");
}
