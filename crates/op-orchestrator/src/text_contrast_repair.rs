//! Re-point text whose colour is invisible against its own background.
//!
//! `op_design_lint::detectors::typography` has detected this since 2026-05,
//! but nothing in the generation path ever called it: the orchestrator uses
//! exactly one lint detector (`detect_missing_progress_rings`), and the rest
//! only run through the MCP `lint_document` tool a user invokes by hand. So a
//! deck cover shipped with its title at **1.10:1** — `#FFFFFF` on `#F1F5F9`,
//! effectively blank (measured 2026-08-01, deepseek-v4-pro).
//!
//! ## Why this repairs rather than echoes
//!
//! The detector deliberately suggests nothing, because "which brand colour
//! belongs here" is an intent question. Picking a *readable* colour is not the
//! same question: when the resolved ratio is near 1:1 the text is not styled,
//! it is missing, and every candidate below comes from the document's own
//! palette. So this stays inside the "contract, auto-fixable" half of the
//! self-check split — it never invents a colour, it re-points the fill at a
//! token the document already defines.
//!
//! ## Why the judgement is contrast, never the variable name
//!
//! It is tempting to say "a text fill must not use `$color-surface`". White on
//! a dark board is correct and common — the shipped deck template's closing
//! slide does exactly that. The defect is the measured ratio, so that is what
//! is measured; the variable name is never consulted.

use crate::types::DocSink;

/// Palette tokens allowed as a replacement, most-preferred first.
///
/// All are emitted by `design_system` / `palette_harmonize`, so they exist in
/// any generated document. `color-surface` is included on purpose: on a dark
/// background it is the readable choice, and excluding it would leave dark
/// boards unrepairable.
/// These are the names `design_system` actually emits. An earlier version of
/// this list was written from memory (`color-text`, `color-text-strong`) and
/// matched NOTHING in a real document, so every repair silently no-opped —
/// the unit tests passed because their fixture used the invented names too.
const CANDIDATE_TOKENS: &[&str] = &[
    "color-text-primary",
    "color-text-body",
    "color-text-muted",
    "color-text-subtle",
    "color-surface",
    "color-bg-deep",
];

/// Contrast a repair must reach before it is worth making. Matches the
/// detector's own normal-text threshold rather than WCAG AA — raising the bar
/// is a separate decision with measured noise implications (see the contrast
/// threshold note), and this pass exists to fix invisible text, not to
/// relitigate the threshold.
const TARGET_RATIO: f64 = 2.0;

/// Repair invisible text under `root_id`. Returns how many fills were
/// re-pointed.
pub(crate) fn repair_text_contrast(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let Some(root) = sink
        .state()
        .active_children()
        .iter()
        .find(|node| op_editor_core::PenNodeExt::id_str(*node) == root_id)
    else {
        return 0;
    };
    let doc = document_for_lint(sink.state());
    let offenders = op_design_lint::detectors::typography::low_contrast_text(root, &doc);
    if offenders.is_empty() {
        return 0;
    }
    let variables = doc.variables.clone().unwrap_or_default();
    let theme = op_design_lint::node_util::default_theme(doc.themes.as_ref());

    let mut patches: Vec<(String, String)> = Vec::new();
    for offender in offenders {
        let Some(token) = best_token(&offender.bg_color, &variables, &theme) else {
            continue;
        };
        // Leave it alone when the palette has nothing better than what is
        // already there — a no-op patch would only add churn.
        patches.push((offender.node_id, token));
    }
    let applied = patches.len();
    for (node_id, token) in patches {
        sink.apply(op_editor_core::EditorCommand::PatchNodeData {
            node_id: op_editor_core::NodeId::new(&node_id),
            patch_json: format!(r#"{{"fill":[{{"type":"solid","color":"${token}"}}]}}"#),
            page_id: None,
        });
    }
    applied
}

/// The FIRST palette token, in preference order, that clears
/// [`TARGET_RATIO`] against `bg`.
///
/// Deliberately not "the highest contrast available": on a light board that
/// picks `color-bg-deep`, which is readable but semantically a background
/// token used as ink. Preference order encodes what the token MEANS, and the
/// ratio only decides whether it is usable — so ink wins on light boards and
/// the light tokens take over once ink stops being readable.
fn best_token(
    bg: &str,
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
) -> Option<String> {
    CANDIDATE_TOKENS.iter().find_map(|token| {
        let hex = token_hex(token, variables, theme)?;
        let ratio = op_design_lint::color::color_contrast(&hex, bg);
        (ratio.is_finite() && ratio >= TARGET_RATIO).then(|| (*token).to_string())
    })
}

/// Resolve one palette token to a hex string.
///
/// Goes through the lint crate's own resolver rather than reading the
/// variable's JSON: a shipped variable is a PER-THEME ARRAY
/// (`[{value:"#FFFFFF",theme:{Mode:Light}}, {value:"#1E293B",theme:{Mode:Dark}}]`),
/// and a hand-rolled `get("value")` returns `None` for every one of them —
/// which is exactly how the first version of this pass repaired nothing while
/// its tests passed against a single-value fixture.
fn token_hex(
    token: &str,
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
) -> Option<String> {
    let hex = op_design_lint::node_util::resolve_color_ref(&format!("${token}"), variables, theme)?;
    hex.starts_with('#').then_some(hex)
}

/// The lint crate reads a `PenDocument`; the orchestrator holds an
/// `EditorState`. Project just enough for the detector: its walk needs the
/// node tree plus the variable and theme tables.
fn document_for_lint(state: &op_editor_core::EditorState) -> jian_ops_schema::PenDocument {
    // Clone the document and swap in the ACTIVE page's nodes: the detector
    // walks `children`, and the orchestrator may be working on a page that is
    // not the document's first. Cloning rather than rebuilding field-by-field
    // keeps this correct when the schema grows a field.
    let mut doc = state.doc.clone();
    doc.children = state.active_children().to_vec();
    doc.pages = None;
    doc
}

#[cfg(test)]
#[path = "text_contrast_repair_tests.rs"]
mod text_contrast_repair_tests;
