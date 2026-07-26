//! Typed per-line failures for the `batch_design` DSL program executor
//! (`batch_program.rs`).
//!
//! Style follows `op_orchestrator::OrchestratorError`: a plain enum plus a
//! hand-written `Display`, no `thiserror` and no new dependency. Each
//! variant's `Display` reproduces the exact sentence the stringly-typed
//! executor produced, because those sentences ship verbatim to the model in
//! the envelope's `errors[]` array (and several tests assert on them).
//!
//! What the enum buys over `String` is the CLASSIFICATION: the executor —
//! and anything downstream, e.g. a future retry ladder in `program_gen.rs` —
//! can now tell a grammar mistake from a missing node from an id-space
//! exhaustion without pattern-matching prose.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProgramError {
    /// The line matched none of the DSL operation grammars.
    UnparsableLine(String),
    /// An operation's argument list is malformed, has the wrong arity, or a
    /// scalar argument has the wrong JSON type.
    Syntax(String),
    /// A JSON argument could not be parsed even after the lenient
    /// agent-typo repair pipeline, or parsed to the wrong JSON kind.
    Json(String),
    /// A body parsed as JSON but is not a valid `PenNode`.
    InvalidNode(String),
    /// A referenced node, path, kit, or component does not exist.
    NotFound(String),
    /// The operation parsed and resolved, but a structural / semantic rule
    /// of the design protocol refuses it (placement contracts, sizing
    /// requirements, layout preconditions).
    Rejected(String),
    /// The simulated apply refused the command — the host would refuse it
    /// too, so the line cannot ship.
    ApplyRejected(String),
    /// An operation that must yield a node yielded none. The payload is the
    /// operation label as it appears in the message (`Insert`, `Copy`,
    /// `Replace`, `G()`).
    ProducedNoNode(&'static str),
    /// The document's node id space is exhausted, so no fresh id can be
    /// minted for the remapped subtree.
    IdSpaceExhausted,
}

impl fmt::Display for ProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProgramError::UnparsableLine(line) => write!(f, "Cannot parse operation: {line}"),
            ProgramError::Syntax(m)
            | ProgramError::Json(m)
            | ProgramError::InvalidNode(m)
            | ProgramError::NotFound(m)
            | ProgramError::Rejected(m)
            | ProgramError::ApplyRejected(m) => f.write_str(m),
            ProgramError::ProducedNoNode(op) => write!(f, "{op} produced no node"),
            ProgramError::IdSpaceExhausted => f.write_str("node id space exhausted"),
        }
    }
}

impl std::error::Error for ProgramError {}
