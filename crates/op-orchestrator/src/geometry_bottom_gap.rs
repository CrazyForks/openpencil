//! Bottom-breathing-room echo for mobile screens.
//!
//! A phone screen whose last section ends flush against the artboard bottom
//! reads as cut off — the content collides with the device edge instead of
//! resting above it. Screens that end in a bottom navigation bar are the
//! deliberate exception: the nav IS the closing element and is supposed to sit
//! flush.
//!
//! This is detect-only. Whether a screen wants 24px, 32px, or a trailing
//! spacer is INTENT, and the "contract → auto-fix, intent → echo" split says
//! we report the fact and let the in-loop model decide. The corpus rule the
//! model is being held to lives in `skills/phases/generation/mobile-ui.md`
//! (MOBILE BOTTOM BREATHING ROOM).
//!
//! No name heuristics: the bottom-nav exception is decided from the authored
//! `role` semantic plus resolved geometry (full-width trailing band, nav-band
//! height, ≥3 evenly-sized tap targets in a horizontal row) — never from what
//! the model happened to call the frame.

use super::*;

/// Widest root that still reads as a phone artboard — same threshold the
/// bottom-nav containment echo uses.
const MOBILE_ROOT_MAX_WIDTH: f64 = 480.0;
/// Full screens only. A short mobile-width root is a component, not a screen,
/// and a component is supposed to end at its content.
const MOBILE_ROOT_MIN_HEIGHT: f64 = 500.0;
/// Below this the last element is touching the bottom edge. Deliberately well
/// under the corpus minimum (24px) so the echo states a fact rather than
/// nagging about a slightly tight but clearly intentional inset.
const FLUSH_BOTTOM_GAP: f64 = 12.0;
/// Height band a bottom navigation bar occupies. The corpus asks for 62-72px;
/// the band is widened at both ends so a nav that came out slightly short or
/// tall still earns its exception instead of drawing a false echo.
const NAV_HEIGHT_RANGE: std::ops::RangeInclusive<f64> = 44.0..=96.0;
/// A tab bar carries at least this many tap targets.
const NAV_MIN_TABS: usize = 3;
/// How far a tab may deviate from the mean tab width and still count as one
/// of an evenly-distributed row.
const NAV_TAB_WIDTH_TOLERANCE: f64 = 0.35;
const FULL_WIDTH_EPS: f64 = 1.0;

/// Echo a mobile screen whose content runs into the bottom edge with no
/// bottom navigation bar to close it.
pub(super) fn push_mobile_bottom_gap_diagnostic(
    root: &Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
) {
    if out.len() >= MAX_DIAGNOSTICS {
        return;
    }
    let Some(root_rect) = resolved(root, rects) else {
        return;
    };
    if root_rect.w > MOBILE_ROOT_MAX_WIDTH || root_rect.h < MOBILE_ROOT_MIN_HEIGHT {
        return;
    }
    // Only a flow-laid screen has a meaningful "last content edge"; an
    // absolutely-positioned root stacks its children wherever it likes.
    if layout_str(root) != Some("vertical") {
        return;
    }
    let kids = children(root);
    let Some(last) = kids.last() else {
        return;
    };
    if is_bottom_nav_shape(last, &root_rect, rects) {
        return;
    }
    // The lowest resolved edge across the root's direct children — not just
    // the last one in document order, since an overlay or a taller sibling can
    // be what actually reaches the bottom.
    let content_bottom = kids
        .iter()
        .filter_map(|child| resolved(child, rects))
        .map(|rect| rect.y + rect.h)
        .fold(f64::NEG_INFINITY, f64::max);
    if !content_bottom.is_finite() {
        return;
    }
    let gap = root_rect.y + root_rect.h - content_bottom;
    // A negative gap is content OVERFLOWING the root — a different fact, and
    // the spill diagnostics already report it.
    if !(0.0..FLUSH_BOTTOM_GAP).contains(&gap) {
        return;
    }
    out.push(format!(
        "mobile-bottom-flush: the last content edge under {} resolves {:.0}px from the \
         screen bottom, and this screen has no bottom navigation bar to close it — the \
         content runs into the device edge. Give the final section 24-32px of bottom \
         breathing room (bottom padding, or a trailing spacer frame).",
        diag_label(root),
        gap
    ));
}

/// Is this trailing child a bottom navigation bar?
///
/// Two independent sufficient signals, neither of which reads `name`:
/// the authored `role` semantic the corpus mandates, or the resolved shape a
/// tab bar always has — a full-width band in the nav height range laying out
/// at least three evenly-sized tap targets in a row.
pub(super) fn is_bottom_nav_shape(
    child: &Value,
    root_rect: &Rect,
    rects: &HashMap<String, Rect>,
) -> bool {
    if child.get("role").and_then(Value::as_str) == Some("bottom-tab-bar") {
        return true;
    }
    if layout_str(child) != Some("horizontal") {
        return false;
    }
    let Some(rect) = resolved(child, rects) else {
        return false;
    };
    if rect.w < root_rect.w - FULL_WIDTH_EPS || !NAV_HEIGHT_RANGE.contains(&rect.h) {
        return false;
    }
    let tabs: Vec<Rect> = children(child)
        .iter()
        .filter_map(|tab| resolved(tab, rects))
        .collect();
    if tabs.len() < NAV_MIN_TABS {
        return false;
    }
    let mean = tabs.iter().map(|tab| tab.w).sum::<f64>() / tabs.len() as f64;
    if mean <= 0.0 {
        return false;
    }
    tabs.iter()
        .all(|tab| (tab.w - mean).abs() <= mean * NAV_TAB_WIDTH_TOLERANCE)
}

fn resolved(v: &Value, rects: &HashMap<String, Rect>) -> Option<Rect> {
    v.get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
        .copied()
}
