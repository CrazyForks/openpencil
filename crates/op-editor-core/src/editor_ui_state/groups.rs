//! Grouped sub-states carved out of [`super::EditorUiState`]'s flat
//! field list.
//!
//! The struct is cloned wholesale on request paths and had grown to
//! ~190 flat fields; these are the clusters whose fields are only ever
//! read together, so folding each into a named struct (the same shape
//! `git_panel: GitPanelState` already had) shortens the field list
//! without changing any behavior.
//!
//! `EditorUiState` carries no `serde` derive and no snapshot
//! serialization reaches into these fields (settings persistence lives
//! in `op-editor-host-core::settings_payload`, which never names them),
//! so the regrouping has no wire / settings-format impact.

use super::{DesignMdRequest, PreviewDeviceKind};
use crate::design_md_button_state::DesignMdButton;

/// Canvas **Preview** (Play) mode state.
///
/// Entering Preview stops painting selection handles + editor chrome
/// and drives a live jian runtime host-side; the runtime itself is
/// `!Send`, so only these plain flags live on the wasm32-clean state.
/// `EditorUiState::{enter,exit,toggle}_preview` own the transitions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreviewState {
    /// Whether the canvas is in Preview (Play) mode. Entering does NOT
    /// mutate `doc`; exiting drops the runtime and leaves `doc`
    /// byte-identical. The TopBar Play/Stop button + `Esc` toggle it.
    pub mode: bool,
    /// Non-fatal warnings raised when the runtime was last built from
    /// `doc` for Preview — e.g. legacy role-frames promoted to widget
    /// nodes (`LegacyRolePromoted`). Surfaced for diagnostics; cleared
    /// on exit. Never serialized.
    pub warnings: Vec<String>,
    /// RESOLVED device-frame kind while previewing (`None` outside
    /// preview; never serialized). Three writers: `enter_preview`
    /// inference (host-side), the switcher, and the host's app-mode
    /// screen-switch re-inference — every re-inference writes back
    /// here so the switcher and the frame can never disagree.
    pub device: Option<PreviewDeviceKind>,
    /// Switcher segment under the cursor (hover wash).
    pub switcher_hover: Option<PreviewDeviceKind>,
    /// Switcher segment currently pressed (activates on RELEASE
    /// inside the same segment).
    pub switcher_pressed: Option<PreviewDeviceKind>,
    /// APP MODE screen-switcher pill under the cursor (hover wash),
    /// indexed into the session's current screen list. `None` outside
    /// hover and outside APP MODE. Never serialized.
    pub screen_switcher_hover: Option<usize>,
    /// APP MODE screen-switcher pill currently pressed (activates on
    /// RELEASE inside the same pill, mirroring [`Self::switcher_pressed`]).
    pub screen_switcher_pressed: Option<usize>,
}

/// PropertyPanel Size-section fill / hug / clip toggles.
///
/// Mirrors the same-named fields on the panel's own
/// `PropertyPanelSnapshot`; the panel derives them per frame from the
/// selected node, so these are the editor-level echo of that state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SizeToggleState {
    pub fill_width: bool,
    pub fill_height: bool,
    pub hug_width: bool,
    pub hug_height: bool,
    pub clip_content: bool,
}

/// Floating Design-MD panel state.
#[derive(Debug, Clone, PartialEq)]
pub struct DesignMdPanelState {
    /// Whether the floating Design-MD panel is shown.
    pub open: bool,
    /// Which design-md-panel button the cursor is over (close / import
    /// / export / remove / section header) — drives the hover wash.
    pub hover: Option<DesignMdButton>,
    /// Top-left corner of the panel in logical px. `None` until first
    /// opened — the host then centres it on the viewport.
    pub pos: Option<(f32, f32)>,
    /// Bitmask of expanded sections (bit 0 = theme, 1 = colors, 2 =
    /// typography, 3 = components, 4 = layout, 5 = notes). Defaults to
    /// theme + colors + typography expanded.
    pub expanded: u8,
    /// Vertical scroll offset (px) of the panel body.
    pub scroll: jian_core::scroll::ScrollState,
    /// True while the desktop host is waiting for an AI-generated
    /// design.md brief. Transient: never serialized.
    pub generating: bool,
    /// A queued import / export request — set by a panel click, drained
    /// by the desktop host (which owns the native file dialog).
    /// Transient: never serialized.
    pub request: Option<DesignMdRequest>,
}

impl Default for DesignMdPanelState {
    fn default() -> Self {
        Self {
            open: false,
            hover: None,
            pos: None,
            // theme + colors + typography expanded.
            expanded: 0b0000_0111,
            scroll: jian_core::scroll::ScrollState::default(),
            generating: false,
            request: None,
        }
    }
}
