//! Shared Scene Template Center press transitions for native and web hosts.

use op_editor_core::{ButtonPressTarget, EditorState, SceneTemplateFocus};

use crate::widgets::{SceneTemplateHit, SceneTemplatePanel};
use crate::{Point2D, Rect};

/// Route one pointer press to the Asset Center gallery.
///
/// `Some(changed)` means the gallery swallowed the press; hosts repaint when
/// `changed` is true. `None` means the gallery is closed and the press must
/// fall through.
///
/// A press outside the panel does NOT fall through — it lands on the scrim,
/// and the scrim's whole job is to dismiss. The gallery covers the canvas
/// region and dims everything else, so a press on the dimmed band is a user
/// aiming at chrome they can barely see; treating it as a dismiss is both what
/// the dimming promises and what stops a half-visible button from firing.
///
/// Choosing a card does not open the document here. It raises
/// `scene_template_center.pending_open`, which the host drains: loading a
/// document is a host capability (it may have to prompt about unsaved work,
/// and on native it touches the recent-files list), and a widget that reached
/// into that would have to be reimplemented per host — the exact fork this
/// shared layer exists to prevent.
pub fn press_scene_template_center(
    state: &mut EditorState,
    panel_rect: Rect,
    point: Point2D,
    now_ms: u64,
) -> Option<bool> {
    let panel = SceneTemplatePanel::for_editor(state)?;
    let hover = panel.hover_at(panel_rect, point);
    let Some(hit) = panel.hit_test(panel_rect, point) else {
        return Some(state.editor_ui.close_scene_template_center());
    };
    let pressed = hover.map(ButtonPressTarget::SceneTemplate);
    let pressed_changed = state.editor_ui.pressed_button != pressed;
    state.editor_ui.pressed_button = pressed;

    let changed = match hit {
        SceneTemplateHit::Close => state.editor_ui.close_scene_template_center(),
        SceneTemplateHit::FocusSearch(offset) => {
            let center = &mut state.editor_ui.scene_template_center;
            let changed =
                center.search.caret() != offset || center.focus != SceneTemplateFocus::Search;
            center.focus = SceneTemplateFocus::Search;
            center.search.set_caret(offset, now_ms);
            changed
        }
        SceneTemplateHit::FocusGenerate(offset) => {
            let center = &mut state.editor_ui.scene_template_center;
            let changed =
                center.generate.caret() != offset || center.focus != SceneTemplateFocus::Generate;
            center.focus = SceneTemplateFocus::Generate;
            center.generate.set_caret(offset, now_ms);
            changed
        }
        // Nothing typed is not a request: the button stays live so it can
        // still take the press (and not fall through to the grid), but an
        // empty topic must not replace the document with a blank one.
        SceneTemplateHit::Generate => state.editor_ui.scene_template_center.request_generate(),
        SceneTemplateHit::SelectFilter(filter) => {
            let center = &mut state.editor_ui.scene_template_center;
            let changed =
                center.filter != filter || center.scroll.offset != 0.0 || center.hover.is_some();
            center.filter = filter;
            // A filter change reorders the grid, so a retained scroll offset
            // or hover index would point at a different card than the one the
            // pointer is over.
            center.scroll.offset = 0.0;
            center.hover = None;
            changed
        }
        SceneTemplateHit::SelectTab(tab) => state.editor_ui.scene_template_center.select_tab(tab),
        SceneTemplateHit::AddTemplateToCanvas(id) => {
            state.editor_ui.scene_template_center.request_open(id);
            true
        }
        // No document work here, and none queued for the host either: this
        // sets up the *next* request rather than making one. Pinning, the
        // filter, and the focus are all editor-UI state, so the panel can
        // apply them itself and stay open — which it must, because the row
        // the user is being pointed at is inside it.
        SceneTemplateHit::GenerateFromTemplate(id) => {
            match op_editor_core::scene_template_catalog::scene_template_by_id(&id) {
                Some(template) => state
                    .editor_ui
                    .use_scene_template_as_generate_basis(template),
                // The id came out of the catalogue a frame ago; an unknown one
                // means the two disagree, and doing nothing is better than
                // pinning a guide chosen for a template we cannot name.
                None => false,
            }
        }
        SceneTemplateHit::ClearGenerateBasis => {
            state.editor_ui.clear_scene_template_generate_basis()
        }
        // Pinning is policy, not a document edit: it is applied inline (no
        // host round-trip like `pending_open` needs) and it never touches
        // the node tree — the next generation reads it, nothing else does.
        // Pressing the pinned card again unpins, so the same gesture that
        // set the policy is the one that clears it, and at most one guide
        // is ever pinned.
        SceneTemplateHit::ToggleStyleGuide(name) => {
            let pinned = &mut state.editor_ui.pinned_style_guide;
            *pinned = if pinned.as_deref() == Some(name.as_str()) {
                None
            } else {
                Some(name)
            };
            true
        }
        SceneTemplateHit::Inside => false,
    };
    Some(changed || pressed_changed)
}

/// Route a wheel/trackpad scroll to the card grid.
///
/// `None` only when the gallery is closed. While it is open the scroll is
/// swallowed wherever the pointer is: over the grid it scrolls the cards, and
/// over the scrim it does nothing — panning the canvas the user cannot see
/// behind a dimmed gallery is motion they did not ask for and cannot follow.
pub fn scroll_scene_template_center(
    state: &mut EditorState,
    panel_rect: Rect,
    point: Point2D,
    delta_y: f32,
) -> Option<bool> {
    let panel = SceneTemplatePanel::for_editor(state)?;
    if !panel_rect.contains(point) {
        return Some(false);
    }
    let max_scroll = panel.max_scroll(panel_rect);
    let center = &mut state.editor_ui.scene_template_center;
    let previous = center.scroll.offset;
    center.scroll.offset = (previous - delta_y).clamp(0.0, max_scroll);
    // Swallow the event either way: a grid that has nothing left to scroll
    // must not hand the wheel to the canvas underneath it.
    Some(center.scroll.offset != previous)
}

/// Resolve hover for a pointer move.
///
/// Returns `(owns_point, changed)` — hosts need the first to suppress
/// canvas-level hover while the gallery is up, which is the gate a floating
/// overlay is easiest to forget (measured 2026-07-29 on the colour variable
/// popover: hover fell straight through to the layer underneath).
///
/// An open gallery owns every point, not just the ones on the panel: the
/// scrim covers the rest of the viewport and takes the press there, so
/// lighting up a layer row or a toolbar button underneath it would advertise
/// a click that dismisses instead.
pub fn hover_scene_template_center(
    state: &mut EditorState,
    panel_rect: Rect,
    point: Point2D,
) -> (bool, bool) {
    let Some(panel) = SceneTemplatePanel::for_editor(state) else {
        return (false, false);
    };
    let hover = panel.hover_at(panel_rect, point);
    let center = &mut state.editor_ui.scene_template_center;
    let changed = center.hover != hover;
    center.hover = hover;
    (true, changed)
}

#[cfg(test)]
#[path = "scene_template_press_flow_tests.rs"]
mod scene_template_press_flow_tests;
