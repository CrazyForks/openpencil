//! Bringing a shipped scene template into the document the user already has.
//!
//! The Asset Center's other door replaces the document; this one adds to it.
//! That difference is the whole module: a template that arrives as a new file
//! can bring its own variable table and start at the origin, while a template
//! landing beside existing work must not collide with either.
//!
//! Two hazards, both handled here rather than at the call site so the native
//! and web hosts cannot solve them differently:
//!
//! 1. **Variable collisions.** Every shipped template paints through the same
//!    seven names (`c-bg`, `c-accent`, …) with different values, so merging
//!    two templates' tables would silently restyle whichever arrived first.
//!    Each template's variables are namespaced by its id on the way in, and
//!    every `$ref` inside its boards is rewritten to match — so appending the
//!    same template twice is idempotent and appending two different ones
//!    leaves both looking like themselves.
//! 2. **Placement.** Boards keep their relative layout and move as a block to
//!    the right of whatever is already on the page.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::variable::VariableDefinition;

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;

/// Document-space gap between the existing content's right edge and the
/// first appended board. Roughly a fifth of a 1920 slide — wide enough that
/// the seam reads as "a different thing starts here" at fit-to-screen zoom.
pub const TEMPLATE_APPEND_GAP: f64 = 400.0;

/// A template's top-level boards plus the variables they reference.
///
/// Produced by [`template_boards`], consumed by
/// [`EditorState::append_template_boards`]. The two are separate so a caller
/// can parse without mutating — the parse is the step that can fail on a
/// malformed asset, and it must fail before anything touches the document.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateBoards {
    pub nodes: Vec<PenNode>,
    pub variables: BTreeMap<String, VariableDefinition>,
}

/// Parse a shipped template's `.op` source into namespaced boards.
///
/// `None` means the asset did not parse — the catalogue verifies every
/// template has a document, so a `None` here is a corrupt or renamed asset,
/// not a user error.
pub fn template_boards(source: &str, template_id: &str) -> Option<TemplateBoards> {
    let mut document: serde_json::Value = serde_json::from_str(source).ok()?;

    let renames = variable_renames(&document, template_id);
    if let Some(children) = document.get_mut("children") {
        rewrite_variable_refs(children, &renames);
    }

    let nodes: Vec<PenNode> = serde_json::from_value(document.get("children")?.clone()).ok()?;
    let variables = document
        .get("variables")
        .and_then(|v| {
            serde_json::from_value::<BTreeMap<String, VariableDefinition>>(v.clone()).ok()
        })
        .unwrap_or_default()
        .into_iter()
        .map(|(name, definition)| (renames.get(&name).cloned().unwrap_or(name), definition))
        .collect();

    Some(TemplateBoards { nodes, variables })
}

/// The namespaced name for each variable the template declares.
///
/// Namespacing unconditionally rather than only on collision is what makes
/// this idempotent: the second append of the same template produces the same
/// names with the same values, so the merge is a no-op instead of a rename
/// chain that grows a suffix every time.
fn variable_renames(document: &serde_json::Value, template_id: &str) -> BTreeMap<String, String> {
    let Some(variables) = document.get("variables").and_then(|v| v.as_object()) else {
        return BTreeMap::new();
    };
    variables
        .keys()
        .map(|name| (name.clone(), format!("{template_id}--{name}")))
        .collect()
}

/// Rewrite every `$name` reference under `value` through `renames`.
///
/// Works on the JSON rather than the typed tree because a reference can sit
/// in any string-valued field of any of the twelve node variants — fills,
/// strokes, text colour, sizing expressions — and a typed walk would have to
/// enumerate all of them and then keep up as the schema grows.
fn rewrite_variable_refs(value: &mut serde_json::Value, renames: &BTreeMap<String, String>) {
    if renames.is_empty() {
        return;
    }
    match value {
        serde_json::Value::String(text) => {
            if let Some(rewritten) = rewrite_refs_in_text(text, renames) {
                *text = rewritten;
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                rewrite_variable_refs(item, renames);
            }
        }
        serde_json::Value::Object(fields) => {
            for field in fields.values_mut() {
                rewrite_variable_refs(field, renames);
            }
        }
        _ => {}
    }
}

/// `None` when the text holds no reference this rename map knows.
///
/// The identifier after `$` is taken at maximal length and looked up whole,
/// so `$c-accent-soft` resolves to that variable rather than to `c-accent`
/// followed by a stray `-soft`. A `$` that is not followed by a known name —
/// a price in body copy, say — is left exactly as written.
fn rewrite_refs_in_text(text: &str, renames: &BTreeMap<String, String>) -> Option<String> {
    if !text.contains('$') {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut changed = false;
    let mut rest = text;
    while let Some(offset) = rest.find('$') {
        out.push_str(&rest[..offset]);
        let after = &rest[offset + 1..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(after.len());
        match renames.get(&after[..end]) {
            Some(renamed) => {
                out.push('$');
                out.push_str(renamed);
                changed = true;
            }
            None => {
                out.push('$');
                out.push_str(&after[..end]);
            }
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    changed.then_some(out)
}

impl EditorState {
    /// Append a template's boards to the active page, to the right of what
    /// is already there.
    ///
    /// One transaction, one undo entry: every board goes in through a single
    /// insert, which is also what keeps the empty-root swap from firing (it
    /// only ever considers a lone incoming frame). Returns whether the
    /// document changed; a rejected insert leaves the variable table alone,
    /// so a failure cannot deposit orphan variables.
    pub fn append_template_boards(&mut self, boards: TemplateBoards) -> bool {
        self.insert_template_boards(boards, false)
    }

    /// Bring a template in the way the user meant, given what is open.
    ///
    /// On an untouched starter the template takes the page over — keeping the
    /// blank frame beside it would leave the user to delete a placeholder
    /// before they could use what they asked for. Anywhere else it appends.
    ///
    /// This is the whole decision a host without a document loader needs; the
    /// desktop takes a longer road for the starter case because it also has a
    /// file path to unbind and preferences to carry across, neither of which
    /// exists here.
    pub fn adopt_template_boards(&mut self, boards: TemplateBoards) -> bool {
        let onto_starter = crate::blank_starter::active_page_is_blank_starter(self);
        self.insert_template_boards(boards, onto_starter)
    }

    /// One transaction either way: snapshot, place, insert, merge, commit.
    ///
    /// The order matters at one point — the insert runs before the variable
    /// merge, so a rejected insert cannot leave the document carrying a
    /// palette for boards that never arrived.
    fn insert_template_boards(&mut self, boards: TemplateBoards, clear_page: bool) -> bool {
        if boards.nodes.is_empty() {
            return false;
        }
        let snapshot = self.snapshot_for_history();

        let mut nodes = boards.nodes;
        if clear_page {
            self.active_children_mut().clear();
            self.deselect_all();
        } else {
            let (dx, dy) = append_offset(self.active_children(), &nodes);
            for node in &mut nodes {
                let base = node.base_mut();
                base.x = Some(base.x.unwrap_or(0.0) + dx);
                base.y = Some(base.y.unwrap_or(0.0) + dy);
            }
        }

        if self
            .insert_subtree_preserving_roots(nodes, &NodeId::NONE)
            .is_none()
        {
            // The page was already emptied on the `clear_page` road, so the
            // snapshot is the only way back — restoring it is what keeps a
            // failed adopt from being a delete.
            if clear_page {
                self.restore(snapshot);
            }
            return false;
        }

        let table = self.doc.variables.get_or_insert_with(BTreeMap::new);
        for (name, definition) in boards.variables {
            table.entry(name).or_insert(definition);
        }

        self.history_push_past(snapshot);
        true
    }
}

/// How far the incoming boards move so they sit right of `existing`.
///
/// Vertically they align with the top of the existing content rather than
/// keeping their own y: a deck authored at y=0 dropped beside work that
/// starts at y=900 would otherwise land off-screen above it.
fn append_offset(existing: &[PenNode], incoming: &[PenNode]) -> (f64, f64) {
    let Some((incoming_min_x, incoming_min_y, _, _)) = board_bounds(incoming) else {
        return (0.0, 0.0);
    };
    let Some((_, existing_min_y, existing_max_x, _)) = board_bounds(existing) else {
        return (0.0, 0.0);
    };
    (
        existing_max_x + TEMPLATE_APPEND_GAP - incoming_min_x,
        existing_min_y - incoming_min_y,
    )
}

/// `(min_x, min_y, max_x, max_y)` over top-level boards, from their authored
/// geometry rather than a layout pass — this crate has no layout engine, and
/// every page-level board carries an explicit position and size. A board
/// sized by its content contributes its origin only, which is the honest
/// answer when its width is not knowable here.
fn board_bounds(nodes: &[PenNode]) -> Option<(f64, f64, f64, f64)> {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    for node in nodes {
        let x = node.base().x.unwrap_or(0.0);
        let y = node.base().y.unwrap_or(0.0);
        let right = x + node.width_px().unwrap_or(0.0);
        let bottom = y + node.height_px().unwrap_or(0.0);
        bounds = Some(match bounds {
            None => (x, y, right, bottom),
            Some((min_x, min_y, max_x, max_y)) => (
                min_x.min(x),
                min_y.min(y),
                max_x.max(right),
                max_y.max(bottom),
            ),
        });
    }
    bounds
}

#[cfg(test)]
#[path = "scene_template_append_tests.rs"]
mod scene_template_append_tests;
