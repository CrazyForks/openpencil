//! Agents-tab hero block — the headline plus the two muted lines that
//! open the tab. Only the copy is specific to this tab, so the painting
//! itself goes through the modal's shared
//! [`crate::widgets::agent_settings_rows::paint_tab_hero`].

use super::*;
use op_editor_core::agent_settings_builtin_presets::{
    BuiltinAgentPresetKey, BUILTIN_AGENT_PRESETS,
};

/// Provider roll shown under the headline. Built from the shipped preset
/// table so the line can never drift from what the product actually
/// supports.
fn provider_roll() -> String {
    BUILTIN_AGENT_PRESETS
        .iter()
        .filter(|preset| preset.key != BuiltinAgentPresetKey::Custom)
        .map(|preset| preset.display_name)
        .collect::<Vec<_>>()
        .join(" · ")
}

pub(super) fn paint_agents_hero(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    content: Rect,
) {
    let roll = provider_roll();
    crate::widgets::agent_settings_rows::paint_tab_hero(
        cx,
        theme,
        content,
        t_settings(ui, "settings.agents.heroTitle"),
        &[&roll, t_settings(ui, "settings.agents.heroSubtitle")],
    );
}
