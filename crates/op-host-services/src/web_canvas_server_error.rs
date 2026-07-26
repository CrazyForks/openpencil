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
//! Several dependencies of this module (`mcp_serve`, `doc_io`, `export`,
//! `settings_io`, `op_pen_loader`) still report `String`; they are outside
//! this conversion's scope, so their errors are adapted into a variant at the
//! call site instead of rippling the change into them. The daemon's three
//! public entry points (`parse_serve_web_args`, `startup_editor_for_web_canvas`,
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
