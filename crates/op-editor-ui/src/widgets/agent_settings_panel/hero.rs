//! Agents-tab hero block — the headline plus the two muted lines that
//! open the tab. Only the copy is specific to this tab, so the painting
//! itself goes through the modal's shared
//! [`crate::widgets::agent_settings_rows::paint_tab_hero`].

use super::*;
use op_editor_core::agent_settings_builtin_presets::{
    BuiltinAgentPresetKey, BUILTIN_AGENT_PRESETS,
};

/// How many provider names the roll spells out before summarising.
const ROLL_NAMED: usize = 7;

/// Provider roll shown under the headline. Built from the shipped preset
/// table so the line can never drift from what the product actually
/// supports.
///
/// Only the first few are spelled out, with a count for the rest. The
/// full list is already eighteen names and grows with every provider we
/// add: relying on the fitter to ellipsize it means the line's last word
/// is always a severed one, and which name gets cut changes with the
/// viewport. A named prefix plus "and N more" stays a sentence.
fn provider_roll(ui: &EditorUiState) -> String {
    let names: Vec<&str> = BUILTIN_AGENT_PRESETS
        .iter()
        .filter(|preset| preset.key != BuiltinAgentPresetKey::Custom)
        .map(|preset| preset.display_name)
        .collect();
    let shown = names.len().min(ROLL_NAMED);
    let roll = names[..shown].join(" · ");
    let rest = names.len() - shown;
    if rest == 0 {
        return roll;
    }
    format!(
        "{roll} · {}",
        t_settings(ui, "settings.agents.providerRollMore").replace("{{count}}", &rest.to_string())
    )
}

pub(super) fn paint_agents_hero(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    content: Rect,
) {
    let roll = provider_roll(ui);
    crate::widgets::agent_settings_rows::paint_tab_hero(
        cx,
        theme,
        content,
        t_settings(ui, "settings.agents.heroTitle"),
        &[&roll, t_settings(ui, "settings.agents.heroSubtitle")],
    );
}
