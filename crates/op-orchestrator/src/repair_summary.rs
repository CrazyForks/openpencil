//! Structured tally of what the deterministic quality passes checked and
//! repaired, so a finished generation can hand the user a truthful
//! "checked layout, overflow, hierarchy — 6 auto-repairs applied" credential
//! instead of leaving every repair buried in a `tracing` line nobody reads.
//!
//! **What one repair unit means.** Exactly one ACCEPTED `DocSink::apply` —
//! i.e. one document edit a quality pass issued and the sink took. Nothing
//! here interprets intent, guesses at "how many problems that was", or reads
//! pass names: the number is a raw count of edits the passes made, which is
//! the only quantity every pass produces uniformly (most of them return `()`
//! or a bare `bool`). The user-facing wording must stay matched to that
//! meaning — see `op_host_services::quality_credential`.
//!
//! **How the count is taken.** [`RepairCounter::wrap`] returns a
//! [`CountingSink`] that delegates every method to the real sink and bumps a
//! shared atomic on each accepted apply. The cleanup driver shadows its own
//! `sink` binding with that wrapper once, at the top, then calls
//! [`RepairCounter::checkpoint`] between contiguous groups of passes to
//! attribute the edits since the previous checkpoint to a [`CheckCategory`].
//! Pass bodies are untouched — this is deliberately a measurement layer, not
//! a refactor of ~40 repair passes.
//!
//! **Honesty rules baked in.** A category is recorded as *checked* by the
//! checkpoint itself, whether or not it repaired anything, because reaching
//! the checkpoint proves the detectors ran. A category nothing ever
//! checkpointed never appears. An empty [`RepairSummary`] (no checkpoint at
//! all — e.g. a plain chat turn that never touched the document) must render
//! as no credential at all rather than as "0 problems found".
//!
//! Semantic passes that rewrite the section forest in place (`role_infer` /
//! `role_post_pass` / `tree_heuristics`) never go through a `DocSink`, so
//! their edits are NOT counted; the categories they influence are still
//! covered by the sink-driven passes in `cleanup::run_cleanup_passes`
//! (`repair_overbold_text_hierarchy` for hierarchy, the geometry loop for
//! overflow, and so on).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId};

use crate::types::DocSink;

/// A family of quality checks, as named to the user. Declaration order is
/// display order (and the `Ord` derive that keeps [`RepairSummary`]'s maps
/// deterministic), chosen so the most legible families lead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CheckCategory {
    /// Sizing / spacing / distribution repairs — padding ownership, footer
    /// sinking, card-height equalization, table column gaps, root height.
    Layout,
    /// The geometry-validation loop: real resolved rects proving text or a
    /// frame overflows its container, a rail collapsed, a row is overfull.
    Overflow,
    /// Visual hierarchy repairs — over-bold text demotion, decorative stroke
    /// stripping.
    Hierarchy,
    /// Structural repairs — duplicate status bars / bottom navs, app-shell
    /// reshaping, flat table rows, empty decorated stubs, broken rings,
    /// cross-screen chrome unification.
    Structure,
    /// Color/theme repairs — split theme-polarity variables, light-mobile nav
    /// surfaces.
    Palette,
}

impl CheckCategory {
    /// Every category, in display order.
    pub const ALL: [CheckCategory; 5] = [
        CheckCategory::Layout,
        CheckCategory::Overflow,
        CheckCategory::Hierarchy,
        CheckCategory::Structure,
        CheckCategory::Palette,
    ];

    /// Stable wire/display key. Also the token the host serializes across the
    /// tool-channel ack, so it must not drift.
    pub fn key(self) -> &'static str {
        match self {
            CheckCategory::Layout => "layout",
            CheckCategory::Overflow => "overflow",
            CheckCategory::Hierarchy => "hierarchy",
            CheckCategory::Structure => "structure",
            CheckCategory::Palette => "palette",
        }
    }

    /// Inverse of [`Self::key`] — used when a summary comes back over a wire
    /// as strings. Unknown keys yield `None` rather than a guess.
    pub fn from_key(key: &str) -> Option<Self> {
        CheckCategory::ALL.into_iter().find(|c| c.key() == key)
    }
}

/// What the quality passes checked and how much they repaired, per category.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepairSummary {
    /// Category -> repairs applied. A key present with value 0 means the
    /// category was checked and found nothing to fix.
    counts: BTreeMap<CheckCategory, usize>,
}

impl RepairSummary {
    /// Record that `category`'s passes ran and applied `repairs` edits.
    /// Calling with `repairs == 0` still marks the category as checked.
    pub fn record(&mut self, category: CheckCategory, repairs: usize) {
        *self.counts.entry(category).or_insert(0) += repairs;
    }

    /// Fold another summary in — used when one run drives cleanup over
    /// several root batches (the orchestrator's append path).
    pub fn merge(&mut self, other: &RepairSummary) {
        for (category, count) in &other.counts {
            self.record(*category, *count);
        }
    }

    /// True when no category was ever checked: render NO credential rather
    /// than claiming a clean bill of health nobody verified.
    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    /// Categories whose passes actually ran, in display order.
    pub fn checked(&self) -> Vec<CheckCategory> {
        self.counts.keys().copied().collect()
    }

    /// Repairs applied under `category` (0 when checked-and-clean or absent).
    pub fn repairs_for(&self, category: CheckCategory) -> usize {
        self.counts.get(&category).copied().unwrap_or(0)
    }

    /// Categories that repaired something, paired with their counts, in
    /// display order. Clean categories are omitted.
    pub fn repaired(&self) -> Vec<(CheckCategory, usize)> {
        self.counts
            .iter()
            .filter(|(_, count)| **count > 0)
            .map(|(category, count)| (*category, *count))
            .collect()
    }

    /// Total edits applied across every category.
    pub fn total_repairs(&self) -> usize {
        self.counts.values().sum()
    }
}

/// Shared edit counter behind a [`CountingSink`]. Held by the cleanup driver
/// alongside — never inside — the wrapped sink, so a checkpoint can read the
/// tally while the sink is still mutably borrowed.
#[derive(Debug)]
pub struct RepairCounter {
    applied: Arc<AtomicUsize>,
    checkpointed: usize,
}

impl Default for RepairCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl RepairCounter {
    pub fn new() -> Self {
        Self {
            applied: Arc::new(AtomicUsize::new(0)),
            checkpointed: 0,
        }
    }

    /// Wrap `inner` so its accepted applies land in this counter.
    pub fn wrap<'a>(&self, inner: &'a mut dyn DocSink) -> CountingSink<'a> {
        CountingSink {
            inner,
            applied: self.applied.clone(),
        }
    }

    /// Attribute every edit since the previous checkpoint to `category`, and
    /// mark `category` as checked even when the delta is 0.
    pub fn checkpoint(&mut self, summary: &mut RepairSummary, category: CheckCategory) {
        let total = self.applied.load(Ordering::SeqCst);
        let delta = total.saturating_sub(self.checkpointed);
        self.checkpointed = total;
        summary.record(category, delta);
    }
}

/// [`DocSink`] decorator that counts accepted applies. Every method
/// delegates verbatim — including `insert_subtree_returning_root_ids`, which
/// MUST forward rather than fall through to the trait default, or an
/// immediate-apply sink's real remapped ids would be swallowed.
pub struct CountingSink<'a> {
    inner: &'a mut dyn DocSink,
    applied: Arc<AtomicUsize>,
}

impl DocSink for CountingSink<'_> {
    fn state(&self) -> &EditorState {
        self.inner.state()
    }

    fn apply(&mut self, cmd: EditorCommand) -> bool {
        let accepted = self.inner.apply(cmd);
        if accepted {
            self.applied.fetch_add(1, Ordering::SeqCst);
        }
        accepted
    }

    fn insert_subtree_returning_root_ids(
        &mut self,
        nodes: Vec<PenNode>,
        parent_id: &NodeId,
    ) -> Option<Vec<String>> {
        let out = self
            .inner
            .insert_subtree_returning_root_ids(nodes, parent_id);
        if out.is_some() {
            self.applied.fetch_add(1, Ordering::SeqCst);
        }
        out
    }

    fn begin_undo_batch(&mut self) {
        self.inner.begin_undo_batch();
    }

    fn end_undo_batch(&mut self) {
        self.inner.end_undo_batch();
    }
}

#[cfg(test)]
#[path = "repair_summary_tests.rs"]
mod tests;
