//! Structured activities and progress steps — rows, ordering and collapse state.
//!
//! Split out of `ai_chat_transcript_tests.rs` to keep that file under the
//! 800-line cap.

use super::*;

#[test]
fn empty_design_progress_lines_do_not_render_inline_or_as_typing_placeholder() {
    let mut m = ChatMessage::assistant_streaming();
    m.thinking = "\n• Planning…\n• Scaffold ready".into();
    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert!(
        items[0].steps.is_empty(),
        "empty plan/progress rows belong to the fixed checklist, matching TS ActionSteps"
    );
    assert!(
        items[0].thinking.is_none(),
        "design progress should not render as a reasoning block"
    );
    assert!(
        items[0].bubble.is_none(),
        "fixed checklist progress should suppress the empty streaming typing placeholder"
    );
}

#[test]
fn structured_activities_render_as_compact_rows_without_thinking_text() {
    let mut message = ChatMessage::assistant_streaming();
    message.activities = vec![
        op_editor_core::ChatActivity {
            id: "header".into(),
            title: "Greeting header".into(),
            detail: None,
            status: op_editor_core::ChatActivityStatus::Running,
            content_offset: None,
        },
        op_editor_core::ChatActivity {
            id: "rail".into(),
            title: "Recently played".into(),
            detail: Some("12 elements".into()),
            status: op_editor_core::ChatActivityStatus::Done,
            content_offset: None,
        },
    ];
    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 2);
    assert_eq!(items[0].steps[0].label, "Greeting header");
    assert!(items[0].steps[0].active);
    assert_eq!(items[0].steps[0].rect.size.y, ACTION_STEP_H);
    assert_eq!(items[0].steps[1].label, "Recently played");
    assert!(items[0].steps[1].done);
    assert!(items[0].thinking.is_none());
    assert!(items[0].bubble.is_none());
}

#[test]
fn structured_activities_interleave_with_cli_narration_by_offset() {
    let first = "I mapped the screen.";
    let second = "The sections are in place.";
    let final_text = "Done — the layout has been checked.";
    let content = format!("{first}\n\n{second}\n\n{final_text}");
    let second_offset = first.len() + 2 + second.len();
    let mut message = ChatMessage::assistant(&content);
    message.activities = vec![
        op_editor_core::ChatActivity {
            id: "build".into(),
            title: "Building sections".into(),
            detail: None,
            status: op_editor_core::ChatActivityStatus::Done,
            content_offset: Some(first.len() as u32),
        },
        op_editor_core::ChatActivity {
            id: "check".into(),
            title: "Checking the design".into(),
            detail: None,
            status: op_editor_core::ChatActivityStatus::Done,
            content_offset: Some(second_offset as u32),
        },
    ];

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let item = &items[0];

    assert_eq!(item.steps.len(), 2);
    assert_eq!(item.flow_bubbles.len(), 3);
    assert!(item.bubble.is_none());
    assert!(item.flow_bubbles[0].rect.origin.y < item.steps[0].rect.origin.y);
    assert!(item.steps[0].rect.origin.y < item.flow_bubbles[1].rect.origin.y);
    assert!(item.flow_bubbles[1].rect.origin.y < item.steps[1].rect.origin.y);
    assert!(item.steps[1].rect.origin.y < item.flow_bubbles[2].rect.origin.y);
}

#[test]
fn legacy_and_interleaved_activity_steps_use_distinct_override_slots() {
    let mut message = ChatMessage::assistant("Narration");
    message.thinking = "• Legacy detail\n  ▸ diagnostic".into();
    message.activities.push(op_editor_core::ChatActivity {
        id: "build".into(),
        title: "Building section".into(),
        detail: Some("2 elements".into()),
        status: op_editor_core::ChatActivityStatus::Done,
        content_offset: Some(0),
    });
    message.action_step_expanded_overrides = vec![Some(false), Some(true)];

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 2);
    assert_eq!(items[0].steps[0].source_index, 0);
    assert_eq!(items[0].steps[1].source_index, 1);
    assert!(!items[0].steps[0].expanded);
    assert!(items[0].steps[1].expanded);
}

#[test]
fn detail_less_structured_activity_has_no_invisible_toggle_hit() {
    let mut message = ChatMessage::assistant_streaming();
    message.activities.push(op_editor_core::ChatActivity {
        id: "header".into(),
        title: "Greeting header".into(),
        detail: None,
        status: op_editor_core::ChatActivityStatus::Running,
        content_offset: None,
    });
    let body = body();
    let canonical = crate::widgets::ai_chat_transcript_cache::unowned_for_tests(
        std::slice::from_ref(&message),
        body,
        op_editor_core::Locale::EnUs,
    );
    let step = &canonical.items[0].steps[0];

    assert_eq!(
        transcript_hit(
            &canonical,
            body,
            step.rect.origin.x + 8.0,
            step.rect.origin.y + 8.0,
            0.0,
        ),
        None
    );
}

#[test]
fn current_step_with_content_is_active_until_terminal() {
    let mut m = ChatMessage::assistant_streaming();
    m.content = r#"<step title="Planning…">Drafting layout constraints</step>"#.into();
    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 1);
    assert_eq!(items[0].steps[0].label, "Planning…");
    assert!(items[0].steps[0].active);
    assert!(!items[0].steps[0].done);
}

#[test]
fn step_tag_content_renders_as_progress_not_raw_bubble() {
    let mut message = ChatMessage::assistant_streaming();
    message.content =
        r#"<step title="Checking guidelines" status="streaming">Analyzing request...</step>"#
            .into();

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 1);
    assert_eq!(items[0].steps[0].label, "Checking guidelines");
    assert!(items[0].steps[0].active);
    assert!(
        items[0].bubble.is_none(),
        "raw <step> markup should not render as answer text"
    );
}

#[test]
fn step_tag_content_surfaces_as_progress_details() {
    let mut message = ChatMessage::assistant_streaming();
    message.content = r#"<step title="Validate design" status="streaming">
lint: fixed spacing
render: captured frame
</step>"#
        .into();

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 1);
    assert_eq!(
        items[0].steps[0].details,
        vec![
            "lint: fixed spacing".to_string(),
            "render: captured frame".to_string()
        ]
    );
    assert!(
        items[0].steps[0].rect.size.y > 28.0,
        "step details should reserve space instead of being dropped"
    );
}

#[test]
fn completed_step_with_content_defaults_collapsed_like_ts_accordion() {
    let mut message = ChatMessage::assistant(
        r#"<step title="Validate design" status="done">
lint: fixed spacing
render: captured frame
</step>"#,
    );
    message.streaming = false;

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 1);
    assert!(items[0].steps[0].done);
    assert!(!items[0].steps[0].active);
    assert!(
        (items[0].steps[0].rect.size.y - ACTION_STEP_H).abs() < 1e-4,
        "TS ActionStepItem defaults completed accordions closed"
    );
}

#[test]
fn paint_completed_step_hides_details_like_collapsed_ts_accordion() {
    let message = ChatMessage::assistant(
        r#"<step title="Validate design" status="done">
lint: fixed spacing
render: captured frame
</step>"#,
    );
    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body(),
        &[message],
        0,
        op_editor_core::Locale::EnUs,
    );

    assert!(
        backend.texts.iter().any(|text| text == "Validate design"),
        "collapsed accordion still paints its title"
    );
    assert!(
        !backend
            .texts
            .iter()
            .any(|text| text.contains("lint: fixed spacing")
                || text.contains("render: captured frame")),
        "collapsed TS accordions hide details until opened"
    );
}
