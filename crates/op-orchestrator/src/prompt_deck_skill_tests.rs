//! Deck-corpus delivery guards — "the corpus was edited" vs "the model saw it".
//!
//! Every assertion here reads the FINAL assembled `system_prompt`, not the
//! resolved skill list, because the two failure modes this file exists for are
//! both invisible upstream of assembly: a skill dropped for `BudgetExhausted`
//! (the phase total ran out) and a skill whose TAIL was chopped by the Step 3
//! knapsack. Both leave the corpus file on disk perfectly intact.
//!
//! Measured before these guards existed (2026-08-04): on the plain non-mobile
//! tier budgets a deck subtask resolved ~6200 tokens of always-kept Base skills
//! against a 5200 (Basic) / 6500 (Standard) ceiling, so `slides` was dropped
//! outright at Basic and cut to 271 of its tokens at Standard — and the
//! Basic-tier allow-set in `compact_skills` then dropped `slides` a second time
//! regardless of budget. A weak model asked for a PPT received zero slide
//! guidance while `domains/slides.md` sat in the repo looking correct.

use super::*;
use crate::plan::{Region, RootFrameSpec};

/// A plan shaped like a real deck board: the fixed 1920x1080 projector
/// artboard `decomposition`'s type-4 branch mandates.
fn deck_plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "deck".into(),
            name: "Deck".into(),
            width: 1920.0,
            height: 1080.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn deck_subtask() -> crate::plan::Subtask {
    crate::plan::Subtask {
        id: "cover".into(),
        label: "封面".into(),
        region: Region {
            width: 1920.0,
            height: 1080.0,
        },
        id_prefix: "cover".into(),
        parent_frame_id: None,
        elements: None,
        screen: Some("封面".into()),
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn deck_request(model: &str) -> DesignRequest {
    DesignRequest {
        prompt: "帮我做一个 8 页的融资路演 PPT，深色科技感".into(),
        model: Some(model.into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
        pinned_style_guide: None,
    }
}

/// The last line of a skill's markdown body. Asserting on the TAIL is what
/// makes these guards non-vacuous: both truncation mechanisms cut from the
/// end, so a head-only assertion passes on a prompt that lost half the skill.
fn skill_tail(name: &str) -> &'static str {
    op_ai_skills::get_skill_by_name(name)
        .unwrap_or_else(|| panic!("{name} must be registered"))
        .content
        .trim_end()
        .lines()
        .last()
        .expect("skill body is non-empty")
}

/// Signature lines from the deck corpus that must reach the model verbatim.
/// Head, middle and tail of each file, so a partial delivery still fails.
const SLIDES_MARKERS: [&str; 5] = [
    "## Style tiers — pick ONE for the whole deck",
    "S2 DARK PITCH",
    "accent/surface 4.73 (FLOOR)",
    "## Route the tier from the request",
    "L09 Single KPI",
];

const DECK_PATTERNS_MARKERS: [&str; 5] = [
    "DECK PATTERNS — SLIDE SKELETONS",
    "y = (i / 3) * (1080 + 360)",
    "round((valueSize - unitSize) * 0.2)",
    "**The last row carries no stroke**",
    "a rectangle does not render its children",
];

/// Full / Standard / Basic all reach the model with the deck corpus whole.
/// One model id per tier — the tier is what selects the budget arm and the
/// `compact_skills` allow-set, and each of those dropped the deck skills on
/// its own before this was wired.
#[test]
fn every_tier_receives_the_deck_corpus_intact() {
    for model in ["claude-opus-5", "kimi-k2.5", "glm-4.6"] {
        let plan = deck_plan();
        let subtask = deck_subtask();
        let (call, report) = build_subagent_prompt(
            &subtask,
            &plan,
            &deck_request(model),
            AbortFlag::new(),
            false,
            false,
            &op_editor_core::ComponentLibrary::default(),
        );
        let prompt = &call.system_prompt;

        for marker in SLIDES_MARKERS.iter().chain(DECK_PATTERNS_MARKERS.iter()) {
            assert!(
                prompt.contains(marker),
                "model {model:?}: assembled system prompt is missing {marker:?} — \
                 the deck corpus was dropped or truncated before the model saw it. \
                 Loaded: {:?}; used {}/{} tokens",
                report
                    .included
                    .iter()
                    .map(|e| e.name.as_str())
                    .collect::<Vec<_>>(),
                report.budget_used,
                report.budget_max,
            );
        }

        // The tails specifically — the knapsack cuts from the end.
        for name in ["slides", "deck-patterns"] {
            assert!(
                prompt.contains(skill_tail(name)),
                "model {model:?}: {name} reached the prompt without its last line — \
                 it was tail-truncated by the phase budget ({}/{})",
                report.budget_used,
                report.budget_max,
            );
        }

        let truncated: Vec<&str> = report
            .included
            .iter()
            .filter(|e| e.truncated)
            .map(|e| e.name.as_str())
            .collect();
        assert!(
            truncated.is_empty(),
            "model {model:?}: truncated skills {truncated:?} ({}/{} tokens)",
            report.budget_used,
            report.budget_max,
        );
    }
}

/// The budget arm must be selected by the deck's ARTBOARD, not by the prompt
/// wording — a deck plan whose request never says "deck" still needs the room.
#[test]
fn the_deck_budget_arm_keys_off_the_projector_artboard() {
    assert!(is_deck_board(&deck_plan()));
    // A desktop page is 1200x0 (auto-height) and a mobile screen 375x812 —
    // neither may claim the deck budget.
    let mut page = deck_plan();
    page.root_frame.width = 1200.0;
    page.root_frame.height = 0.0;
    assert!(!is_deck_board(&page));
    let mut mobile = deck_plan();
    mobile.root_frame.width = 375.0;
    mobile.root_frame.height = 812.0;
    assert!(!is_deck_board(&mobile));
}

/// A non-deck plan must be byte-for-byte unaffected: the deck skills are
/// keyword-gated, so they must not appear on an ordinary page prompt, and the
/// tier budget for that page must stay where it was.
#[test]
fn a_non_deck_subtask_is_unchanged_by_the_deck_arm() {
    let (call, _report) = bsp(&subtask(), &plan(), &req(), AbortFlag::new(), false, false);
    assert!(!call
        .system_prompt
        .contains("DECK PATTERNS — SLIDE SKELETONS"));
    assert!(!call
        .system_prompt
        .contains("## Route the tier from the request"));
}

/// The reduced-complexity retry keeps the 16:9 contract. Without `slides` in
/// `RETRY_ALLOWED` a Basic-tier retry re-generates the board as a scrolling
/// page, which is the failure the retry was supposed to fix.
#[test]
fn the_basic_reduced_retry_keeps_the_slide_format_contract() {
    let (call, _report) = build_subagent_prompt(
        &deck_subtask(),
        &deck_plan(),
        &deck_request("glm-4.6"),
        AbortFlag::new(),
        true, // reduced_complexity
        false,
        &op_editor_core::ComponentLibrary::default(),
    );
    assert!(
        call.system_prompt
            .contains("Each slide is a 16:9 frame, 1920×1080"),
        "the reduced-complexity retry lost the slide format contract"
    );
}

/// Planning side: the type-4 outline templates and copy caps must survive
/// `decomposition`'s own per-skill budget AND the planning phase total.
#[test]
fn the_planning_prompt_carries_the_deck_outline_mode() {
    let request = DesignRequest {
        prompt: "帮我做一个 8 页的融资路演 PPT".into(),
        ..deck_request("claude-opus-5")
    };
    let pp = build_orchestrator_prompt(&request, PlanningMode::Rich, AbortFlag::new());
    let prompt = &pp.call_request.system_prompt;
    for marker in [
        "OUTLINE MODE",
        "Pitch / 路演 / 融资: cover - the problem (3 pains)",
        "Lecture / 课件 / 培训: cover - learning objectives",
        "COPY LIMITS",
        "slide title <= 14 CJK chars",
    ] {
        assert!(
            prompt.contains(marker),
            "planning prompt is missing {marker:?} — `decomposition` was truncated \
             by its own budget or by the planning phase total"
        );
    }
}
