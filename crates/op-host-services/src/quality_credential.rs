//! User-facing rendering of the deterministic quality passes' tally — the
//! "trust receipt" a finished generation shows instead of leaving every
//! auto-repair in a log line.
//!
//! Both generation paths render through here so the sentence is identical:
//! the built-in agent loop (`chat_agent_loop::finalize_and_report`, from the
//! executor's [`QualitySummary`]) and the orchestrator
//! (`Progress::QualityChecked`, from `op_orchestrator::RepairSummary` via
//! [`quality_summary_from_repairs`]).
//!
//! Honesty contract, in the order it bites:
//!
//! 1. **No checks, no credential.** An empty summary means the passes never
//!    ran (plain chat turn, executor without a live document, a host build
//!    that doesn't report). [`quality_credential_line`] returns `None` — it
//!    never degrades into "0 problems found", which would vouch for work
//!    nobody did.
//! 2. **Only what ran gets listed.** The check names come from the categories
//!    that actually reached a checkpoint, never from a hardcoded roster.
//! 3. **The number is what was counted.** One repair = one document edit a
//!    quality pass applied (see `op_orchestrator::repair_summary`), so the
//!    wording says "auto-repair(s) applied", not "problems found".
//! 4. **Leftovers are stated, not buried.** When the caller knows how many
//!    issues are still open (the loop does: unfilled screens + unresolved
//!    blockers), the line says so plainly. When it genuinely does not yet
//!    know (the orchestrator, whose promise-delivery check runs later), the
//!    clause is omitted rather than optimistically filled in with "none".
//!
//! Text is English and hardcoded, matching every other progress/report line
//! on these paths (`report_unfilled_if_any`, `report_blockers_if_any`,
//! `web_chat_standard::progress_label`); `op-i18n` is not involved in this
//! family of diagnostic lines today.

use op_ai::chat_provider::QualitySummary;

/// Convert the orchestrator's in-crate tally into the transport-free wire
/// shape the renderer takes. Keeps the display order the summary already
/// guarantees.
pub fn quality_summary_from_repairs(summary: &op_orchestrator::RepairSummary) -> QualitySummary {
    QualitySummary {
        checks: summary
            .checked()
            .into_iter()
            .map(|c| c.key().to_string())
            .collect(),
        repairs: summary
            .repaired()
            .into_iter()
            .map(|(check, count)| (check.key().to_string(), count))
            .collect(),
    }
}

/// Render the credential, or `None` when nothing was checked.
///
/// `remaining` is how many issues are still open after the passes ran:
/// `Some(0)` earns the "no issues left" clause, `Some(n)` states the count,
/// and `None` omits the clause entirely for a caller that cannot yet know.
///
/// The returned string is a transcript fragment — it carries its own leading
/// blank line, like the other `• …` report lines it is appended after.
pub fn quality_credential_line(
    quality: &QualitySummary,
    remaining: Option<usize>,
) -> Option<String> {
    if !quality.ran() {
        return None;
    }
    let repairs = quality.total_repairs();
    let repair_clause = if repairs == 0 {
        "nothing needed fixing".to_string()
    } else {
        format!("{repairs} auto-repair(s) applied")
    };
    let remaining_clause = match remaining {
        None => String::new(),
        Some(0) => ", no issues left".to_string(),
        Some(n) => format!(", {n} issue(s) still open"),
    };
    let mut line = format!(
        "\n\n• Checked {} — {repair_clause}{remaining_clause}",
        quality.checks.join(", ")
    );
    if repairs > 0 {
        let breakdown: Vec<String> = quality
            .repairs
            .iter()
            .map(|(check, count)| format!("{check} {count}"))
            .collect();
        line.push_str(&format!("\n  ▸ repairs: {}", breakdown.join(", ")));
    }
    Some(line)
}

#[cfg(test)]
#[path = "quality_credential_tests.rs"]
mod tests;
