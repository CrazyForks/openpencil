//! Generation-skill resolution and the style-guide instruction block
//! (including the resolved-style public helper).

use super::*;

/// Resolve the generation-phase skill set against a message, returning the
/// full AgentContext (skills + load report). The report is held for B3b's
/// IntentMiss/BudgetExhausted merge.
pub(super) fn resolve_generation_skills(
    message: &str,
    opts: &op_ai_skills::ResolveOptions,
) -> op_ai_skills::AgentContext {
    op_ai_skills::resolve_skills(op_ai_skills::Phase::Generation, message, opts)
}

/// 该 plan 是否代表一整屏移动端页面。
///
/// Port of `computeIsMobileFullScreen` (orchestrator-plan-classify.ts:41-58):
/// 窄(≤480)且高(≥480)即整屏;窄而高度为 0/auto 时用 subtask 数 ≥2
/// 区分"整屏多区块页面"与单卡片 Type 0 组件。TS 的 WeakMap memo 是为了
/// 跨 status-bar strip 保持一致——Rust 在 strip 之后的最终 plan 上直接算,
/// 无需 memo。
pub(super) fn is_mobile_full_screen(plan: &OrchestratorPlan) -> bool {
    if plan.root_frame.width > 480.0 {
        return false;
    }
    if plan.root_frame.height >= 480.0 {
        return true;
    }
    plan.subtasks.len() >= 2
}

/// Build the sub-agent style-guide instruction block for the planner-selected
/// guide. Port of `buildSubAgentStyleGuideInstruction`
/// (orchestrator-sub-agent-compact.ts:78-124).
///
/// RUST ADAPTATION: TS emits `$color-*` refs (which it seeds into
/// `doc.variables`); Rust does NOT seed style-guide vars, so refs wouldn't
/// resolve — we emit the guide's concrete HEX values instead. Same effect:
/// the sub-agent uses the selected palette rather than inventing one.
/// Returns `None` when no guide name is set or it isn't in the registry.
pub(super) fn build_style_guide_instruction(
    style_guide_name: Option<&str>,
    tier: ModelTier,
) -> Option<String> {
    let name = style_guide_name?;
    let opts = SelectOptions {
        name: Some(name.to_string()),
        ..Default::default()
    };
    let guide = select_style_guide(style_guide_registry(), &opts)?;
    let v = extract_style_guide_values(&guide.content);

    let color_line = |label: &str, hex: &Option<String>| -> Option<String> {
        hex.as_ref().map(|h| format!("- {label}: {h}"))
    };
    let colors: Vec<String> = [
        color_line("Background", &v.colors.background),
        color_line("Surface", &v.colors.surface),
        color_line("Accent", &v.colors.accent),
        color_line("Text", &v.colors.text_primary),
        color_line("Secondary text", &v.colors.text_secondary),
        color_line("Muted text", &v.colors.text_muted),
        color_line("Border", &v.colors.border),
    ]
    .into_iter()
    .flatten()
    .collect();

    // Full tier: the whole guide + an exact-hex palette appendix.
    if tier == ModelTier::Full {
        let mut s = format!(
            "VISUAL STYLE GUIDE (follow these specifications exactly):\n{}",
            guide.content.trim()
        );
        if !colors.is_empty() {
            s.push_str(
                "\n\nPALETTE — use these EXACT hex colors, do NOT invent a conflicting palette:\n",
            );
            s.push_str(&colors.join("\n"));
        }
        return Some(s);
    }

    // Standard/Basic: a compact summary.
    let mut lines = vec![format!("VISUAL STYLE GUIDE SUMMARY ({name}):")];
    let tags: Vec<String> = guide.tags.iter().take(6).cloned().collect();
    if !tags.is_empty() {
        lines.push(format!("- Tags: {}", tags.join(", ")));
    }
    lines.extend(colors);
    if let Some(f) = &v.typography.display_font {
        lines.push(format!("- Heading font: {f}"));
    }
    if let Some(f) = &v.typography.body_font {
        lines.push(format!("- Body font: {f}"));
    }
    if let Some(r) = v.radius.card {
        lines.push(format!("- Card radius: {r}"));
    }
    if let Some(r) = v.radius.button {
        lines.push(format!("- Button radius: {r}"));
    }
    lines.push(
        "Use these EXACT hex colors in your fills — do not invent a conflicting palette.".into(),
    );
    Some(lines.join("\n"))
}

pub fn build_resolved_style_instruction(
    name: &str,
    params: &op_ai_skills::resolve_style::StyleParams,
) -> Option<String> {
    let guide = match resolve_style(name, params) {
        ResolveOutcome::Hit(guide) => guide,
        ResolveOutcome::Miss { .. } => return None,
    };
    let tokens = &guide.tokens;

    let mut lines = vec![
        format!(
            "RESOLVED STYLE REFERENCE ({} / {})",
            name.trim(),
            params.color_palette.trim()
        ),
        "Bake these reference values directly into node fills, text colors, borders, radii, and font fields. Do NOT create document variables. Do NOT call set_variables.".to_string(),
    ];
    push_resolved_string_tokens(&mut lines, "surface", &tokens.surface);
    push_resolved_string_tokens(&mut lines, "foreground", &tokens.foreground);
    push_resolved_string_tokens(&mut lines, "accent", &tokens.accent);
    push_resolved_string_tokens(&mut lines, "border", &tokens.border);
    for (role, value) in &tokens.rounded {
        lines.push(format!("rounded.{role}={}px", format_design_number(*value)));
    }
    lines.push(format!(
        "typography: headings={}, body={}, captions={}, data={}",
        tokens.typography.headings,
        tokens.typography.body,
        tokens.typography.captions,
        tokens.typography.data
    ));
    for (role, value) in &tokens.on {
        let role = if role.starts_with("on-") {
            role.to_string()
        } else {
            format!("on-{role}")
        };
        lines.push(format!("{role}={value}"));
    }

    Some(lines.join("\n"))
}

pub(super) fn push_resolved_string_tokens(
    lines: &mut Vec<String>,
    prefix: &str,
    values: &std::collections::BTreeMap<String, String>,
) {
    for (role, value) in values {
        lines.push(format!("{prefix}.{role}={value}"));
    }
}

pub(super) fn resolve_generation_skills_after_prompt_filter(
    intent: &str,
    opts: &ResolveOptions,
    tier: ModelTier,
    is_mobile_screen: bool,
    design_system_covered: bool,
    minimal_skills: bool,
    reduced_complexity: bool,
) -> (
    Vec<ResolvedSkill>,
    SkillLoadReport,
    Vec<(String, DropReason)>,
) {
    let total_budget = opts
        .budget_override
        .unwrap_or_else(|| Phase::Generation.default_budget());
    let phase_skills: Vec<op_ai_skills::SkillEntry> = get_skills_by_phase(Phase::Generation)
        .into_iter()
        .cloned()
        .collect();
    let matched = filter_by_intent(&phase_skills, intent, &opts.flags);

    let mut dropped: Vec<DroppedSkill> = phase_skills
        .iter()
        .filter(|candidate| !matched.iter().any(|m| m.meta.name == candidate.meta.name))
        .map(|candidate| DroppedSkill {
            name: candidate.meta.name.clone(),
            reason: DropReason::IntentMiss,
        })
        .collect();

    let mut dynamic = opts.dynamic_content.clone();
    dynamic
        .entry("recentHistory".to_string())
        .or_insert_with(|| "No recent history.".to_string());
    let injected: Vec<op_ai_skills::SkillEntry> = matched
        .into_iter()
        .map(|mut skill| {
            skill.content = inject_dynamic_content(&skill.content, &dynamic);
            skill
        })
        .collect();

    let (filtered_entries, filter_drops) = apply_skill_filter(
        injected,
        tier,
        is_mobile_screen,
        design_system_covered,
        minimal_skills,
        reduced_complexity,
    );
    // Honor caller-pinned skills (force-included, budget-exempt) — same
    // mechanism `resolve_skills` uses on the non-mobile path. Empty by default,
    // so a no-library mobile generation is unchanged.
    let pinned: Vec<&str> = opts.pinned_skills.iter().map(String::as_str).collect();
    let trimmed = trim_by_budget_pinned(&filtered_entries, total_budget, intent, &pinned);

    for entry in &filtered_entries {
        if !trimmed.iter().any(|kept| kept.meta.name == entry.meta.name) {
            dropped.push(DroppedSkill {
                name: entry.meta.name.clone(),
                reason: DropReason::BudgetExhausted,
            });
        }
    }

    let included: Vec<SkillLoadEntry> = trimmed
        .iter()
        .map(|skill| SkillLoadEntry {
            name: skill.meta.name.clone(),
            category: skill.meta.category,
            token_count: skill.token_count,
            truncated: skill.truncated,
        })
        .collect();
    let budget_used = included.iter().map(|entry| entry.token_count).sum();
    let report = SkillLoadReport {
        included,
        dropped,
        budget_used,
        budget_max: total_budget,
    };

    (trimmed, report, filter_drops)
}
