//! Gateway + split-borrow coverage.
//!
//! These drive the *projection* into each phase directly rather than standing
//! up a real session: the gate reads only `phase` / `role` / `pending_edit`, so
//! a projected state exercises exactly the decision under test without a
//! network peer.

use op_editor_core::{
    AuthenticatedCollabSession, CollabConnectionPhase, CollabDocumentMutation, CollabEditSource,
    CollabGateAction, CollabPendingEditUi, CollabUiRole, EditorCommand, EditorState,
};

use super::super::WebCanvasState;

fn daemon() -> WebCanvasState {
    WebCanvasState::new(EditorState::starter(), 0)
}

fn in_session(state: &mut WebCanvasState, phase: CollabConnectionPhase, role: CollabUiRole) {
    state.editor.editor_ui.collab.set_authenticated_session(
        phase,
        AuthenticatedCollabSession {
            session_name: "studio".into(),
            role,
            share_endpoint: None,
        },
        Vec::new(),
    );
}

const DOCUMENT_EDIT: CollabGateAction =
    CollabGateAction::Document(CollabDocumentMutation::NodePropertyBatch);

#[test]
fn an_idle_daemon_passes_every_source_through_untouched() {
    let state = daemon();
    for source in [
        CollabEditSource::User,
        CollabEditSource::Ai,
        CollabEditSource::Mcp,
        CollabEditSource::Import,
        CollabEditSource::ExternalSync,
    ] {
        assert!(
            state.gate_daemon_mutation(DOCUMENT_EDIT, source).is_ok(),
            "{source:?} must be unaffected when there is no session"
        );
        assert!(state
            .gate_daemon_mutation(CollabGateAction::ReplaceDocument, source)
            .is_ok());
    }
    assert!(!state.collab_session_is_active());
}

#[test]
fn an_active_owner_may_write_but_ai_and_mcp_may_not() {
    let mut state = daemon();
    in_session(
        &mut state,
        CollabConnectionPhase::Active,
        CollabUiRole::Owner,
    );
    assert!(state.collab_session_is_active());

    assert!(state
        .gate_daemon_mutation(DOCUMENT_EDIT, CollabEditSource::User)
        .is_ok());
    for source in [CollabEditSource::Ai, CollabEditSource::Mcp] {
        let refusal = state
            .gate_daemon_mutation(DOCUMENT_EDIT, source)
            .expect_err("desktop refuses these during a session, so the daemon must too");
        assert_eq!(refusal.code(), "collab-active");
        assert_eq!(refusal.http_status(), "409 Conflict");
    }
}

#[test]
fn a_viewer_is_read_only_for_its_own_writes() {
    let mut state = daemon();
    in_session(
        &mut state,
        CollabConnectionPhase::Active,
        CollabUiRole::Viewer,
    );
    let refusal = state
        .gate_daemon_mutation(DOCUMENT_EDIT, CollabEditSource::User)
        .expect_err("a viewer cannot write");
    assert_eq!(refusal.code(), "collab-readonly");
}

#[test]
fn a_frozen_phase_reports_busy_rather_than_read_only() {
    for phase in [
        CollabConnectionPhase::Starting,
        CollabConnectionPhase::Joining,
        CollabConnectionPhase::Authenticating,
    ] {
        let mut state = daemon();
        state.editor.editor_ui.collab.set_phase(phase);
        let refusal = state
            .gate_daemon_mutation(DOCUMENT_EDIT, CollabEditSource::User)
            .expect_err("a transitioning session accepts no writes");
        assert_eq!(refusal.code(), "collab-busy", "{phase:?}");
    }
}

#[test]
fn a_pending_edit_reports_busy() {
    let mut state = daemon();
    in_session(
        &mut state,
        CollabConnectionPhase::Active,
        CollabUiRole::Owner,
    );
    state.editor.editor_ui.collab.pending_edit = CollabPendingEditUi::Submitting;
    let refusal = state
        .gate_daemon_mutation(DOCUMENT_EDIT, CollabEditSource::User)
        .expect_err("one edit at a time");
    assert_eq!(refusal.code(), "collab-busy");
}

#[test]
fn replacing_the_whole_document_is_refused_for_every_source_in_a_session() {
    let mut state = daemon();
    in_session(
        &mut state,
        CollabConnectionPhase::Active,
        CollabUiRole::Owner,
    );
    for source in [
        CollabEditSource::User,
        CollabEditSource::Ai,
        CollabEditSource::Mcp,
        CollabEditSource::ExternalSync,
    ] {
        let refusal = state
            .gate_daemon_mutation(CollabGateAction::ReplaceDocument, source)
            .expect_err("a whole-document swap cannot be sequenced");
        assert_eq!(refusal.code(), "collab-active", "{source:?}");
    }
}

#[test]
fn apply_gated_separates_a_refusal_from_a_no_op() {
    let mut state = daemon();
    // No session: the editor's own ack rides through untouched, whatever it is.
    // What matters is that it arrives as `Ok`, so the caller can still tell a
    // refusal from whatever the editor decided.
    assert!(
        state
            .apply_gated(EditorCommand::ClearSelection, CollabEditSource::Mcp)
            .is_ok(),
        "no session refuses nothing"
    );

    in_session(
        &mut state,
        CollabConnectionPhase::Active,
        CollabUiRole::Owner,
    );
    let insert = EditorCommand::InsertNode {
        kind: "rect".into(),
        name: "From MCP".into(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: None,
        target_parent: op_editor_core::NodeId::NONE,
        page_id: None,
    };
    let refusal = state
        .apply_gated(insert, CollabEditSource::Mcp)
        .expect_err("an MCP write during a session must be a typed refusal");
    assert_eq!(refusal.code(), "collab-active");
}

#[test]
fn the_split_borrow_yields_a_host_over_the_same_editor() {
    use op_editor_host_core::collab::CollaborationEditorHost;

    let mut state = daemon();
    state.editor.editor_ui.preserve_authored_geometry = true;

    let (_runtime, mut host) = state.collab_runtime_and_host();
    assert!(host.editor_state().editor_ui.preserve_authored_geometry);
    host.editor_state_mut().editor_ui.preserve_authored_geometry = false;

    assert!(
        !state.editor.editor_ui.preserve_authored_geometry,
        "the host must borrow the daemon's editor, not a copy of it"
    );
}

#[test]
fn the_host_seam_raises_the_dirty_flag_the_driver_reads() {
    use op_collab_host::CollabHost;

    let mut state = daemon();
    assert!(!state.collab.take_dirty());

    let (_runtime, mut host) = state.collab_runtime_and_host();
    host.mark_editor_state_dirty();

    assert!(state.collab.take_dirty());
    assert!(!state.collab.take_dirty(), "taking the flag clears it");
}

#[test]
fn ingest_refuses_rather_than_writing_when_no_capture_can_open() {
    let mut state = daemon();
    // A projected Active phase with no real session actor: the runtime cannot
    // open a capture, and the ingest must report that instead of writing
    // behind the session's back.
    in_session(
        &mut state,
        CollabConnectionPhase::Active,
        CollabUiRole::Owner,
    );
    let before = state.editor.doc.clone();

    let prepared = op_editor_core::PreparedDocument::prepare(
        serde_json::from_value(serde_json::json!({
            "version": "1.0",
            "children": [{"type": "rectangle", "id": "n1", "x": 0, "y": 0, "width": 5, "height": 5}]
        }))
        .expect("valid document"),
    )
    .expect("validates");

    assert!(!state.ingest_document_in_session(prepared));
    assert_eq!(state.editor.doc, before, "a refused ingest writes nothing");
}

#[test]
fn the_projection_sequence_is_independent_of_the_document_version() {
    let mut state = daemon();
    assert_eq!(state.sse_tick().version, 0);
    assert_eq!(state.sse_tick().collab_seq, 0);

    // A presence-shaped change moves only `collabSeq`, so a client polling
    // `version` for document content is not made to refetch the document.
    state.collab.bump_seq();
    assert_eq!(state.sse_tick().version, 0);
    assert_eq!(state.sse_tick().collab_seq, 1);
}
