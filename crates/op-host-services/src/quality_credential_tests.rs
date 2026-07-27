use super::{quality_credential_line, quality_summary_from_repairs};
use op_ai::chat_provider::QualitySummary;
use op_orchestrator::{CheckCategory, RepairSummary};

fn summary(checks: &[&str], repairs: &[(&str, usize)]) -> QualitySummary {
    QualitySummary {
        checks: checks.iter().map(|c| c.to_string()).collect(),
        repairs: repairs.iter().map(|(c, n)| (c.to_string(), *n)).collect(),
    }
}

#[test]
fn nothing_checked_renders_no_credential() {
    assert_eq!(
        quality_credential_line(&QualitySummary::default(), Some(0)),
        None,
        "a turn that never ran the passes must not be credited with checking"
    );
}

#[test]
fn clean_run_still_earns_a_positive_credential() {
    let line =
        quality_credential_line(&summary(&["layout", "overflow", "hierarchy"], &[]), Some(0))
            .expect("checks ran, so a credential is owed");

    assert_eq!(
        line,
        "\n\n• Checked layout, overflow, hierarchy — nothing needed fixing, no issues left"
    );
    assert!(
        !line.contains("▸ repairs"),
        "no repairs means no breakdown sub-line"
    );
}

#[test]
fn repairs_are_reported_with_a_per_check_breakdown() {
    let line = quality_credential_line(
        &summary(
            &["layout", "overflow", "hierarchy", "structure"],
            &[("layout", 2), ("overflow", 1), ("structure", 3)],
        ),
        Some(0),
    )
    .expect("credential owed");

    assert_eq!(
        line,
        "\n\n• Checked layout, overflow, hierarchy, structure — 6 auto-repair(s) applied, \
         no issues left\n  ▸ repairs: layout 2, overflow 1, structure 3"
    );
}

#[test]
fn leftover_issues_are_stated_not_papered_over() {
    let line = quality_credential_line(&summary(&["layout"], &[("layout", 1)]), Some(2))
        .expect("credential owed");

    assert!(
        line.contains("2 issue(s) still open"),
        "unresolved work must survive into the credential: {line}"
    );
    assert!(
        !line.contains("no issues left"),
        "must not claim a clean finish while issues remain: {line}"
    );
}

#[test]
fn unknown_remaining_omits_the_clause_entirely() {
    let line = quality_credential_line(&summary(&["structure"], &[("structure", 4)]), None)
        .expect("credential owed");

    assert_eq!(
        line, "\n\n• Checked structure — 4 auto-repair(s) applied\n  ▸ repairs: structure 4",
        "a caller that cannot know the leftover count must not imply there is none"
    );
}

#[test]
fn repair_summary_converts_preserving_checked_and_repaired_split() {
    let mut repairs = RepairSummary::default();
    repairs.record(CheckCategory::Layout, 2);
    repairs.record(CheckCategory::Overflow, 0);
    repairs.record(CheckCategory::Structure, 1);

    let wire = quality_summary_from_repairs(&repairs);

    assert_eq!(wire.checks, vec!["layout", "overflow", "structure"]);
    assert_eq!(
        wire.repairs,
        vec![("layout".to_string(), 2), ("structure".to_string(), 1)],
        "checked-but-clean categories are listed as checked, not as repairs"
    );
    assert_eq!(wire.total_repairs(), 3);
}
