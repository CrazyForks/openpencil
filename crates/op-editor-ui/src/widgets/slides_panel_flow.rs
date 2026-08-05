//! Shared tab / press / hover / release flow for the left rail's slides
//! tab.
//!
//! Both hosts hit-test the same rects and run the same state machine, so
//! all of it lives here and each host keeps only its platform tail (frame
//! the board, apply the reorder, start the presentation, repaint). The
//! native↔web `widget_host` pair has drifted apart every time a flow was
//! written twice; this one is written once.
//!
//! The gesture: press marks a row pressed, movement past the drag
//! threshold turns the press into a reorder, release either frames the
//! row's board (a click) or moves the board in the child order (a
//! drag). Deciding on release is what lets a click with a shaky hand
//! still navigate.

use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::{EditorState, LeftPanelTab, SlidesDrag, SlidesPanelTarget};

use crate::layout_scene::LayoutScene;
use crate::widgets::deck_boards::{board_chips, reorder_target_index, BoardChip};
use crate::widgets::slides_panel::{
    drag_is_live, SlidesPanel, SlidesPanelLayout, SlidesPanelTabs, DEFAULT_BOARD_ASPECT,
};
use crate::{Point2D, Rect};

/// What a press on the panel amounted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlidesPress {
    /// The point is not on the panel — the host falls through to the
    /// tiers below.
    Missed,
    /// The panel owns the press. It landed on something when `Some`,
    /// and on bare panel chrome when `None`; either way nothing below
    /// may see it.
    Claimed(Option<SlidesPanelTarget>),
}

/// What a release on the panel amounted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlidesRelease {
    /// No press was in flight.
    Idle,
    /// A press was in flight but activated nothing (released off the
    /// target, or dropped where it started).
    Cancelled,
    /// Show this tab.
    SelectTab(LeftPanelTab),
    /// Frame this slide's board.
    Activate(usize),
    /// Move the board at `from` to child index `to`.
    Reorder { from: usize, to: usize },
    /// Start presenting.
    Present,
}

/// What the navigator tab is called for this document.
///
/// The scenario picks the WORD — a deck lists slides, a card set lists
/// cards — but never whether the tab exists; see [`tab_row_visible`].
/// A document with no scenario recorded gets the neutral "Slides", which
/// is what the boards of an ordinary design page are to a navigator.
pub fn slides_tab_label_key(state: &EditorState) -> &'static str {
    match state.editor_ui.scenario {
        Some(TemplateScene::Card) => "slidesPanel.tabCards",
        _ => "slidesPanel.tabSlides",
    }
}

/// Whether the left rail shows its tab row for this document.
///
/// **Having boards is the whole test.** The tab used to be gated on the
/// recorded scenario as well, which meant the navigator — the only place
/// a page's order is visible, let alone reorderable — was missing from
/// every document not tagged a deck, including every multi-frame design
/// a user laid out by hand. A page with frames on it has an order, so it
/// gets the tab; a page with none has nothing to list, so the rail stays
/// the Layers tree it has always been. A pristine starter document has
/// no top-level frame, so a new file still opens without one.
///
/// Presenting hides it along with the whole rail, so this answers
/// `false` there too and no host can paint a navigator over a
/// presentation.
pub fn tab_row_visible(state: &EditorState) -> bool {
    !state.editor_ui.preview.mode && !board_chips(state).is_empty()
}

/// Both tab labels for this document, already translated.
///
/// The single place either host or the layout reads them from: the tab
/// row's rects are sized from the labels, so a caller resolving its own
/// copy could paint a pill that does not match the one it hit-tests.
pub fn tab_labels(state: &EditorState) -> (&'static str, &'static str) {
    let ui = &state.editor_ui;
    (
        crate::widgets::editor_state_ext::translate(ui, "layers.title"),
        crate::widgets::editor_state_ext::translate(ui, slides_tab_label_key(state)),
    )
}

/// The tab row's rects for a rail occupying `panel`, or `None` when
/// this document shows no tab row.
pub fn tab_row(state: &EditorState, panel: Rect) -> Option<SlidesPanelTabs> {
    tab_row_visible(state).then(|| {
        let (layers, slides) = tab_labels(state);
        SlidesPanelTabs::new(panel, state.editor_ui.slides_panel.tab, layers, slides)
    })
}

/// The rail rect the Layers tree gets: the whole panel, less the tab
/// row when one is shown.
///
/// The single source for "where do the layer rows start" — paint,
/// hit-test, drag and scroll all derive from it, so a tab row can never
/// shift the painted rows out from under their own click targets.
pub fn layers_content_rect(state: &EditorState, panel: Rect) -> Rect {
    match tab_row(state, panel) {
        Some(tabs) => tabs.content_rect(panel),
        None => panel,
    }
}

/// Whether the slides tab is the one on show. False whenever the tab
/// row is hidden, so a stale tab selection cannot strand a document
/// that stopped being a deck on a navigator it no longer has.
pub fn slides_tab_active(state: &EditorState) -> bool {
    tab_row_visible(state) && state.editor_ui.slides_panel.tab == LeftPanelTab::Slides
}

/// The slides listed in the panel, in page order.
pub fn slides(state: &EditorState) -> Vec<BoardChip> {
    board_chips(state)
}

/// Each board's width / height, in page order — one per chip, because
/// a page may hold a phone screen next to a dashboard and each is fitted
/// into the row box on its own terms.
///
/// Falls back to 16:9 for any board the scene has not resolved yet,
/// which happens on the first frame after a document opens.
pub fn board_aspects(chips: &[BoardChip], scene: &LayoutScene) -> Vec<f32> {
    chips
        .iter()
        .map(|chip| {
            let bounds = scene
                .active_page()
                .and_then(|page| page.find(&chip.id))
                .map(|node| node.aggregate_bounds());
            match bounds {
                Some(rect) if rect.size.x > 0.0 && rect.size.y > 0.0 => rect.size.x / rect.size.y,
                _ => DEFAULT_BOARD_ASPECT,
            }
        })
        .collect()
}

/// Build the panel's layout for the current document, or `None` when
/// the slides tab is not showing. The single entry point both hosts'
/// paint and hit-test go through, so neither can lay the rows out its
/// own way.
pub fn layout(
    state: &EditorState,
    chips: &[BoardChip],
    scene: &LayoutScene,
    panel: Rect,
) -> Option<SlidesPanelLayout> {
    if !slides_tab_active(state) {
        return None;
    }
    let (layers, slides) = tab_labels(state);
    SlidesPanelLayout::new(
        panel,
        SlidesPanelTabs::new(panel, state.editor_ui.slides_panel.tab, layers, slides),
        &board_aspects(chips, scene),
        state.editor_ui.slides_panel.scroll.offset,
    )
}

/// The widget for the current state, ready to paint.
pub fn widget<'a>(
    chips: &'a [BoardChip],
    active: Option<usize>,
    state: &EditorState,
    layers_label: &'a str,
    slides_label: &'a str,
    present_label: &'a str,
) -> SlidesPanel<'a> {
    let panel = state.editor_ui.slides_panel;
    let dragging = panel.drag.is_some_and(|drag| drag_is_live(&drag));
    SlidesPanel {
        chips,
        active,
        // A hover wash under the drop bar reads as a second answer to
        // "where does this land", so the carried row owns the list
        // alone. Hover is still TRACKED during the drag — the release
        // checks it to tell a drop from a click.
        hover: panel.hover.filter(|_| !dragging),
        drag: panel.drag,
        thumbnails_supported: state.editor_ui.slide_thumbnails_supported,
        layers_label,
        slides_label,
        present_label,
    }
}

/// Route a press at `point`. Records what was pressed and arms the
/// drag; nothing moves until the release.
pub fn press(state: &mut EditorState, layout: &SlidesPanelLayout, point: Point2D) -> SlidesPress {
    if !layout.contains_point(point) {
        return SlidesPress::Missed;
    }
    let hit = layout.hit(point);
    let panel = &mut state.editor_ui.slides_panel;
    panel.pressed = hit;
    // Seed hover from the press so a pointer that never sent a move
    // (touch, a synthesized click) can still activate on release.
    panel.hover = hit;
    panel.drag = match hit {
        Some(SlidesPanelTarget::Slide(from)) => Some(SlidesDrag {
            from,
            press_y: point.y,
            pointer_y: point.y,
        }),
        _ => None,
    };
    SlidesPress::Claimed(hit)
}

/// Track the cursor: the hover wash outside a gesture, the carried row
/// during one. Returns whether anything visible changed.
///
/// A drag keeps following the pointer past the list's own edges — a
/// reorder that cancelled the moment the cursor strayed a few pixels
/// below the rail would be unusable.
pub fn cursor_move(state: &mut EditorState, layout: &SlidesPanelLayout, point: Point2D) -> bool {
    let hover = layout.hit(point);
    let panel = &mut state.editor_ui.slides_panel;
    let mut changed = panel.hover != hover;
    panel.hover = hover;
    if let Some(drag) = panel.drag.as_mut() {
        changed |= drag.pointer_y != point.y;
        drag.pointer_y = point.y;
    }
    changed
}

/// Track the cursor over the tab row alone — the hover path while the
/// LAYERS tab owns the rail and there is no slides layout to hit-test
/// against.
pub fn tab_cursor_move(state: &mut EditorState, tabs: &SlidesPanelTabs, point: Point2D) -> bool {
    let hover = tabs.hit(point);
    let panel = &mut state.editor_ui.slides_panel;
    let changed = panel.hover != hover;
    panel.hover = hover;
    changed
}

/// Close the gesture. The host applies whatever comes back.
pub fn release(state: &mut EditorState, layout: &SlidesPanelLayout) -> SlidesRelease {
    let hover = state.editor_ui.slides_panel.hover;
    let panel = &mut state.editor_ui.slides_panel;
    let pressed = panel.pressed.take();
    let drag = panel.drag.take();
    // A press on bare panel chrome armed nothing, so its release has
    // nothing to close.
    let Some(pressed) = pressed else {
        return SlidesRelease::Idle;
    };
    if let Some(drag) = drag.filter(drag_is_live) {
        let slot = layout.insertion_slot(drag.pointer_y);
        return match reorder_target_index(drag.from, slot) {
            Some(to) => SlidesRelease::Reorder {
                from: drag.from,
                to,
            },
            None => SlidesRelease::Cancelled,
        };
    }
    // A click activates only while the cursor is still on what it
    // pressed, the same contract the presenting toolbar uses.
    if hover != Some(pressed) {
        return SlidesRelease::Cancelled;
    }
    match pressed {
        SlidesPanelTarget::LayersTab => SlidesRelease::SelectTab(LeftPanelTab::Layers),
        SlidesPanelTarget::SlidesTab => SlidesRelease::SelectTab(LeftPanelTab::Slides),
        SlidesPanelTarget::Slide(index) => SlidesRelease::Activate(index),
        SlidesPanelTarget::Present => SlidesRelease::Present,
    }
}

/// Close a press that landed on the tab row while the LAYERS tab owned
/// the rail. Same shape as [`release`], minus everything that needs a
/// slides layout.
pub fn tab_release(state: &mut EditorState) -> SlidesRelease {
    let hover = state.editor_ui.slides_panel.hover;
    let panel = &mut state.editor_ui.slides_panel;
    panel.drag = None;
    let Some(pressed) = panel.pressed.take() else {
        return SlidesRelease::Idle;
    };
    if hover != Some(pressed) {
        return SlidesRelease::Cancelled;
    }
    match pressed {
        SlidesPanelTarget::LayersTab => SlidesRelease::SelectTab(LeftPanelTab::Layers),
        SlidesPanelTarget::SlidesTab => SlidesRelease::SelectTab(LeftPanelTab::Slides),
        _ => SlidesRelease::Cancelled,
    }
}

/// Switch tabs. Returns whether the rail changed what it shows.
///
/// Leaving the slides tab drops the pointer bookkeeping with it: a
/// hover or a half-finished drag belonging to a list that is no longer
/// on screen would otherwise wake up when the user came back.
pub fn select_tab(state: &mut EditorState, tab: LeftPanelTab) -> bool {
    let panel = &mut state.editor_ui.slides_panel;
    if panel.tab == tab {
        return false;
    }
    panel.tab = tab;
    panel.clear_pointer();
    true
}

/// Scroll the slide list. `None` when the pointer is not over the
/// panel, so the caller keeps looking.
pub fn scroll(
    state: &mut EditorState,
    layout: Option<&SlidesPanelLayout>,
    point: Point2D,
    delta_y: f32,
) -> Option<bool> {
    let layout = layout?;
    if !layout.contains_point(point) {
        return None;
    }
    let max = layout.max_scroll();
    Some(crate::util::scroll_by_max(
        &mut state.editor_ui.slides_panel.scroll,
        -delta_y,
        max,
    ))
}

#[cfg(test)]
#[path = "slides_panel_flow_tests.rs"]
mod slides_panel_flow_tests;
