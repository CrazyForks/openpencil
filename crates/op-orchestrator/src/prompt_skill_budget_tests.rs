//! Budget guards for planning skills whose content is AUGMENTED at runtime.
//!
//! `op-ai-skills` has `no_skill_silently_exceeds_its_own_budget`, but that
//! test measures the file on disk — and `style-guide-selector.md` carries a
//! `{{availableStyleGuides}}` placeholder that the orchestrator substitutes
//! with the whole style-guide catalog before the trimmer ever sees it. The
//! static corpus was 210 tokens against a 500 budget; the injected catalog
//! took the real prompt to 819 tokens, and Step 1 of `trim_by_budget_pinned`
//! silently chopped 1287 chars off the tail — the selection rules that tell
//! the model what to DO with the catalog — on every planning call
//! (2026-07-28 production log).
//!
//! The corpus test cannot catch this: `op-ai-skills` is below the
//! orchestrator in the dependency graph and cannot see the catalog. So the
//! guard has to live here, on the side that owns the augmentation.

use std::collections::HashMap;

use jian_ops_schema::{DesignMdColor, DesignMdSpec, DesignMdTypography};
use op_ai_skills::budget::estimate_tokens;
use op_ai_skills::resolver::inject_dynamic_content;

use super::*;

/// Every placeholder the orchestrator substitutes into a PLANNING skill.
/// Adding a runtime-augmented planning key without adding it here is what
/// this guard exists to make impossible — a new key with no worst-case entry
/// leaves the same blind spot `{{availableStyleGuides}}` sat in.
const AUGMENTED_PLANNING_KEYS: [&str; 1] = ["availableStyleGuides"];

/// A design.md spec filled to the size limits `build_design_md_style_policy`
/// enforces (200/300/400-char truncations, 10 palette rows, 6 surface rows),
/// so the design.md branch is measured at ITS worst case too — that branch
/// interpolates USER content, and a fixed budget has to cover it.
fn saturated_design_md() -> DesignMdSpec {
    let color = |i: usize| DesignMdColor {
        name: format!("Palette Color Number {i}"),
        hex: format!("#0000{i:02}"),
        role: format!("surface role {i} — card, panel and sidebar backgrounds"),
    };
    DesignMdSpec {
        raw: String::new(),
        project_name: Some("A Rather Long Project Name For Measurement".into()),
        visual_theme: Some("v".repeat(400)),
        color_palette: Some((0..20).map(color).collect()),
        typography: Some(DesignMdTypography {
            font_family: Some("f".repeat(120)),
            headings: Some("h".repeat(120)),
            body: Some("b".repeat(120)),
            scale: Some("s".repeat(400)),
        }),
        component_styles: Some("c".repeat(600)),
        layout_principles: Some("l".repeat(800)),
        generation_notes: Some("n".repeat(800)),
    }
}

/// The worst-case value of `{{availableStyleGuides}}` over every branch that
/// can produce one: both catalog planning modes crossed with every model tier
/// (the tier sets the snippet count), plus the design.md branch.
fn worst_case_style_guide_context() -> (String, String) {
    // One model id per tier — `snippet_limit` is the only tier-sensitive
    // input, and it is what makes the catalog branch grow.
    let models = ["claude-opus", "glm-4", "minimax-m3", ""];
    let prompts = [
        "a fintech dashboard",
        "a dark minimalist mobile music app landing page",
        "xyz123",
    ];
    let design_md = saturated_design_md();
    let mut worst = (String::new(), String::new());
    for mode in [PlanningMode::Rich, PlanningMode::Minimal] {
        for model in models {
            for prompt in prompts {
                for spec in [None, Some(&design_md)] {
                    let ctx = build_planning_style_guide_context(prompt, Some(model), mode, spec, None);
                    if ctx.available_style_guides.chars().count() > worst.0.chars().count() {
                        let label = format!(
                            "mode={mode:?} model={model:?} design_md={} prompt={prompt:?}",
                            spec.is_some()
                        );
                        worst = (ctx.available_style_guides, label);
                    }
                }
            }
        }
    }
    worst
}

#[test]
fn runtime_augmented_planning_skills_fit_their_own_budget() {
    let (context, label) = worst_case_style_guide_context();
    let dynamic = HashMap::from([("availableStyleGuides".to_string(), context.clone())]);

    let mut offenders = Vec::new();
    for skill in op_ai_skills::get_skills_by_phase(op_ai_skills::Phase::Planning) {
        let augmented = inject_dynamic_content(&skill.content, &dynamic);
        if augmented == skill.content {
            continue; // no placeholder — the corpus test already covers it
        }
        let actual = estimate_tokens(&augmented);
        if actual > skill.meta.budget {
            offenders.push(format!(
                "{} (budget={}, augmented={actual}, over by {}) — worst case {label}",
                skill.meta.name,
                skill.meta.budget,
                actual - skill.meta.budget
            ));
        }
    }
    assert!(
        offenders.is_empty(),
        "planning skills are truncated AFTER runtime augmentation, silently dropping \
         their tail from every planning prompt: {offenders:?}"
    );
}

#[test]
fn every_augmented_placeholder_has_a_worst_case_in_this_guard() {
    // The guard is only as good as its key list. Any `{{placeholder}}` a
    // planning skill declares must be one this file measures — otherwise a
    // new augmented key reopens exactly the hole this file closes.
    for skill in op_ai_skills::get_skills_by_phase(op_ai_skills::Phase::Planning) {
        for (index, _) in skill.content.match_indices("{{") {
            let rest = &skill.content[index + 2..];
            let Some(end) = rest.find("}}") else { continue };
            let key = &rest[..end];
            assert!(
                AUGMENTED_PLANNING_KEYS.contains(&key),
                "planning skill {:?} interpolates {{{{{key}}}}}, which this budget guard \
                 does not measure — add its worst case to AUGMENTED_PLANNING_KEYS",
                skill.meta.name
            );
        }
    }
}

#[test]
fn no_planning_skill_is_dropped_or_truncated_by_the_phase_budget() {
    // End-to-end through the real resolver: the per-skill cap AND the phase
    // total. The landing-page prompt is deliberate — `landing-page-predesign`
    // is the phase's only Domain skill, so it is the one that gets squeezed
    // out when the base skills eat the whole total.
    let (context, label) = worst_case_style_guide_context();
    let opts = op_ai_skills::ResolveOptions {
        dynamic_content: HashMap::from([("availableStyleGuides".to_string(), context)]),
        ..Default::default()
    };
    let prompt = "a marketing landing page for a fintech product";
    let ctx = op_ai_skills::resolve_skills(op_ai_skills::Phase::Planning, prompt, &opts);

    let truncated: Vec<&str> = ctx
        .report
        .included
        .iter()
        .filter(|entry| entry.truncated)
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(
        truncated.is_empty(),
        "truncated planning skills {truncated:?} (worst case {label}); \
         used {}/{} tokens",
        ctx.report.budget_used,
        ctx.report.budget_max
    );

    let starved: Vec<&str> = ctx
        .report
        .dropped
        .iter()
        .filter(|entry| entry.reason == op_ai_skills::DropReason::BudgetExhausted)
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(
        starved.is_empty(),
        "planning skills dropped for budget {starved:?}; used {}/{} tokens — the phase \
         total must cover the base skills (which are budget-exempt and therefore always \
         counted) plus every intent-matched Domain skill",
        ctx.report.budget_used,
        ctx.report.budget_max
    );
}
