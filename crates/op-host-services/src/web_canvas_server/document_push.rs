//! Parsing a whole-document push, off the state lock.
//!
//! Split out of the `web_canvas_server` spine at the 800-line cap. The point
//! of the type is WHERE it is built — see [`PendingDocumentPush`].

use super::{Result, ServeMode, WebCanvasError};

/// A whole-document push, parsed and validated but not yet installed.
///
/// The point of the type is WHERE it is built: JSON parse, schema load and
/// structural validation are the expensive, entirely self-contained part of a
/// push, and they used to run with the state mutex held — so one client
/// uploading a large document with embedded images stalled every other
/// request to that tenant, including the SSE version probe. Building this
/// outside the lock leaves only the install under it.
pub(crate) struct PendingDocumentPush {
    pub(super) base_version: Option<u64>,
    pub(super) editor_meta: op_pen_loader::EditorMeta,
    /// `None` for a metadata-only push (an active-page switch), which carries
    /// no document to install.
    pub(super) prepared: Option<op_editor_core::PreparedDocument>,
}

impl PendingDocumentPush {
    /// Parse and validate a push body. **Call this OUTSIDE the state lock.**
    ///
    /// `mode` decides only whether the document's thumbnails may be published
    /// to the process-global registry; nothing else here reads live state,
    /// which is what makes it safe to run unlocked.
    pub(crate) fn parse(body: &str, mode: ServeMode) -> Result<Self> {
        let request = crate::mcp_serve::parse_document_sync_request(body)?;
        let base_version = request.base_version;
        let editor_meta = request.resolved_editor_meta(request.embedded_editor_meta.clone());
        if request.metadata_only {
            return Ok(Self {
                base_version,
                editor_meta,
                prepared: None,
            });
        }
        // Load via the same proven path as desktop file-open. A load failure
        // is a client fault → 400, like the TS validation 400s.
        let loaded = op_pen_loader::load_canonical(request.document_json)
            .map_err(|e| WebCanvasError::Document(e.to_string()))?;
        if !mode.allows_image_thumb_registry() {
            // The thumbnail registry is a process-global map that a document
            // activation replaces WHOLESALE, so in a shared process this
            // push would drop every other account's thumbnails and rebind
            // their ids to these bytes. Dropping the pending seed here makes
            // the activation downstream a strict no-op, and image nodes
            // paint their placeholder. See
            // `ServeMode::allows_image_thumb_registry` for the real fix.
            jian_ops_schema::image_thumbs::discard_for_document(&loaded.value);
        }
        for w in &loaded.warnings {
            eprintln!("openpencil-desktop --serve-web: schema warning: {w:?}");
        }
        // Structural validation happens here, before any state changes, so the
        // in-session install is infallible and cannot leave a half-applied
        // document inside an open edit capture.
        let prepared = op_editor_core::PreparedDocument::prepare(loaded.value)
            .map_err(|e| WebCanvasError::Document(e.to_string()))?;
        Ok(Self {
            base_version,
            editor_meta,
            prepared: Some(prepared),
        })
    }
}
