//! PropertyPanel draft commits — the blur / Enter path that turns the
//! shared `property_input` draft into a document write.
//!
//! Both hosts carried a byte-identical copy of this in
//! `widget_host/property_input_dispatch.rs`. It is pure `EditorState`
//! work over the panel's `PropertyFocus` vocabulary plus this crate's
//! hex helpers, so it lives here; the hosts keep only the surrounding
//! glue (the variables-panel commits that run first, and `mark_dirty`).

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, PropertyFocus};

use crate::util::{color_to_hex, color_to_hex_with_alpha, parse_hex_color};
use crate::Color;

/// Commit a pending effect-parameter edit (the Effects section's
/// editable value box). Parses the shared draft and writes it via
/// `SetEffectParam`; a non-numeric draft is discarded. Returns `true`
/// when a focus was taken, i.e. the host should mark dirty.
pub fn commit_effect_param_focus(state: &mut EditorState) -> bool {
    let Some(focus) = state.editor_ui.effect_param_focus.take() else {
        return false;
    };
    state.ui.property_draft_select_all = false;
    let draft = state.ui.property_input.text().to_owned();
    state.ui.property_input.set_text("");
    state.ui.property_input_draft.clear();
    state.ui.property_caret_pos = 0;
    if let Ok(value) = draft.trim().parse::<f32>() {
        if value.is_finite() {
            let id = state.selection.anchor.clone();
            if id.is_real() {
                // Instance-write redirect (GAP #10) — see
                // `apply_property_action` for the choke-point note.
                let instance_scope = state.begin_instance_write_for_anchor();
                state.commit_history();
                let _ = state.apply(op_editor_core::EditorCommand::SetEffectParam {
                    node_id: id,
                    index: focus.effect as u32,
                    field: focus.field,
                    value,
                });
                if let Some(scope) = instance_scope {
                    state.finish_instance_write(scope);
                }
            }
        }
    }
    true
}

/// Commit the focused property input. Returns `true` when a focus was
/// taken, i.e. the host should mark dirty.
///
/// The caller commits the variables-panel header / row drafts and the
/// effect-param draft first — those live host-side because they route
/// through host-owned panel state.
pub fn commit_property_focus(state: &mut EditorState) -> bool {
    let Some(focus) = state.ui.property_focus.take() else {
        return false;
    };
    state.ui.property_draft_select_all = false;
    let draft = state.ui.property_input.text().to_owned();
    state.ui.property_input.set_text("");
    state.ui.property_input_draft.clear();
    state.ui.property_caret_pos = 0;
    // Instance-write redirect (GAP #10) — see `apply_property_action`
    // for the choke-point note.
    let before = state.snapshot_for_history();
    let instance_scope = state.begin_instance_write_for_anchor();
    match focus {
        PropertyFocus::PageBackgroundHex => {
            let authored = draft.trim();
            // A page with no authored background seeds an empty draft.
            // Empty/unchanged blur is deliberately a no-op; clearing is
            // an explicit panel action so focus alone cannot inflate an
            // old document with a new background field.
            if state.active_page_background_color() != Some(authored) {
                if let Some(hex) = normalized_page_background_hex(authored) {
                    let _ = state.set_active_page_background_color(Some(hex));
                }
            }
        }
        PropertyFocus::ImageTileScale => {
            if let Ok(value) = draft.trim().parse::<f32>() {
                let _ = state.set_selected_image_tile_scale(value);
            }
        }
        PropertyFocus::FillHex(index) => {
            let stripped = draft.trim().trim_start_matches('#');
            if !stripped.is_empty() {
                if let Some(color) = parse_hex_color(draft.trim()) {
                    let hex = color_to_hex_with_alpha(color);
                    // The primary fill (index 0) keeps `set_selected_color`
                    // (prepends a solid + colour-variable-aware); a
                    // non-primary row writes its own solid fill by index.
                    if index == 0 {
                        let _ = state.set_selected_color(true, &hex);
                    } else {
                        let _ = state.set_selected_fill_hex_at(index, &hex);
                    }
                }
            }
        }
        PropertyFocus::StrokeHex => {
            let stripped = draft.trim().trim_start_matches('#');
            if !stripped.is_empty() {
                if let Some(color) = parse_hex_color(draft.trim()) {
                    let _ = state.set_selected_color(false, &color_to_hex(color));
                }
            }
        }
        PropertyFocus::GradientStopHex(index) => {
            let stripped = draft.trim().trim_start_matches('#');
            if !stripped.is_empty() {
                if let Some(color) = parse_hex_color(draft.trim()) {
                    // The input pill never paints alpha digits, so
                    // re-attach the stop's existing alpha here — a
                    // transparent stop must stay transparent after the
                    // user edits its RGB.
                    let existing_alpha = state
                        .selected_node()
                        .and_then(|n| current_stop_alpha(n, index))
                        .unwrap_or(1.0);
                    let with_alpha = Color {
                        r: color.r,
                        g: color.g,
                        b: color.b,
                        a: existing_alpha,
                    };
                    let _ = state.set_selected_gradient_stop_hex(
                        index,
                        &color_to_hex_with_alpha(with_alpha),
                    );
                }
            }
        }
        PropertyFocus::WidgetPlaceholder => {
            let _ = state.set_selected_widget_text(
                op_editor_core::WidgetTextField::Placeholder,
                draft.trim(),
            );
        }
        PropertyFocus::WidgetValue => {
            let _ = state
                .set_selected_widget_text(op_editor_core::WidgetTextField::Value, draft.trim());
        }
        PropertyFocus::WidgetLabel => {
            let _ = state
                .set_selected_widget_text(op_editor_core::WidgetTextField::Label, draft.trim());
        }
        PropertyFocus::WidgetLeadingIcon => {
            let _ = state.set_selected_widget_text(
                op_editor_core::WidgetTextField::LeadingIcon,
                draft.trim(),
            );
        }
        PropertyFocus::WidgetTrailingIcon => {
            let _ = state.set_selected_widget_text(
                op_editor_core::WidgetTextField::TrailingIcon,
                draft.trim(),
            );
        }
        PropertyFocus::WidgetBindKey => {
            let _ = state.set_selected_widget_bind_value(draft.trim());
        }
        _ => {
            if let Ok(value) = draft.trim().parse::<f32>() {
                let _ = state.commit_property_edit(focus, value);
            }
        }
    }
    if let Some(scope) = instance_scope {
        state.finish_instance_write(scope);
    }
    if state.snapshot_for_history() != before {
        state.history_push_past(before);
    }
    true
}

/// Read the live alpha of gradient stop `index` on `node`, parsed out
/// of the canonical hex (8-char `#RRGGBBAA`). `None` when the first
/// fill isn't a gradient or the stop hex omits alpha — the caller
/// defaults to `1.0` in that case so opaque stops stay opaque through
/// an RGB edit.
pub fn current_stop_alpha(node: &PenNode, index: usize) -> Option<f32> {
    use jian_ops_schema::style::PenFill;
    let first = op_editor_core::fills::node_fills(node).and_then(|f| f.first())?;
    let stops = match first {
        PenFill::LinearGradient(b) => &b.stops,
        PenFill::RadialGradient(b) => &b.stops,
        _ => return None,
    };
    let hex = &stops.get(index)?.color;
    Some(op_editor_core::parse_hex_alpha(hex))
}

/// Validate and canonicalize an authored page colour without routing it
/// through the RGB-only helper (which would discard an imported alpha
/// byte).
fn normalized_page_background_hex(value: &str) -> Option<String> {
    let digits = value.strip_prefix('#')?;
    if !matches!(digits.len(), 6 | 8) || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", digits.to_ascii_uppercase()))
}
