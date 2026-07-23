//! Destination-specific Web Save serialization.
//!
//! A daemon save and a browser download have different wire contracts. Keep
//! that decision explicit so the UI builds only the payload it is about to
//! send instead of materializing both whole-document strings up front.

use op_editor_core::EditorState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavePayloadTarget {
    /// Compact request body consumed by `/api/file/save`.
    Daemon,
    /// Canonical, image-table-externalized `.op` downloaded by the browser.
    BrowserDownload,
}

/// Match an asynchronous daemon acknowledgement to the exact host document.
///
/// EditorState's generation can restart at zero after a whole-state install,
/// so the host replacement epoch is part of the identity. This prevents a
/// delayed save of old document `0:0` from marking a newly imported `0:0`
/// document clean.
pub fn save_ack_matches_document(
    live_replacement_epoch: u64,
    live_document_generation: u64,
    snapshot_replacement_epoch: u64,
    snapshot_document_generation: u64,
) -> bool {
    live_replacement_epoch == snapshot_replacement_epoch
        && live_document_generation == snapshot_document_generation
}

/// Match a fallback download to the exact revision that the rejected daemon
/// request represented. Unlike an acknowledgement, a fallback creates a new
/// file from live state, so allowing a newer revision (or a replacement with
/// a restarted generation) would save and mark clean the wrong snapshot.
pub fn save_snapshot_matches_document(
    live_replacement_epoch: u64,
    live_document_generation: u64,
    live_document_revision: u64,
    snapshot_replacement_epoch: u64,
    snapshot_document_generation: u64,
    snapshot_document_revision: u64,
) -> bool {
    save_ack_matches_document(
        live_replacement_epoch,
        live_document_generation,
        snapshot_replacement_epoch,
        snapshot_document_generation,
    ) && live_document_revision == snapshot_document_revision
}

/// Serialize exactly one destination-specific Save payload.
pub fn serialize_save_payload(
    state: &EditorState,
    target: SavePayloadTarget,
) -> Result<String, String> {
    match target {
        SavePayloadTarget::Daemon => serialize_daemon_request(state),
        SavePayloadTarget::BrowserDownload => serialize_download_document(state),
    }
}

/// Compatibility entry point used by the browser IO tests.
#[cfg(test)]
pub fn serialize_document(state: &EditorState) -> Result<String, String> {
    serialize_save_payload(state, SavePayloadTarget::BrowserDownload)
}

/// Download file name for Save / Save As — the current display name when one
/// is set (a previously opened file), else `untitled.op`.
pub fn save_file_name(state: &EditorState) -> String {
    match state.editor_ui.file_name_display.as_deref() {
        Some(name) if !name.trim().is_empty() => name.to_string(),
        _ => "untitled.op".to_string(),
    }
}

/// Commit a successful synchronous browser download as the current save
/// baseline. The anchor click has already captured the Blob when this runs;
/// there is no asynchronous window in which a newer revision can appear.
pub fn acknowledge_browser_download(state: &mut EditorState, file_name: &str) {
    if state.editor_ui.file_name_display.is_none() {
        state.editor_ui.file_name_display = Some(file_name.to_string());
    }
    state.mark_saved_revision();
}

/// Compatibility entry point for the daemon request body.
#[cfg(test)]
pub fn save_request_body(state: &EditorState) -> Result<String, String> {
    serialize_save_payload(state, SavePayloadTarget::Daemon)
}

fn write_canonical_document(out: &mut Vec<u8>, state: &EditorState) -> Result<(), String> {
    let thumbnails = jian_ops_schema::image_thumbs::capture_snapshot();
    jian_ops_schema::image_table::write_document_with_extension(
        out,
        &state.doc,
        &thumbnails,
        "editorMeta",
        &op_pen_loader::EditorMeta {
            active_page_index: state.ui.active_page_index,
            preserve_authored_geometry: state.editor_ui.preserve_authored_geometry,
        },
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

fn finish_utf8(out: Vec<u8>) -> Result<String, String> {
    String::from_utf8(out).map_err(|error| error.to_string())
}

fn serialize_download_document(state: &EditorState) -> Result<String, String> {
    let mut out = Vec::new();
    write_canonical_document(&mut out, state)?;
    finish_utf8(out)
}

fn serialize_daemon_request(state: &EditorState) -> Result<String, String> {
    let mut out = Vec::new();
    out.extend_from_slice(br#"{"document":"#);
    write_canonical_document(&mut out, state)?;
    out.extend_from_slice(br#", "activePageIndex": "#);
    serde_json::to_writer(&mut out, &state.ui.active_page_index).map_err(|e| e.to_string())?;
    out.push(b'}');
    finish_utf8(out)
}

#[derive(Debug)]
pub struct SaveResponse {
    pub file_name: String,
    pub version: Option<u64>,
}

pub fn parse_save_response(response: &str) -> Result<SaveResponse, String> {
    let parsed: serde_json::Value = serde_json::from_str(response).map_err(|e| e.to_string())?;
    if !parsed.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        let message = parsed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Save failed");
        return Err(message.to_string());
    }
    let file_name = parsed
        .get("fileName")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("untitled.op")
        .to_string();
    let version = parsed.get("version").and_then(|value| value.as_u64());
    Ok(SaveResponse { file_name, version })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_page_state() -> EditorState {
        let src = r#"{"version":"1.0.0","children":[],"pages":[{"id":"p1","name":"One","children":[]},{"id":"p2","name":"Two","children":[{"type":"rectangle","id":"save-node","x":0,"y":0,"width":80,"height":40}]}]}"#;
        let doc = op_pen_loader::load_canonical(src)
            .expect("canonical doc")
            .value;
        let mut state = EditorState::from_document(doc);
        assert!(state.set_active_page(1));
        state
    }

    #[test]
    fn destination_selects_one_wire_contract() {
        let mut state = two_page_state();
        state.editor_ui.preserve_authored_geometry = true;
        let daemon = serialize_save_payload(&state, SavePayloadTarget::Daemon).unwrap();
        let download = serialize_save_payload(&state, SavePayloadTarget::BrowserDownload).unwrap();
        let daemon: serde_json::Value = serde_json::from_str(&daemon).unwrap();
        let download: serde_json::Value = serde_json::from_str(&download).unwrap();

        assert_eq!(daemon["activePageIndex"], 1);
        assert_eq!(
            daemon["document"]["pages"][1]["children"][0]["id"],
            "save-node"
        );
        assert_eq!(daemon["document"], download);
        assert!(download.get("document").is_none());
        assert_eq!(download["editorMeta"]["activePageIndex"], 1);
        assert_eq!(download["editorMeta"]["preserveAuthoredGeometry"], true);
        assert_eq!(
            daemon["document"]["editorMeta"]["preserveAuthoredGeometry"],
            true
        );
        assert_eq!(download["pages"][1]["children"][0]["id"], "save-node");
    }

    #[test]
    fn both_destinations_share_image_table_and_editor_meta_semantics() {
        let image = format!("data:image/png;base64,{}", "A".repeat(4_200));
        let src = format!(
            r#"{{"version":"1.0.0","children":[{{"type":"image","id":"a","src":{image:?}}},{{"type":"image","id":"b","src":{image:?}}}]}}"#
        );
        let doc = op_pen_loader::load_canonical(&src)
            .expect("image document")
            .value;
        let mut state = EditorState::from_document(doc);
        state.ui.active_page_index = 3;

        let daemon: serde_json::Value = serde_json::from_str(
            &serialize_save_payload(&state, SavePayloadTarget::Daemon).unwrap(),
        )
        .unwrap();
        let download: serde_json::Value = serde_json::from_str(
            &serialize_save_payload(&state, SavePayloadTarget::BrowserDownload).unwrap(),
        )
        .unwrap();

        assert_eq!(daemon["document"], download);
        assert_eq!(daemon["activePageIndex"], 3);
        assert_eq!(download["editorMeta"]["activePageIndex"], 3);
        assert_eq!(
            download["images"].as_object().map(|table| table.len()),
            Some(1)
        );
        assert_eq!(download.to_string().matches("op-image:").count(), 2);
        assert_eq!(download.to_string().matches(&image).count(), 1);
    }

    #[test]
    fn compatibility_entry_points_match_the_explicit_targets() {
        let state = two_page_state();
        assert_eq!(
            save_request_body(&state).unwrap(),
            serialize_save_payload(&state, SavePayloadTarget::Daemon).unwrap()
        );
        assert_eq!(
            serialize_document(&state).unwrap(),
            serialize_save_payload(&state, SavePayloadTarget::BrowserDownload).unwrap()
        );
    }

    #[test]
    fn save_ack_identity_includes_the_host_replacement_epoch() {
        assert!(save_ack_matches_document(7, 0, 7, 0));
        assert!(save_ack_matches_document(7, 4, 7, 4));
        assert!(!save_ack_matches_document(8, 0, 7, 0));
        assert!(!save_ack_matches_document(7, 5, 7, 4));
    }

    #[test]
    fn fallback_identity_includes_epoch_generation_and_exact_revision() {
        assert!(save_snapshot_matches_document(7, 4, 9, 7, 4, 9));
        assert!(!save_snapshot_matches_document(8, 4, 9, 7, 4, 9));
        assert!(!save_snapshot_matches_document(7, 5, 9, 7, 4, 9));
        assert!(!save_snapshot_matches_document(7, 4, 10, 7, 4, 9));
    }

    #[test]
    fn successful_browser_download_commits_the_current_revision() {
        let mut state = two_page_state();
        state.mark_document_changed();
        assert!(state.is_dirty());

        acknowledge_browser_download(&mut state, "download.op");

        assert!(!state.is_dirty());
        assert!(!state.editor_ui.document_dirty);
        assert_eq!(
            state.editor_ui.file_name_display.as_deref(),
            Some("download.op")
        );
    }
}
