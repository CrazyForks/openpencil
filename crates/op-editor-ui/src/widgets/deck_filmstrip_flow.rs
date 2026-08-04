//! Shared press / hover / release flow for the deck filmstrip.
//!
//! Both hosts hit-test the same rects and run the same state machine, so
//! all of it lives here and each host keeps only its platform tail (frame
//! the board, apply the reorder command, repaint). The native↔web
//! `widget_host` pair has drifted apart every time a flow was written
//! twice; this one is written once.
//!
//! The gesture: press marks a chip pressed, movement past the drag
//! threshold turns the press into a reorder, release either frames the
//! chip's board (a click) or moves the board in the child order (a drag).
//! Deciding on release is what lets a click with a shaky hand still
//! navigate.

use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::{EditorState, FilmstripDrag};

use crate::layout_scene::LayoutScene;
use crate::widgets::deck_filmstrip::{
    drag_is_live, reorder_target_index, DeckFilmstrip, FilmstripChip, FilmstripLayout,
};
use crate::{Point2D, Rect};

/// What a press on the strip amounted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilmstripPress {
    /// The point is not on the strip — the host falls through to the
    /// tiers below.
    Missed,
    /// The strip owns the press. It landed on a chip when `Some`, and on
    /// the pill's padding when `None`; either way nothing below may see
    /// it, or a press on the strip would also poke the canvas behind it.
    Claimed(Option<usize>),
}

/// What a release on the strip amounted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilmstripRelease {
    /// No press was in flight.
    Idle,
    /// A press was in flight but activated nothing (released off the
    /// chip, or dropped where it started).
    Cancelled,
    /// Frame this slide's board.
    Activate(usize),
    /// Move the board at `from` to child index `to`.
    Reorder { from: usize, to: usize },
}

/// Whether this document shows a filmstrip, and the slides it lists.
///
/// Gated on the recorded scenario, exactly like the slideshow itself —
/// a document is a deck because it was authored as one, never because
/// its boards happen to be 1920×1080. Preview hides the strip: while a
/// deck is being presented the whole canvas belongs to the slide, and a
/// navigator that kept painting there would also keep eating clicks
/// meant for the presentation.
pub fn filmstrip_chips(state: &EditorState) -> Option<Vec<FilmstripChip>> {
    if state.editor_ui.scenario != Some(TemplateScene::Slides) || state.editor_ui.preview.mode {
        return None;
    }
    let chips = board_chips(state);
    (!chips.is_empty()).then_some(chips)
}

/// The active page's top-level boards as navigator entries, in page
/// order, with no scenario gating.
///
/// Split out of [`filmstrip_chips`] so the left rail's slides tab lists
/// the same boards in the same order as the filmstrip does — the two
/// navigators differ in where they paint and which scenarios show them,
/// never in what a slide IS.
pub fn board_chips(state: &EditorState) -> Vec<FilmstripChip> {
    let children = state.active_children();
    active_page_boards(state)
        .into_iter()
        .map(|id| {
            let name = children
                .iter()
                .find(|node| op_editor_core::PenNodeExt::id_str(*node) == id)
                .and_then(|node| op_editor_core::PenNodeExt::base(node).name.clone())
                .unwrap_or_default();
            FilmstripChip { id, name }
        })
        .collect()
}

/// The slide the camera is looking at: the board whose centre is nearest
/// the centre of the canvas region.
///
/// Nearest-centre rather than "the one under the cursor" or "the
/// selected node" because the strip has to answer for a camera that is
/// merely parked somewhere, which is most of the time. `None` when no
/// board resolves in the scene yet — the strip then simply highlights
/// nothing rather than guessing at slide 1.
pub fn active_chip_index(
    chips: &[FilmstripChip],
    scene: &LayoutScene,
    state: &EditorState,
    canvas: Rect,
) -> Option<usize> {
    let viewport = &state.viewport;
    if viewport.zoom <= 0.0 {
        return None;
    }
    // Screen centre of the canvas region, back through the camera.
    let centre = Point2D::new(
        (canvas.size.x / 2.0 - viewport.pan_x) / viewport.zoom,
        (canvas.size.y / 2.0 - viewport.pan_y) / viewport.zoom,
    );
    let page = scene.active_page()?;
    chips
        .iter()
        .enumerate()
        .filter_map(|(index, chip)| {
            let bounds = page.find(&chip.id)?.aggregate_bounds();
            let dx = bounds.origin.x + bounds.size.x / 2.0 - centre.x;
            let dy = bounds.origin.y + bounds.size.y / 2.0 - centre.y;
            Some((index, dx * dx + dy * dy))
        })
        .min_by(|(_, a): &(usize, f32), (_, b)| a.total_cmp(b))
        .map(|(index, _)| index)
}

/// Build the strip's layout for the current document, or `None` when no
/// strip is shown. The single entry point both hosts' paint and hit-test
/// go through, so neither can lay the strip out its own way.
pub fn filmstrip_layout(
    chips: &[FilmstripChip],
    active: Option<usize>,
    canvas: Rect,
) -> Option<FilmstripLayout> {
    FilmstripLayout::new(canvas, chips.len(), active)
}

/// The widget for the current state, ready to paint.
pub fn filmstrip_widget<'a>(
    chips: &'a [FilmstripChip],
    active: Option<usize>,
    state: &EditorState,
) -> DeckFilmstrip<'a> {
    let filmstrip = state.editor_ui.deck_filmstrip;
    let dragging = filmstrip.drag.is_some_and(|drag| drag_is_live(&drag));
    DeckFilmstrip {
        chips,
        active,
        // A hover wash under the drop bar reads as a second answer to
        // "where does this land", so the carried chip owns the strip
        // alone. Hover is still TRACKED during the drag — the release
        // checks it to tell a drop from a click.
        hover: filmstrip.hover.filter(|_| !dragging),
        drag: filmstrip.drag,
    }
}

/// Route a press at `point`. Records the pressed chip and arms the drag;
/// nothing moves until the release.
pub fn press(state: &mut EditorState, layout: &FilmstripLayout, point: Point2D) -> FilmstripPress {
    if !layout.contains_point(point) {
        return FilmstripPress::Missed;
    }
    let chip = layout.chip_at(point);
    let filmstrip = &mut state.editor_ui.deck_filmstrip;
    filmstrip.pressed = chip;
    // Seed hover from the press so a pointer that never sent a move
    // (touch, a synthesized click) can still activate on release.
    filmstrip.hover = chip;
    filmstrip.drag = chip.map(|from| FilmstripDrag {
        from,
        press_x: point.x,
        pointer_x: point.x,
    });
    FilmstripPress::Claimed(chip)
}

/// Track the cursor: the hover wash outside a gesture, the carried chip
/// during one. Returns whether anything visible changed.
///
/// A drag keeps following the pointer past the strip's own edges — a
/// reorder that cancelled the moment the cursor strayed a few pixels
/// below the pill would be unusable.
pub fn cursor_move(state: &mut EditorState, layout: &FilmstripLayout, point: Point2D) -> bool {
    let hover = layout.chip_at(point);
    let filmstrip = &mut state.editor_ui.deck_filmstrip;
    let mut changed = filmstrip.hover != hover;
    filmstrip.hover = hover;
    if let Some(drag) = filmstrip.drag.as_mut() {
        changed |= drag.pointer_x != point.x;
        drag.pointer_x = point.x;
    }
    changed
}

/// Close the gesture. The host applies whatever comes back.
pub fn release(state: &mut EditorState, layout: &FilmstripLayout) -> FilmstripRelease {
    let filmstrip = &mut state.editor_ui.deck_filmstrip;
    let pressed = filmstrip.pressed.take();
    let drag = filmstrip.drag.take();
    // A press on the pill's padding armed nothing, so its release has
    // nothing to close.
    let Some(pressed) = pressed else {
        return FilmstripRelease::Idle;
    };
    match drag {
        Some(drag) if drag_is_live(&drag) => {
            let slot = layout.insertion_slot(drag.pointer_x);
            match reorder_target_index(drag.from, slot) {
                Some(to) => FilmstripRelease::Reorder {
                    from: drag.from,
                    to,
                },
                None => FilmstripRelease::Cancelled,
            }
        }
        // A click activates only while the cursor is still on the chip it
        // pressed, the same contract the presenting toolbar uses.
        _ if state.editor_ui.deck_filmstrip.hover == Some(pressed) => {
            FilmstripRelease::Activate(pressed)
        }
        _ => FilmstripRelease::Cancelled,
    }
}

/// Move a board to child index `to`, undoably. Returns whether the
/// document changed.
///
/// A plain reparent of the board to the page root at a new index: it
/// rewrites the child ORDER and nothing else. No geometry is touched, so
/// the boards stay exactly where they are on the canvas and only the page
/// sequence — which is what presenting and exporting read — changes.
///
/// The snapshot is taken first and dropped again if the move is refused,
/// so a rejected reorder leaves no empty entry on the undo stack for the
/// user to step through.
pub fn apply_reorder(state: &mut EditorState, board_id: &str, to: usize) -> bool {
    state.commit_history();
    if state.apply(reorder_command(board_id, to)) {
        return true;
    }
    state.history.past.pop_back();
    false
}

/// The `EditorCommand` a filmstrip reorder issues. Public so the reorder
/// can be asserted on directly.
pub fn reorder_command(board_id: &str, to: usize) -> op_editor_core::EditorCommand {
    op_editor_core::EditorCommand::MoveNode {
        node_id: op_editor_core::NodeId::new(board_id.to_string()),
        target_parent: op_editor_core::NodeId::NONE,
        page_id: None,
        index: Some(to),
    }
}

#[cfg(test)]
#[path = "deck_filmstrip_flow_tests.rs"]
mod deck_filmstrip_flow_tests;
