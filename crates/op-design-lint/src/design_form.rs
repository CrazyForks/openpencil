//! What kind of SURFACE a root frame is, judged from the artboard itself.
//!
//! **Why this lives in the lint crate rather than the orchestrator.** The
//! classifier is a pure judgement over two numbers, and its two consumers sit
//! on opposite sides of the crate graph: the orchestrator's repair passes
//! (`spacing_repair`, the geometry validation loop, `role_post_pass`) and the
//! detectors in [`crate::detectors`]. `op-orchestrator` depends on
//! `op-design-lint`, so the only placement that keeps ONE judge for both is
//! the lower crate. `op_orchestrator::design_type` re-exports every item here,
//! so the orchestrator-side import paths are unchanged.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::SizingBehavior;

/// What kind of SURFACE an assembled root frame is, judged from the artboard
/// itself rather than from the prompt.
///
/// `op_orchestrator::design_type::detect_design_type` answers "what did the
/// user ask for", and only the prompt / plan layer can call it — repair passes
/// run on an assembled tree, and on the agentic-loop path there is no plan at
/// all. They need the same distinction derived from what is actually on the
/// canvas.
///
/// **This is that single judge.** A repair pass must not re-derive the form
/// from a width comparison of its own: the workspace already carries six
/// separate `480.0` literals (`mobile_reflow`, `mobile_content_rail`,
/// `geometry_bottom_gap`, `cleanup_mobile_dense`, `cleanup_root_and_nav`,
/// `role_defaults`), which is exactly the drift this exists to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignForm {
    /// A phone-sized viewport. Chrome contracts (status bar, bottom nav),
    /// edge-to-edge content and tight rhythm apply here.
    MobileScreen,
    /// A scrolling page wide enough for a desktop browser — marketing site,
    /// landing page, desktop app screen. Sections own a vertical rhythm the
    /// root's gap cannot express, and content sits inside gutters.
    Page,
    /// A fixed 16:9 projector board. Neither a viewport nor a scroll surface.
    Deck,
    /// Not enough evidence to classify — an unsized root, a `fill_container`
    /// width, or a width between the phone and desktop bands. Passes MUST
    /// treat this as "no type information", never as a default form.
    Unknown,
}

impl DesignForm {
    /// A surface the reader scrolls through, where the root's direct children
    /// are page sections rather than viewport chrome.
    pub fn is_scrolling_page(self) -> bool {
        matches!(self, DesignForm::Page)
    }

    /// A fixed projector board. Content that overflows one is not clipped or
    /// shrunk — it moves to another board (deck-system spec §3.1), which no
    /// geometry pass can decide on its own.
    pub fn is_deck_board(self) -> bool {
        matches!(self, DesignForm::Deck)
    }
}

/// Widest artboard (inclusive) that reads as a phone viewport.
///
/// `op_orchestrator::plan_normalize::MOBILE_MAX_WIDTH` aliases this constant
/// so the plan layer and the tree layer agree on the band by construction.
pub const MOBILE_MAX_WIDTH: f64 = 480.0;
/// Narrowest artboard that reads as a desktop browser page. Between this and
/// [`MOBILE_MAX_WIDTH`] sits the tablet band, which is deliberately
/// [`DesignForm::Unknown`] — neither set of contracts is safe to assume there.
const PAGE_MIN_WIDTH: f64 = 1024.0;
/// Narrowest artboard that can be a projector board (the 1920 preset, minus
/// room for a model that rounds down).
const DECK_MIN_WIDTH: f64 = 1600.0;
/// 16:9 is 0.5625. The band accepts a board a model sized slightly off while
/// still excluding any page tall enough to scroll.
const DECK_ASPECT_RANGE: std::ops::RangeInclusive<f64> = 0.50..=0.65;

/// Classify a root frame from its artboard size. `width` / `height` are the
/// authored numeric values; a non-numeric (`fill_container`, `fit_content`) or
/// absent size is passed as `None` and yields [`DesignForm::Unknown`].
pub fn classify_root_form(width: Option<f64>, height: Option<f64>) -> DesignForm {
    let Some(width) = width.filter(|w| *w > 0.0) else {
        return DesignForm::Unknown;
    };
    if width <= MOBILE_MAX_WIDTH {
        return DesignForm::MobileScreen;
    }
    if width >= DECK_MIN_WIDTH {
        if let Some(height) = height.filter(|h| *h > 0.0) {
            if DECK_ASPECT_RANGE.contains(&(height / width)) {
                return DesignForm::Deck;
            }
        }
    }
    if width >= PAGE_MIN_WIDTH {
        return DesignForm::Page;
    }
    DesignForm::Unknown
}

/// [`classify_root_form`] over a root node's JSON. Sizes that are strings
/// (`"fill_container"`) read as unknown, matching the numeric contract above.
pub fn classify_root_form_value(root: &serde_json::Value) -> DesignForm {
    let number = |key: &str| root.get(key).and_then(serde_json::Value::as_f64);
    classify_root_form(number("width"), number("height"))
}

/// [`classify_root_form`] over a typed root node.
///
/// Only the three container variants can BE an artboard; anything else is a
/// leaf that happens to sit at the top level, and reading its box as a board
/// would classify a stray text node as a page. Reads the typed fields rather
/// than round-tripping through `serde_json` — the detectors call this on every
/// root, and serializing a whole design to read two numbers is not free.
pub fn classify_root_form_node(root: &PenNode) -> DesignForm {
    let container = match root {
        PenNode::Frame(node) => &node.container,
        PenNode::Group(node) => &node.container,
        PenNode::Rectangle(node) => &node.container,
        _ => return DesignForm::Unknown,
    };
    classify_root_form(size_px(&container.width), size_px(&container.height))
}

/// The pixel value of an authored size, or `None` for a keyword / expression.
fn size_px(size: &Option<SizingBehavior>) -> Option<f64> {
    match size {
        Some(SizingBehavior::Number(px)) => Some(*px),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    #[test]
    fn a_typed_root_classifies_the_same_as_its_json() {
        let deck = node(json!({
            "type": "frame", "id": "board", "width": 1920, "height": 1080
        }));
        assert_eq!(classify_root_form_node(&deck), DesignForm::Deck);
        assert_eq!(
            classify_root_form_value(&json!({"width": 1920, "height": 1080})),
            DesignForm::Deck
        );
    }

    #[test]
    fn a_keyword_sized_root_is_unknown_not_a_default() {
        let root = node(json!({
            "type": "frame", "id": "root", "width": "fill_container", "height": 1080
        }));
        assert_eq!(classify_root_form_node(&root), DesignForm::Unknown);
    }

    #[test]
    fn a_leaf_at_the_top_level_is_never_an_artboard() {
        // A stray 1920x1080 image is not a projector board — only a container
        // variant can be one.
        let image = node(json!({
            "type": "image", "id": "hero", "src": "x.png", "width": 1920, "height": 1080
        }));
        assert_eq!(classify_root_form_node(&image), DesignForm::Unknown);
    }

    #[test]
    fn a_deck_board_reports_itself_as_one() {
        assert!(DesignForm::Deck.is_deck_board());
        assert!(!DesignForm::Page.is_deck_board());
        assert!(!DesignForm::Unknown.is_deck_board());
    }
}
