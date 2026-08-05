//! Left-rail tab selection + the slides tab's pointer bookkeeping.
//!
//! The left rail is a two-tab surface for scenario documents: the
//! ordinary Pages + Layers tree, and a page navigator that lists one
//! thumbnail per top-level board. Which tab is showing, which row the
//! cursor is on, and the reorder gesture in flight are all transient UI
//! state and live here.
//!
//! The slide ORDER is deliberately absent: it is the document's own
//! top-level child order (`crate::preview_slideshow::active_page_boards`)
//! and a copy here would be a second answer to "what order are the
//! slides in".

use jian_core::scroll::ScrollState;

/// Which tab the left rail is showing.
///
/// `Layers` is the default and the only tab a non-scenario document
/// offers, so a document that never opts into a scenario behaves
/// exactly as it did before the tab row existed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LeftPanelTab {
    #[default]
    Layers,
    Slides,
}

/// A slides-list reorder gesture in flight.
///
/// `press_y` is kept alongside the live `pointer_y` so the release can
/// tell a click (jump to the slide) from a drag (reorder the deck) —
/// the two are the same gesture until the pointer has travelled far
/// enough, and deciding on release is what lets a shaky click still
/// navigate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlidesDrag {
    /// Index of the slide being dragged, in page order.
    pub from: usize,
    /// Where the press landed, in screen px.
    pub press_y: f32,
    /// Where the cursor is now, in screen px.
    pub pointer_y: f32,
}

/// What a press on the slides panel landed on. Hover and press both
/// carry one of these so the two never disagree about what is under
/// the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlidesPanelTarget {
    /// The "Layers" tab in the tab row.
    LayersTab,
    /// The "Slides" / "Cards" tab in the tab row.
    SlidesTab,
    /// A slide row, by index in page order.
    Slide(usize),
    /// The footer's present button.
    Present,
}

/// Left-rail tab selection + the slides tab's transient pointer state.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SlidesPanelState {
    /// Which tab the rail is showing.
    pub tab: LeftPanelTab,
    /// What the cursor is over — drives every hover wash in the panel.
    pub hover: Option<SlidesPanelTarget>,
    /// What a press landed on. Activates on RELEASE while the cursor is
    /// still on it, the same contract the presenting toolbar uses.
    pub pressed: Option<SlidesPanelTarget>,
    /// The reorder gesture in flight, once the pointer has moved.
    pub drag: Option<SlidesDrag>,
    /// Vertical scroll of the slide list.
    pub scroll: ScrollState,
}

impl SlidesPanelState {
    /// Forget every in-flight pointer interaction, keeping the selected
    /// tab and the scroll position. Returns whether anything was live —
    /// hosts use that as the repaint signal.
    ///
    /// Called whenever the panel stops being reachable (the sidebar
    /// closes, a presentation starts), so a gesture cannot survive the
    /// surface it belongs to.
    pub fn clear_pointer(&mut self) -> bool {
        let live = self.hover.is_some() || self.pressed.is_some() || self.drag.is_some();
        self.hover = None;
        self.pressed = None;
        self.drag = None;
        live
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layers_tab_is_the_default() {
        assert_eq!(SlidesPanelState::default().tab, LeftPanelTab::Layers);
    }

    #[test]
    fn clearing_pointer_state_keeps_the_tab_and_scroll() {
        let mut state = SlidesPanelState {
            tab: LeftPanelTab::Slides,
            hover: Some(SlidesPanelTarget::Slide(2)),
            pressed: Some(SlidesPanelTarget::Slide(2)),
            drag: Some(SlidesDrag {
                from: 2,
                press_y: 10.0,
                pointer_y: 40.0,
            }),
            scroll: ScrollState { offset: 24.0 },
        };
        assert!(state.clear_pointer());
        assert_eq!(state.tab, LeftPanelTab::Slides);
        assert_eq!(state.scroll.offset, 24.0);
        assert_eq!(state.hover, None);
        assert_eq!(state.pressed, None);
        assert_eq!(state.drag, None);
        // Idle again — nothing to repaint for.
        assert!(!state.clear_pointer());
    }
}
