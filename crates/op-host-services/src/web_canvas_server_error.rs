//! Typed failures for the `--serve-web` web-canvas daemon
//! (`web_canvas_server.rs`).
//!
//! Style follows `op_orchestrator::OrchestratorError`: a plain enum plus a
//! hand-written `Display`, no `thiserror` and no new dependency. `Display` is
//! transparent — each variant carries the exact sentence the route already
//! embedded in its JSON error body, so the wire bytes are unchanged.
//!
//! What the enum adds is a route-independent classification plus
//! [`WebCanvasError::http_status`], which turns "which status does this
//! failure answer with" from a per-call-site literal into one table.
//!
//! `mcp_serve` now reports [`crate::mcp_serve::McpServeError`], so its
//! failures reach this enum through the [`From`] impl at the bottom of this
//! file and the routes just use `?` — no per-call-site re-labelling. The
//! remaining dependencies (`doc_io`, `settings_io`, `op_pen_loader`, and the
//! file-writing `export` / `export_pdf` entry points, which are consumed
//! directly by `op-host-desktop::persistence`) still report `String` and are
//! adapted into a variant at the call site. The daemon's three public entry
//! points (`parse_serve_web_args`, `startup_editor_for_web_canvas`,
//! `run_web_canvas`) likewise keep their `Result<_, String>` signatures — they
//! are consumed by `cli_modes.rs` and the host binaries — and convert at the
//! boundary.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WebCanvasError {
    /// The request body is malformed, has the wrong shape, or names an
    /// unsupported parameter — a client fault.
    BadRequest(String),
    /// A document failed to load or validate through the canonical loader.
    Document(String),
    /// The raster / PDF export pipeline failed.
    Export(String),
    /// A filesystem operation on a request-scoped file (the save target, an
    /// export temp file, the backing document) failed.
    Io(String),
    /// Start-up configuration failed: bad `--serve-web` argv, an unusable
    /// settings file, or a socket that would not bind. Never becomes an HTTP
    /// response — it aborts daemon start-up.
    Config(String),
    /// A connection-level read/write failed (request parse, response write,
    /// SSE stream). Never becomes an HTTP response — the socket is already
    /// unusable; the accept loop just logs it.
    Transport(String),
}

impl WebCanvasError {
    /// The HTTP status a route answers with when it surfaces this failure.
    ///
    /// The four request-scoped kinds are all client faults — the daemon's
    /// in-memory document authority is healthy; what failed is the request's
    /// payload or the file it named — so they answer `400`, matching the
    /// pre-conversion behaviour of every route in this module exactly.
    /// `Config` / `Transport` never reach a response; they report the generic
    /// `500` so a future caller that does surface one is not silently wrong.
    pub(crate) fn http_status(&self) -> &'static str {
        match self {
            WebCanvasError::BadRequest(_)
            | WebCanvasError::Document(_)
            | WebCanvasError::Export(_)
            | WebCanvasError::Io(_) => "400 Bad Request",
            WebCanvasError::Config(_) | WebCanvasError::Transport(_) => "500 Internal Server Error",
        }
    }
}

impl fmt::Display for WebCanvasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebCanvasError::BadRequest(m)
            | WebCanvasError::Document(m)
            | WebCanvasError::Export(m)
            | WebCanvasError::Io(m)
            | WebCanvasError::Config(m)
            | WebCanvasError::Transport(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for WebCanvasError {}

/// Single-table mapping from the shared MCP transport's failures onto this
/// daemon's classification, replacing the per-call-site `.map_err(...)`
/// re-labelling every route used to carry. Both `Display` impls are
/// transparent, so the resulting HTTP body / log line is byte-identical to
/// the pre-conversion text.
impl From<crate::mcp_serve::McpServeError> for WebCanvasError {
    fn from(error: crate::mcp_serve::McpServeError) -> WebCanvasError {
        use crate::mcp_serve::McpServeError as E;
        match error {
            // The daemon's document authority failed to load the file it was
            // pointed at — the same 400 the route reported before.
            E::Document(m) => WebCanvasError::Document(m),
            // A rejected JSON-RPC message is a client fault: the parser or
            // registry refused it and nothing was applied.
            E::Dispatch(m) => WebCanvasError::BadRequest(m),
            // Malformed framing and socket failures alike leave the
            // connection unusable, so both land on `Transport` — which the
            // accept loop logs instead of answering with. Matches the
            // pre-conversion `.map_err(WebCanvasError::Transport)` exactly.
            E::Protocol(m) | E::Io(m) => WebCanvasError::Transport(m),
            E::Config(m) => WebCanvasError::Config(m),
        }
    }
}

/// Same idea for the raster/PDF export core. Every export failure answered
/// `400` before this conversion, and `Export` / `Io` both still do (see
/// [`WebCanvasError::http_status`]), so routing the write failure to `Io`
/// sharpens the classification without moving a single status code or byte
/// of the response body.
impl From<crate::export::ExportError> for WebCanvasError {
    fn from(error: crate::export::ExportError) -> WebCanvasError {
        use crate::export::ExportError as E;
        match error {
            E::Write(m) => WebCanvasError::Io(m),
            other => WebCanvasError::Export(other.to_string()),
        }
    }
}
