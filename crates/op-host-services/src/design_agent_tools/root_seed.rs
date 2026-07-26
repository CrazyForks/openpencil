//! Root-seed guard: decide whether a design turn's first batch needs a
//! synthetic root frame (mobile vs desktop), seed it, and keep the mobile
//! status bar canonical + de-duplicated. Split out of
//! `design_agent_tools.rs` to keep the spine under the 800-line cap.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootSeedTarget {
    Mobile,
    Desktop,
}

impl RootSeedTarget {
    fn from_mobile(mobile: bool) -> Self {
        if mobile {
            Self::Mobile
        } else {
            Self::Desktop
        }
    }

    fn dimensions(self) -> (f64, f64) {
        match self {
            Self::Mobile => (390.0, 844.0),
            Self::Desktop => (1440.0, 900.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RootSeedGuard {
    target: RootSeedTarget,
    consumed: bool,
}

impl RootSeedGuard {
    pub fn from_prompt(prompt: &str) -> Self {
        Self {
            target: root_seed_target_for_prompt(prompt),
            consumed: false,
        }
    }

    pub fn disabled() -> Self {
        Self {
            target: RootSeedTarget::Desktop,
            consumed: true,
        }
    }

    fn pending_target(&self) -> Option<RootSeedTarget> {
        (!self.consumed).then_some(self.target)
    }

    fn mark_consumed(&mut self) {
        self.consumed = true;
    }
}

pub fn root_seed_target_for_prompt(prompt: &str) -> RootSeedTarget {
    RootSeedTarget::from_mobile(root_seed_prompt_is_mobile(prompt))
}

pub fn root_seed_prompt_is_mobile(prompt: &str) -> bool {
    if prompt.contains("手机") {
        return true;
    }
    let lower = prompt.to_ascii_lowercase();
    // "web app" / "webapp" name desktop products — a dashboard "web app"
    // must not be seeded 390x844. Strip the desktop-ish phrases before the
    // word scan so the bare "app" signal only fires for actual mobile asks.
    if lower.contains("web app") || lower.contains("webapp") || lower.contains("desktop") {
        let desktopish = lower.replace("web app", " ").replace("webapp", " ");
        return desktopish
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| matches!(word, "mobile" | "phone" | "ios" | "android"));
    }
    lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| matches!(word, "mobile" | "app" | "phone" | "ios" | "android"))
}

pub(super) fn should_track_root_seed_candidate(
    _state: &EditorState,
    name: &str,
    indicator_epoch: Option<u64>,
    root_seed_guard: Option<&RootSeedGuard>,
) -> bool {
    if name != "batch_design" {
        return false;
    }
    root_seed_guard
        .and_then(RootSeedGuard::pending_target)
        .is_some()
        || indicator_epoch
            .and_then(op_editor_core::agent_indicators::root_seed_hint_if_pending)
            .is_some()
}

pub(super) fn collect_active_top_level_node_ids(state: &EditorState) -> HashSet<String> {
    state
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect()
}

pub(super) fn maybe_apply_root_seed_guard(
    state: &mut EditorState,
    ids_before: &HashSet<String>,
    indicator_epoch: Option<u64>,
    root_seed_guard: Option<&mut RootSeedGuard>,
) -> Option<String> {
    let explicit_target = root_seed_guard
        .as_ref()
        .and_then(|guard| guard.pending_target());
    let epoch_target = indicator_epoch
        .and_then(op_editor_core::agent_indicators::root_seed_hint_if_pending)
        .map(RootSeedTarget::from_mobile);
    let target = explicit_target.or(epoch_target)?;
    let seed_hint = seed_root_frame_if_needed(state, ids_before, target);
    // Mobile chrome parity with the orchestrator scaffold: the loop's first
    // batch gets the SAME pre-inserted status bar, so `mobile-app.md`'s
    // "status bar is already pre-inserted" contract holds on this path too.
    // Runs even when the model authored explicit root dimensions (the seed
    // above early-returns then, but the chrome must still land).
    let chrome_hint = (target == RootSeedTarget::Mobile)
        .then(|| inject_mobile_status_bar_if_missing(state, ids_before))
        .flatten();

    if let Some(guard) = root_seed_guard {
        guard.mark_consumed();
    } else if let Some(epoch) = indicator_epoch {
        op_editor_core::agent_indicators::mark_root_seed_guard_consumed(epoch);
    }

    match (seed_hint, chrome_hint) {
        (Some(seed), Some(chrome)) => Some(format!("{seed} {chrome}")),
        (Some(seed), None) => Some(seed),
        (None, Some(chrome)) => Some(chrome),
        (None, None) => None,
    }
}

/// Insert the standard iOS status-bar chrome as the mobile root's FIRST
/// child unless the model already built one. Reuses the orchestrator
/// scaffold's exact node tree (`scaffold::mobile_status_bar_node`) so both
/// generation paths ship byte-identical chrome. Returns the model-facing
/// hint when the bar was injected.
pub(super) fn inject_mobile_status_bar_if_missing(
    state: &mut EditorState,
    ids_before: &HashSet<String>,
) -> Option<String> {
    let root = root_seed_candidate_mut(state, ids_before)?;
    // OS chrome has exactly one canonical form. A model-built status bar
    // (name matches, structure doesn't — no role, ad-hoc children) is
    // REPLACED in place rather than kept: every hand-rolled variant we
    // measured deviated visibly from the iOS reference (GLM-5.2 2026-07-11).
    let noncanonical_index = root
        .children()
        .into_iter()
        .flatten()
        .position(|child| is_status_bar_node(child) && !is_canonical_status_bar(child));
    if let Some(index) = noncanonical_index {
        let root_id = root.id_str().to_string();
        let fill_hex = op_editor_core::first_solid_fill_hex(root)
            .unwrap_or("#ffffff")
            .to_string();
        let width = root.width_px().unwrap_or(390.0);
        if let Ok(bar) =
            op_orchestrator::scaffold::mobile_status_bar_node(&root_id, &fill_hex, width)
        {
            if let Some(children) = root.children_mut() {
                children[index] = bar;
                return Some(
                    "The status bar you built was replaced with the standard iOS status bar                      (62px, role=status-bar) - do NOT rebuild or restyle it."
                        .to_string(),
                );
            }
        }
        return None;
    }
    if root
        .children()
        .into_iter()
        .flatten()
        .any(is_status_bar_node)
    {
        return None;
    }
    let root_id = root.id_str().to_string();
    let fill_hex = op_editor_core::first_solid_fill_hex(root)
        .unwrap_or("#ffffff")
        .to_string();
    let width = root.width_px().unwrap_or(390.0);
    let bar = op_orchestrator::scaffold::mobile_status_bar_node(&root_id, &fill_hex, width).ok()?;
    root.children_mut()?.insert(0, bar);
    Some(
        "A standard iOS status bar (62px, role=status-bar) was pre-inserted as the root's \
         first child - do NOT create another status bar; start your content below it."
            .to_string(),
    )
}

/// The injected/scaffold chrome shape: role tag + the Time/Levels pair.
/// Anything else that merely NAMES itself a status bar is a hand-rolled
/// variant slated for replacement.
pub(super) fn is_canonical_status_bar(node: &PenNode) -> bool {
    node.base().role.as_deref() == Some("status-bar")
        && node.children().is_some_and(|children| {
            children
                .iter()
                .any(|c| c.base().name.as_deref() == Some("Levels"))
        })
}

pub(super) fn is_status_bar_node(node: &PenNode) -> bool {
    if node.base().role.as_deref() == Some("status-bar") {
        return true;
    }
    node.base()
        .name
        .as_deref()
        .is_some_and(|name| name.to_ascii_lowercase().contains("status bar"))
}

/// Contract sweep, every batch: once a root carries the canonical status
/// bar as a direct child, any OTHER status-bar-looking node in that root is
/// removed on the spot. The first-batch injection hook can't see later
/// batches — measured (test0711-22 23:07 run): the model hand-built a
/// second "21:30" bar inside the Header several batches in, which then sat
/// on screen until finalize. OS chrome is a contract, so the duplicate is
/// removed deterministically rather than echoed.
pub(super) fn remove_nested_duplicate_status_bars(state: &mut EditorState) -> usize {
    let mut removed = 0;
    for root in state.active_children_mut() {
        let has_canonical = root
            .children()
            .into_iter()
            .flatten()
            .any(is_canonical_status_bar);
        if !has_canonical {
            continue;
        }
        fn prune(node: &mut PenNode, keep_canonical: bool, removed: &mut usize) {
            let Some(children) = node.children_mut() else {
                return;
            };
            children.retain(|child| {
                let duplicate = is_status_bar_node(child)
                    && !(keep_canonical && is_canonical_status_bar(child));
                if duplicate {
                    *removed += 1;
                }
                !duplicate
            });
            for child in children {
                prune(child, false, removed);
            }
        }
        prune(root, true, &mut removed);
    }
    removed
}

pub(super) fn seed_root_frame_if_needed(
    state: &mut EditorState,
    ids_before: &HashSet<String>,
    target: RootSeedTarget,
) -> Option<String> {
    let root = root_seed_candidate_mut(state, ids_before)?;
    let width_before = root.width_px();
    let height_before = root.height_px();
    if width_before.is_some() && height_before.is_some() {
        return None;
    }

    let (target_width, target_height) = target.dimensions();
    if width_before.is_none() {
        root.set_width_px(target_width);
    }
    if height_before.is_none() {
        root.set_height_px(target_height);
    }
    default_root_layout_to_vertical(root);

    let width = root.width_px().unwrap_or(target_width);
    let height = root.height_px().unwrap_or(target_height);
    Some(format!(
        "root seeded to {}x{} - grow height if content exceeds.",
        format_seed_dimension(width),
        format_seed_dimension(height)
    ))
}

pub(super) fn root_seed_candidate_mut<'a>(
    state: &'a mut EditorState,
    ids_before: &HashSet<String>,
) -> Option<&'a mut PenNode> {
    let roots = state.active_children_mut();
    let new_root_index = roots
        .iter()
        .position(|node| !ids_before.contains(node.id_str()) && matches!(node, PenNode::Frame(_)));
    if let Some(index) = new_root_index {
        return roots.get_mut(index);
    }
    if roots.len() == 1 && matches!(roots[0], PenNode::Frame(_)) {
        roots.get_mut(0)
    } else {
        None
    }
}

pub(super) fn default_root_layout_to_vertical(node: &mut PenNode) {
    if let PenNode::Frame(frame) = node {
        if frame.container.layout.is_none() {
            frame.container.layout = Some(LayoutMode::Vertical);
        }
    }
}

pub(super) fn format_seed_dimension(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value:.1}")
    }
}
