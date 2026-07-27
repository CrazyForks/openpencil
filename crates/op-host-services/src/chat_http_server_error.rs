//! Typed failures for the OpenCode chat transport (`chat_http_server.rs`) —
//! the local `opencode serve` lifecycle plus the JSON control-plane calls
//! that drive one turn.
//!
//! Style follows `op_orchestrator::OrchestratorError`: a plain enum plus a
//! hand-written `Display`, no `thiserror` and no new dependency. The
//! lifecycle variants carry STRUCTURED fields and `Display` re-formats the
//! sentence; every message here is a verbatim port of the TS reference path
//! (`apps/web/server/opencode/server.ts` + `chat.ts`), reaches the user
//! through `ChatDelta::Error`, and is asserted on, so the wording is part of
//! the contract.
//!
//! What the enum adds is a distinction the stringly-typed code could not
//! express: whether a failure came from the SERVER LIFECYCLE (spawn, the
//! stdout handshake, the listen timeout — recoverable by retrying the turn,
//! and the only kind that leaves a child process to reap) or from a
//! REQUEST against an already-running server. The two arrive at the same
//! `fail(tx, …)` sink today; keeping them apart is what lets a future caller
//! decide differently without re-parsing prose.
//!
//! Two variants are deliberately transparent, both because the text comes
//! from somewhere this module does not own: [`OpenCodeError::Request`]
//! carries a `reqwest` error's own `Display`, and [`OpenCodeError::Provider`]
//! carries whatever `format_opencode_error` extracted from the SERVER's error
//! object (a label + nested-JSON unwrap whose shape is OpenCode's, not ours).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenCodeError {
    /// The `opencode` binary could not be launched at all.
    Spawn { binary: String, message: String },
    /// The spawned child exposed no stdout pipe, so the listening
    /// handshake can never arrive.
    NoStdout,
    /// The child exited before announcing a listening URL. `output` is the
    /// captured stderr/stdout diagnostic buffer; it is appended only when
    /// non-blank, matching the TS message exactly.
    ServerExited { code: String, output: String },
    /// stdout stayed open but never carried the `opencode server listening
    /// on <url>` line within the platform listen budget.
    ListenTimeout { millis: u128 },
    /// A control-plane request never completed (connect / timeout / body).
    /// Text is `reqwest`'s own, an upstream crate this pass does not own.
    Request(String),
    /// The server answered with a structured OpenCode error object, already
    /// rendered by `format_opencode_error`.
    Provider(String),
    /// A non-2xx answer whose body was not a parseable OpenCode error
    /// object, so the raw (trimmed) text is shown instead.
    HttpStatus {
        status: reqwest::StatusCode,
        body: String,
    },
    /// A 2xx answer whose body was not valid JSON.
    InvalidJson { message: String },
}

impl fmt::Display for OpenCodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenCodeError::Spawn { binary, message } => {
                write!(f, "spawn {binary} serve: {message}")
            }
            OpenCodeError::NoStdout => f.write_str("opencode serve: no stdout"),
            OpenCodeError::ServerExited { code, output } => {
                write!(f, "Server exited with code {code}")?;
                if !output.trim().is_empty() {
                    write!(f, "\nServer output: {output}")?;
                }
                Ok(())
            }
            OpenCodeError::ListenTimeout { millis } => {
                write!(f, "Timeout waiting for server to start after {millis}ms")
            }
            OpenCodeError::Request(message) | OpenCodeError::Provider(message) => {
                f.write_str(message)
            }
            OpenCodeError::HttpStatus { status, body } => write!(f, "http {status}: {body}"),
            OpenCodeError::InvalidJson { message } => {
                write!(f, "invalid JSON response: {message}")
            }
        }
    }
}

impl std::error::Error for OpenCodeError {}
