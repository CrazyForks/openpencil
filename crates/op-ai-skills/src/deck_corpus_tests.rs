//! Corpus guards for the deck skills (`slides` + `deck-patterns`).
//!
//! Delivery into the assembled system prompt is guarded on the orchestrator
//! side (`prompt_deck_skill_tests`); this file guards the corpus itself —
//! registration, trigger wiring, and the tier/pattern content the deck
//! generation path depends on.

use crate::loader::{get_skill_by_name, get_skills_by_phase};
use crate::resolver::match_keyword;
use crate::types::{Phase, SkillCategory, SkillTrigger};

#[test]
fn deck_patterns_registers_as_a_keyword_gated_generation_domain_skill() {
    let skill = get_skill_by_name("deck-patterns").expect("deck-patterns must be registered");
    assert_eq!(skill.meta.category, SkillCategory::Domain);
    assert!(skill.meta.phase.contains(&Phase::Generation));
    // Priority below every other generation Domain skill that a deck prompt can
    // also trigger (`dashboard` 28, `web-app` 30, `landing-page` 35): the budget
    // filler walks Domain skills in ascending priority, so a deck prompt that
    // also says "数据"/"product" must spend its remaining tokens on the deck
    // teaching first. Measured: at the old priority 28 `slides` lost its tail —
    // or was dropped entirely — on "做一份季度数据汇报 PPT，要有仪表盘和数据表格".
    assert!(
        skill.meta.priority < 28,
        "deck-patterns must outrank the generic page domains, got {}",
        skill.meta.priority
    );
    match &skill.meta.trigger {
        SkillTrigger::Keywords(keywords) => {
            for word in ["deck", "ppt", "幻灯片", "课件", "路演"] {
                assert!(
                    keywords.iter().any(|k| k == word),
                    "deck-patterns must trigger on {word:?}"
                );
            }
        }
        other => panic!("deck-patterns must be keyword-gated, got {other:?}"),
    }
}

#[test]
fn a_presentation_deck_prompt_no_longer_pulls_the_stacked_card_worked_example() {
    // `shapes-and-decks` teaches concentric rings and STACKED CARD decks. Its
    // bare `deck` keyword also fired on every presentation deck, spending ~1000
    // tokens of the generation budget on a ring/card-stack worked example that
    // has nothing to do with slides — and those are tokens `slides` then could
    // not have. The narrowed keywords must still cover the real stacked-card
    // intents while leaving presentation decks alone.
    let shapes = get_skill_by_name("shapes-and-decks").expect("shapes-and-decks registered");
    let SkillTrigger::Keywords(keywords) = &shapes.meta.trigger else {
        panic!("shapes-and-decks must be keyword-gated");
    };
    let fires = |message: &str| {
        let lowered = message.to_lowercase();
        keywords
            .iter()
            .any(|k| match_keyword(&lowered, &k.to_lowercase()))
    };
    for slides_prompt in [
        "generate a 6-slide pitch deck about our launch",
        "帮我做一个融资路演 PPT",
        "a keynote deck for the all-hands",
    ] {
        assert!(
            !fires(slides_prompt),
            "the stacked-card worked example must not load for {slides_prompt:?}"
        );
    }
    for stack_prompt in [
        "a swipeable card deck of testimonials",
        "show a stacked card stack",
        "堆叠卡组",
        "an activity ring for the fitness screen",
    ] {
        assert!(
            fires(stack_prompt),
            "shapes-and-decks must still fire for {stack_prompt:?}"
        );
    }
}

#[test]
fn slides_teaches_four_routable_style_tiers_with_measured_contrast_floors() {
    let slides = get_skill_by_name("slides").expect("slides registered");
    let body = &slides.content;
    for tier in [
        "S1 WARM-WHITE BUSINESS",
        "S2 DARK PITCH",
        "S3 LIGHT LECTURE",
        "S4 MINIMAL KEYNOTE",
    ] {
        assert!(body.contains(tier), "slides must define {tier}");
    }
    // Every tier states the pair that fails first, so the model re-measures the
    // right one after swapping an accent. These ratios are copied from the
    // shipped template generators (`templates/step0/_generators/*.py`) — a tier
    // without a FLOOR line is a tier whose palette can be silently broken.
    assert_eq!(
        body.matches("(FLOOR)").count(),
        3,
        "S1/S2/S3 each declare their floor pair; S4 reuses S1/S2's ground"
    );
    for floor in [
        "muted/accent-soft 4.51 (FLOOR)", // S1, computed from tpl_slides.py's palette
        "accent/surface 4.73 (FLOOR)",    // S2, tpl_pitch_dark.py's own measurement
        "muted/accent-soft 5.01 (FLOOR)", // S3, tpl_lecture_light.py's own measurement
    ] {
        assert!(
            body.contains(floor),
            "slides must carry the measured {floor}"
        );
    }
    // Routing must be executable: each tier is reachable from words a user
    // actually types, in both languages.
    assert!(body.contains("## Route the tier from the request"));
    for word in ["深色", "极简", "课件", "minimal", "lecture", "dark"] {
        assert!(body.contains(word), "tier routing must cover {word:?}");
    }
    // Per-tier element caps — the rule that keeps a generated slide from
    // becoming a document.
    assert_eq!(
        body.matches("elements per slide").count(),
        4,
        "every tier must cap its element count"
    );
}

#[test]
fn deck_patterns_carries_the_structural_numbers_the_templates_proved() {
    let body = &get_skill_by_name("deck-patterns")
        .expect("deck-patterns registered")
        .content;
    for rule in [
        // Board placement — the "six slides stacked at the origin" bug.
        "x = (i % 3) * (1920 + 120)",
        "y = (i / 3) * (1080 + 360)",
        // KPI unit baseline compensation — there is no real baseline align.
        "round((valueSize - unitSize) * 0.2)",
        // Timeline axis continuity.
        "**gap 0**",
        // Table rules.
        "**The last row carries no stroke**",
        // Bullet-dot optical centring.
        "round(fontSize * lineHeight / 2 - dotSize / 2)",
        // Numbered circle digit size.
        "round(size*0.46)",
        // Placeholders must be frames, not rectangles.
        "a rectangle does not render its children",
    ] {
        assert!(body.contains(rule), "deck-patterns must teach {rule:?}");
    }
}

#[test]
fn decomposition_carries_the_deck_outline_templates_and_copy_caps() {
    let body = &get_skill_by_name("decomposition")
        .expect("decomposition registered")
        .content;
    assert!(body.contains("OUTLINE MODE"));
    for outline in [
        "Pitch / 路演 / 融资",
        "Lecture / 课件 / 培训",
        "Report / 汇报 / 季度",
        "Product launch / 发布",
    ] {
        assert!(body.contains(outline), "type-4 must plan {outline}");
    }
    assert!(body.contains("COPY LIMITS"));
    assert!(body.contains("slide title <= 14 CJK chars"));
}

#[test]
fn every_deck_prompt_resolves_both_deck_skills_untruncated() {
    // The generation-phase total is what actually decides this; these prompts
    // are the ones that used to lose `slides` (each pulls a second, larger
    // domain skill: dashboard / web-app / landing-page / mobile-app).
    for prompt in [
        "帮我做一个 8 页的融资路演 PPT，深色科技感",
        "做一份季度数据汇报 PPT，要有仪表盘和数据表格",
        "generate a 10-slide deck for our SaaS admin console product with analytics data tables",
        "极简 keynote 演示，讲移动端 app 的设计",
        "帮我做一个教学课件幻灯片，讲解表单设计和登录页",
        "pitch deck landing page marketing homepage slides",
    ] {
        let ctx =
            crate::resolve_skills(Phase::Generation, prompt, &crate::ResolveOptions::default());
        for name in ["slides", "deck-patterns"] {
            let entry = ctx
                .report
                .included
                .iter()
                .find(|e| e.name == name)
                .unwrap_or_else(|| {
                    panic!(
                        "{prompt:?} dropped {name}; kept {:?} at {}/{} tokens",
                        ctx.report
                            .included
                            .iter()
                            .map(|e| e.name.as_str())
                            .collect::<Vec<_>>(),
                        ctx.report.budget_used,
                        ctx.report.budget_max
                    )
                });
            assert!(
                !entry.truncated,
                "{prompt:?} tail-truncated {name} at {}/{} tokens",
                ctx.report.budget_used, ctx.report.budget_max
            );
        }
    }
}

#[test]
fn cjk_typography_outranks_the_large_optional_domains() {
    // 339 tokens of CJK correctness must never lose its slot to a 1800-token
    // optional skill. It did, once the deck skills joined the same queue:
    // "深色路演 deck，交互式原型演示" dropped `cjk-typography` for
    // `interactivity`. Priority is the only lever that fixes that ordering.
    let cjk = get_skill_by_name("cjk-typography").expect("cjk-typography registered");
    for name in [
        "interactivity",
        "dashboard",
        "web-app",
        "mobile-app",
        "slides",
    ] {
        let other = get_skill_by_name(name).unwrap_or_else(|| panic!("{name} registered"));
        assert!(
            cjk.meta.priority < other.meta.priority,
            "cjk-typography ({}) must fill before {name} ({})",
            cjk.meta.priority,
            other.meta.priority
        );
    }
    let ctx = crate::resolve_skills(
        Phase::Generation,
        "深色路演 deck，交互式原型演示，展示我们的 web app 控制台",
        &crate::ResolveOptions::default(),
    );
    assert!(
        ctx.report
            .included
            .iter()
            .any(|e| e.name == "cjk-typography" && !e.truncated),
        "a CJK deck prompt must keep cjk-typography; kept {:?}",
        ctx.report
            .included
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn the_deck_corpus_is_reachable_from_the_generation_phase_registry() {
    // Guards the loader's directory walk: a new file under `skills/domains/`
    // only becomes a skill if `include_dir` embedded it AND the frontmatter
    // parsed. A typo in either yields silence, not an error.
    let names: Vec<&str> = get_skills_by_phase(Phase::Generation)
        .iter()
        .map(|s| s.meta.name.as_str())
        .collect();
    assert!(names.contains(&"deck-patterns"), "got {names:?}");
    assert!(names.contains(&"slides"), "got {names:?}");
}
