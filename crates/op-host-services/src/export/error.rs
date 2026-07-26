//! Typed failures for the raster export core (`export.rs`).
//!
//! Style follows `op_orchestrator::OrchestratorError`: a plain enum plus a
//! hand-written `Display`, no `thiserror` and no new dependency. Unlike the
//! transport enums in this crate, most variants carry STRUCTURED fields and
//! `Display` re-formats the sentence, so the user-visible text is reproduced
//! byte for byte while callers can match on the reason instead of the prose.
//! The MCP screenshot glue wraps some of these in the TS-parity "Renderer
//! reported failure: …" envelope, and `mcp_serve/export_tool.rs` ships them
//! to the model verbatim, so the wording is part of the contract.
//!
//! Scope note: the FILE-WRITING entry points (`export_raster`,
//! `export_node_raster`, `export_svg`, `export_node_svg`, `export_pdf`) and
//! `screenshot::capture{,_scene}` deliberately still report `String`. They
//! are consumed by code outside this crate that returns their `Result`
//! DIRECTLY (`op-host-desktop::persistence::export_editor_state_to_path`) or
//! bakes the error type into a channel / closure signature
//! (`mcp_live.rs`'s screenshot `SyncSender`), so a typed error would not
//! convert through `?` — it would ripple into modules this pass does not own.
//! Each of those sites converts with a documented `.map_err`; everything
//! reachable only from inside this crate is typed.

use std::fmt;

use super::{MAX_RASTER_SIDE_PX, MAX_RASTER_TOTAL_PX};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// The scene carries no active page, so there is nothing to render.
    NoActivePage,
    /// The active page has no painted content at all.
    NothingToExport,
    /// A named page resolved but contributes no visible pixels.
    PageEmpty { page_id: String },
    /// The requested node id is absent from the scene's active page.
    NodeNotFoundOnActivePage { node_id: String },
    /// The requested node id is absent from the explicitly named page.
    NodeNotFoundOnPage { node_id: String, page_id: String },
    /// The node exists but its subtree paints nothing, so there are no
    /// bounds to size a surface from.
    NodePaintsNothing { node_id: String },
    /// The node is hidden; exporting it would produce an empty image, so the
    /// request is refused rather than silently satisfied.
    NodeHidden { node_id: String },
    /// Corrupt bounds or scale produced a non-finite surface size.
    NonFiniteOutputSize,
    /// The requested surface exceeds [`MAX_RASTER_SIDE_PX`] per side or
    /// [`MAX_RASTER_TOTAL_PX`] in total — the guard against a giant
    /// UI-thread allocation.
    OutputTooLarge { width_px: i64, height_px: i64 },
    /// Skia refused to allocate the offscreen raster surface.
    SurfaceAlloc,
    /// Skia's encoder failed for the requested format ("PNG" / "JPEG" /
    /// "WEBP").
    Encode { format: &'static str },
    /// Skia's PDF backend closed the document without emitting any bytes.
    PdfEncoderEmpty,
    /// A multi-node PDF export was asked for an empty id list.
    NoNodeIdsRequested,
    /// None of the requested node ids resolved to paintable content, so
    /// there is no PDF page to emit.
    NoRequestedNodesPaint,
    /// Writing the encoded bytes to the export target failed.
    Write(String),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::NoActivePage => f.write_str("no active page"),
            ExportError::NothingToExport => f.write_str("nothing to export"),
            ExportError::PageEmpty { page_id } => {
                write!(f, "page {page_id} has no visible content to export")
            }
            ExportError::NodeNotFoundOnActivePage { node_id } => {
                write!(f, "node {node_id} not found on the active page")
            }
            ExportError::NodeNotFoundOnPage { node_id, page_id } => {
                write!(f, "node {node_id} not found on page {page_id}")
            }
            ExportError::NodePaintsNothing { node_id } => {
                write!(f, "node {node_id} paints nothing")
            }
            ExportError::NodeHidden { node_id } => {
                write!(f, "node {node_id} is hidden and cannot be exported")
            }
            ExportError::NonFiniteOutputSize => {
                f.write_str("raster output size is not finite (corrupt bounds / scale)")
            }
            ExportError::OutputTooLarge {
                width_px,
                height_px,
            } => write!(
                f,
                "raster output {width_px}x{height_px} px exceeds the size cap \
                 ({MAX_RASTER_SIDE_PX} px per side, {MAX_RASTER_TOTAL_PX} px total) — \
                 lower the scale / padding or export a smaller node"
            ),
            ExportError::SurfaceAlloc => f.write_str("alloc surface"),
            ExportError::Encode { format } => write!(f, "encode {format} failed"),
            ExportError::PdfEncoderEmpty => f.write_str("PDF encoder returned no bytes"),
            ExportError::NoNodeIdsRequested => f.write_str("no node ids provided"),
            ExportError::NoRequestedNodesPaint => {
                f.write_str("no requested nodes have paintable content on the active page")
            }
            ExportError::Write(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ExportError {}

/// Boundary bridge for the file-writing entry points and
/// `screenshot::capture{,_scene}`, which still report `String` (see the
/// module docs for why). `Display` reproduces the original sentence, so a
/// conversion through here is text-preserving. Delete it once those
/// signatures can move.
impl From<ExportError> for String {
    fn from(error: ExportError) -> String {
        error.to_string()
    }
}
