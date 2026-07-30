//! Hidden headless image-enrichment mode for batch design artifacts.
//!
//! `openpencil-desktop --enrich-images <input.op> <output.op> [timeout_seconds]`
//! loads and saves through the canonical document I/O layer while delegating
//! every target decision and mutation to `ImageSearchSession`. The command
//! runs before the desktop event loop and single-instance gate. It is
//! intentionally search-only: Auto and Search targets use stock search, while
//! an explicit Generate target fails instead of silently changing acquisition
//! mode.

use std::collections::HashSet;
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use op_editor_core::{walkers, EditorState, NodeId, PenNodeExt};
use serde::Serialize;

use crate::image_search_session::{
    collect_targets, ImageSearchSession, SEARCH_FAILED_PLACEHOLDER_SRC,
};

const DEFAULT_TIMEOUT_SECONDS: u64 = 120;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const USAGE: &str =
    "usage: openpencil-desktop --enrich-images <input.op> <output.op> [timeout_seconds]";
const MODE_CONTRACT: &str = "search-only mode: Auto/Search targets use stock search; \
explicit Generate targets fail and are never converted to search";

#[derive(Debug, Clone, PartialEq, Eq)]
struct EnrichRequest {
    input: PathBuf,
    output: PathBuf,
    timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct EnrichSummary {
    pages: usize,
    targets: usize,
    resolved: usize,
    failed: usize,
    unresolved: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ArgumentError {
    MissingInput,
    MissingOutput,
    InvalidTimeout(String),
    ZeroTimeout,
    UnexpectedArgument(String),
}

impl fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingInput => write!(f, "missing input document"),
            Self::MissingOutput => write!(f, "missing output document"),
            Self::InvalidTimeout(value) => write!(f, "invalid timeout_seconds: {value}"),
            Self::ZeroTimeout => write!(f, "timeout_seconds must be greater than zero"),
            Self::UnexpectedArgument(value) => write!(f, "unexpected argument: {value}"),
        }
    }
}

#[derive(Debug)]
enum EnrichError {
    Load { path: PathBuf, message: String },
    RewriteBlocked { path: PathBuf },
    InvalidDocument { path: PathBuf, message: String },
    InvalidPage { page: usize, page_count: usize },
    Timeout { page: usize, seconds: u64 },
    Failed(EnrichSummary),
    Save { path: PathBuf, message: String },
}

impl fmt::Display for EnrichError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load { path, message } => {
                write!(f, "enrich-images: load {}: {message}", path.display())
            }
            Self::RewriteBlocked { path } => write!(
                f,
                "enrich-images: refusing to rewrite {} because its schema version is malformed or newer than this build",
                path.display()
            ),
            Self::InvalidDocument { path, message } => write!(
                f,
                "enrich-images: invalid document {}: {message}",
                path.display()
            ),
            Self::InvalidPage { page, page_count } => write!(
                f,
                "enrich-images: page index {page} is outside page count {page_count}"
            ),
            Self::Timeout { page, seconds } => write!(
                f,
                "enrich-images: timed out on page {page} after {seconds} second(s)"
            ),
            Self::Failed(summary) => write!(
                f,
                "enrich-images: {} failed and {} unresolved target(s); {MODE_CONTRACT}",
                summary.failed, summary.unresolved,
            ),
            Self::Save { path, message } => {
                write!(f, "enrich-images: save {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for EnrichError {}

/// Dispatch the hidden mode when requested. Failures exit non-zero before the
/// GUI and single-instance paths can run.
pub(crate) fn run_cli_if_requested() -> bool {
    let request = match parse_cli_args(std::env::args_os()) {
        Ok(None) => return false,
        Ok(Some(request)) => request,
        Err(error) => {
            eprintln!("enrich-images: {error}");
            eprintln!("{USAGE}");
            eprintln!("{MODE_CONTRACT}");
            std::process::exit(2);
        }
    };
    match enrich_document(&request) {
        Ok(summary) => {
            let encoded = serde_json::to_string(&summary)
                .expect("an integer-only image enrichment summary always serializes");
            println!("{encoded}");
            true
        }
        Err(EnrichError::Failed(summary)) => {
            let encoded = serde_json::to_string(&summary)
                .expect("an integer-only image enrichment summary always serializes");
            println!("{encoded}");
            eprintln!("{}", EnrichError::Failed(summary));
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn parse_cli_args<I>(args: I) -> Result<Option<EnrichRequest>, ArgumentError>
where
    I: IntoIterator<Item = OsString>,
{
    let args: Vec<OsString> = args.into_iter().collect();
    let Some(position) = args.iter().position(|arg| arg == "--enrich-images") else {
        return Ok(None);
    };
    let trailing = &args[position + 1..];
    let Some(input) = trailing.first() else {
        return Err(ArgumentError::MissingInput);
    };
    let Some(output) = trailing.get(1) else {
        return Err(ArgumentError::MissingOutput);
    };
    if let Some(extra) = trailing.get(3) {
        return Err(ArgumentError::UnexpectedArgument(
            extra.to_string_lossy().into_owned(),
        ));
    }
    let timeout_seconds = match trailing.get(2) {
        Some(value) => {
            let value = value.to_string_lossy();
            value
                .parse::<u64>()
                .map_err(|_| ArgumentError::InvalidTimeout(value.into_owned()))?
        }
        None => DEFAULT_TIMEOUT_SECONDS,
    };
    if timeout_seconds == 0 {
        return Err(ArgumentError::ZeroTimeout);
    }
    Ok(Some(EnrichRequest {
        input: PathBuf::from(input),
        output: PathBuf::from(output),
        timeout: Duration::from_secs(timeout_seconds),
    }))
}

fn enrich_document(request: &EnrichRequest) -> Result<EnrichSummary, EnrichError> {
    let loaded = op_host_services::doc_io::load_editor_state_with_report(
        &request.input,
        op_editor_core::Locale::EnUs,
    )
    .map_err(|error| EnrichError::Load {
        path: request.input.clone(),
        message: error.to_string(),
    })?;
    if loaded.report.rewrite_blocked_by_schema_warning {
        return Err(EnrichError::RewriteBlocked {
            path: request.input.clone(),
        });
    }
    let mut state = loaded.state;
    state
        .validate()
        .map_err(|error| EnrichError::InvalidDocument {
            path: request.input.clone(),
            message: error.to_string(),
        })?;

    // Restore Openverse OAuth from the same settings payload as the GUI, but
    // remove every in-memory generation profile for this one-shot process.
    // `active_image_gen_profile()` intentionally falls back to the first
    // profile, so clearing only its selected id would not be search-only.
    op_host_services::settings_io::load(&mut state);
    state.editor_ui.agent_settings.image_gen_profiles.clear();
    state.editor_ui.agent_settings.active_image_gen_profile_id = None;

    let original_page = state.ui.active_page_index;
    let result = enrich_state(&mut state, request.timeout);
    let restored = state.set_active_page(original_page);
    if !restored {
        return Err(EnrichError::InvalidPage {
            page: original_page,
            page_count: state.page_count(),
        });
    }
    let summary = result?;
    if summary.failed != 0 || summary.unresolved != 0 {
        return Err(EnrichError::Failed(summary));
    }
    op_host_services::doc_io::save_to_path(&state, &request.output).map_err(|error| {
        EnrichError::Save {
            path: request.output.clone(),
            message: error.to_string(),
        }
    })?;
    Ok(summary)
}

fn enrich_state(state: &mut EditorState, timeout: Duration) -> Result<EnrichSummary, EnrichError> {
    let mut session = ImageSearchSession::new();
    enrich_state_with_session(state, timeout, &mut session)
}

fn enrich_state_with_session(
    state: &mut EditorState,
    timeout: Duration,
    session: &mut ImageSearchSession,
) -> Result<EnrichSummary, EnrichError> {
    // One deadline covers the complete enrichment phase across every page.
    // Document load and the final atomic save intentionally sit outside it.
    let started = Instant::now();
    let timeout_seconds = timeout.as_secs();
    let page_count = state.page_count();
    let mut targets = 0usize;
    let mut failed = 0usize;
    let mut unresolved = 0usize;

    for page in 0..page_count {
        if !state.set_active_page(page) {
            return Err(EnrichError::InvalidPage { page, page_count });
        }
        let mut target_ids: HashSet<NodeId> = collect_targets(state, &HashSet::new())
            .into_iter()
            .map(|target| target.node_id)
            .collect();
        collect_failure_sentinel_ids(state.active_children(), &mut target_ids);
        targets += target_ids.len();

        loop {
            let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
            let enqueued = session.enqueue_missing_with_scene(state, &scene);
            let was_pending = session.is_pending();
            let changed = session.poll_into_with_scene(state, &scene);
            if !session.is_pending() && !enqueued && !was_pending && !changed {
                break;
            }
            // A synchronous or just-completed job gets an immediate
            // quiescence pass even at the deadline. Only pending work can
            // time out; this prevents a completed final poll from being
            // misreported as a timeout.
            if !session.is_pending() {
                continue;
            }
            if started.elapsed() >= timeout {
                // Poll once more at the boundary so a completion that raced
                // the first poll wins over the deadline.
                let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
                let _ = session.poll_into_with_scene(state, &scene);
                if !session.is_pending() {
                    continue;
                }
                return Err(EnrichError::Timeout {
                    page,
                    seconds: timeout_seconds,
                });
            }
            std::thread::sleep(POLL_INTERVAL);
        }

        let remaining = collect_targets(state, &HashSet::new());
        unresolved += remaining.len();
        failed += target_ids
            .iter()
            .filter(|node_id| node_has_failure_sentinel(state, node_id))
            .count();
    }

    Ok(EnrichSummary {
        pages: page_count,
        targets,
        resolved: targets.saturating_sub(failed.saturating_add(unresolved)),
        failed,
        unresolved,
    })
}

fn collect_failure_sentinel_ids(children: &[PenNode], out: &mut HashSet<NodeId>) {
    for node in children {
        if node_contains_failure_sentinel(node) {
            if let Some(id) = NodeId::new_opt(node.id_str()) {
                out.insert(id);
            }
        }
        if let Some(children) = node.children() {
            collect_failure_sentinel_ids(children, out);
        }
    }
}

fn node_contains_failure_sentinel(node: &PenNode) -> bool {
    match node {
        PenNode::Image(image) => image.src == SEARCH_FAILED_PLACEHOLDER_SRC,
        PenNode::Frame(frame) => fills_have_failure_sentinel(frame.container.fill.as_deref()),
        PenNode::Rectangle(rectangle) => {
            fills_have_failure_sentinel(rectangle.container.fill.as_deref())
        }
        _ => false,
    }
}

fn node_has_failure_sentinel(state: &EditorState, node_id: &NodeId) -> bool {
    let Some(node) = walkers::find_node(state.active_children(), node_id) else {
        return false;
    };
    node_contains_failure_sentinel(node)
}

fn fills_have_failure_sentinel(fills: Option<&[PenFill]>) -> bool {
    fills.is_some_and(|fills| {
        fills.iter().any(|fill| {
            matches!(
                fill,
                PenFill::Image(image) if image.url == SEARCH_FAILED_PLACEHOLDER_SRC
            )
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn parser_accepts_default_and_explicit_timeout() {
        let default = parse_cli_args(args(&["app", "--enrich-images", "in.op", "out.op"]))
            .expect("valid arguments")
            .expect("requested");
        assert_eq!(default.input, Path::new("in.op"));
        assert_eq!(default.output, Path::new("out.op"));
        assert_eq!(default.timeout, Duration::from_secs(120));

        let explicit = parse_cli_args(args(&["app", "--enrich-images", "in.op", "out.op", "7"]))
            .expect("valid arguments")
            .expect("requested");
        assert_eq!(explicit.timeout, Duration::from_secs(7));
    }

    #[test]
    fn parser_rejects_missing_invalid_and_extra_arguments() {
        assert_eq!(
            parse_cli_args(args(&["app", "--enrich-images"])),
            Err(ArgumentError::MissingInput)
        );
        assert_eq!(
            parse_cli_args(args(&["app", "--enrich-images", "in.op"])),
            Err(ArgumentError::MissingOutput)
        );
        assert_eq!(
            parse_cli_args(args(&["app", "--enrich-images", "in.op", "out.op", "soon"])),
            Err(ArgumentError::InvalidTimeout("soon".to_string()))
        );
        assert_eq!(
            parse_cli_args(args(&[
                "app",
                "--enrich-images",
                "in.op",
                "out.op",
                "1",
                "extra"
            ])),
            Err(ArgumentError::UnexpectedArgument("extra".to_string()))
        );
        assert!(MODE_CONTRACT.contains("explicit Generate targets fail"));
    }

    #[test]
    fn quiescent_document_wins_over_an_elapsed_deadline() {
        let unique = format!(
            "openpencil-enrich-images-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        );
        let directory = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&directory).expect("create fixture directory");
        let input = directory.join("input.op");
        let output = directory.join("output.op");
        let original = EditorState::new();
        op_host_services::doc_io::save_to_path(&original, &input).expect("save input");

        let summary = enrich_document(&EnrichRequest {
            input: input.clone(),
            output: output.clone(),
            timeout: Duration::ZERO,
        })
        .expect("already-quiescent enrichment succeeds at the deadline");

        assert_eq!(
            summary,
            EnrichSummary {
                pages: 1,
                targets: 0,
                resolved: 0,
                failed: 0,
                unresolved: 0,
            }
        );
        let reloaded =
            op_host_services::doc_io::load_editor_state(&output, op_editor_core::Locale::EnUs)
                .expect("load output");
        assert_eq!(reloaded.doc, original.doc);
        std::fs::remove_dir_all(directory).expect("remove fixture directory");
    }

    #[test]
    fn explicit_generate_fails_without_becoming_search_and_preserves_output() {
        let unique = format!(
            "openpencil-enrich-generate-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        );
        let directory = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&directory).expect("create fixture directory");
        let input = directory.join("input.op");
        let output = directory.join("output.op");
        std::fs::write(
            &input,
            r#"{"version":"1.0","children":[{"type":"image","id":"generated","name":"Generated art","src":"","imagePrompt":"paint a moonlit forest","width":160,"height":90}]}"#,
        )
        .expect("save input");
        std::fs::write(&output, b"keep-existing-output").expect("seed output");

        let result = enrich_document(&EnrichRequest {
            input,
            output: output.clone(),
            timeout: Duration::ZERO,
        });

        assert!(matches!(
            result,
            Err(EnrichError::Failed(EnrichSummary {
                pages: 1,
                targets: 1,
                resolved: 0,
                failed: 1,
                unresolved: 0,
            }))
        ));
        assert_eq!(
            std::fs::read(&output).expect("read preserved output"),
            b"keep-existing-output"
        );
        std::fs::remove_dir_all(directory).expect("remove fixture directory");
    }

    #[test]
    fn preexisting_failure_sentinel_is_counted_as_a_failed_target() {
        let source = format!(
            r#"{{
  "version": "1.0",
  "children": [
    {{
      "type": "image",
      "id": "failed",
      "name": "Failed search",
      "src": "{SEARCH_FAILED_PLACEHOLDER_SRC}",
      "width": 160,
      "height": 90
    }}
  ]
}}"#
        );
        let mut state = op_host_services::doc_io::load_editor_state_from_source(
            &source,
            op_editor_core::Locale::EnUs,
        )
        .expect("load failed-target fixture");

        let summary = enrich_state(&mut state, Duration::ZERO)
            .expect("preexisting sentinel is terminal, not pending");

        assert_eq!(
            summary,
            EnrichSummary {
                pages: 1,
                targets: 1,
                resolved: 0,
                failed: 1,
                unresolved: 0,
            }
        );
    }

    #[test]
    fn schema_warning_blocks_rewrite_and_preserves_output() {
        let unique = format!(
            "openpencil-enrich-future-schema-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        );
        let directory = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&directory).expect("create fixture directory");
        let input = directory.join("input.op");
        let output = directory.join("output.op");
        std::fs::write(
            &input,
            r#"{"version":"1.0","formatVersion":"1.3","children":[]}"#,
        )
        .expect("write future-schema input");
        std::fs::write(&output, b"keep-existing-output").expect("seed output");

        let result = enrich_document(&EnrichRequest {
            input: input.clone(),
            output: output.clone(),
            timeout: Duration::from_secs(1),
        });

        assert!(matches!(
            result,
            Err(EnrichError::RewriteBlocked { path }) if path == input
        ));
        assert_eq!(
            std::fs::read(&output).expect("read preserved output"),
            b"keep-existing-output"
        );
        std::fs::remove_dir_all(directory).expect("remove fixture directory");
    }

    #[test]
    fn invalid_document_ids_are_rejected_before_enrichment() {
        let unique = format!(
            "openpencil-enrich-invalid-ids-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        );
        let directory = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&directory).expect("create fixture directory");
        let input = directory.join("input.op");
        let output = directory.join("output.op");
        std::fs::write(
            &input,
            r#"{"version":"1.0","children":[{"type":"path","id":"duplicate","geometry":"M0 0L1 1"},{"type":"path","id":"duplicate","geometry":"M1 1L2 2"}]}"#,
        )
        .expect("write duplicate-id input");

        let result = enrich_document(&EnrichRequest {
            input: input.clone(),
            output: output.clone(),
            timeout: Duration::from_secs(1),
        });

        assert!(matches!(
            result,
            Err(EnrichError::InvalidDocument { path, message })
                if path == input && message.contains("duplicate NodeId")
        ));
        assert!(!output.exists());
        std::fs::remove_dir_all(directory).expect("remove fixture directory");
    }

    #[test]
    fn enrichment_visits_every_empty_page_without_network() {
        let source = format!(
            r#"{{
  "version": "{}",
  "pages": [
    {{"id":"p1","name":"One","children":[]}},
    {{"id":"p2","name":"Two","children":[]}}
  ]
}}"#,
            env!("CARGO_PKG_VERSION")
        );
        let mut state = op_host_services::doc_io::load_editor_state_from_source(
            &source,
            op_editor_core::Locale::EnUs,
        )
        .expect("load two-page fixture");

        let summary =
            enrich_state(&mut state, Duration::from_secs(10)).expect("headless pass completes");

        assert_eq!(
            summary,
            EnrichSummary {
                pages: 2,
                targets: 0,
                resolved: 0,
                failed: 0,
                unresolved: 0,
            }
        );
        assert_eq!(state.ui.active_page_index, 1);
    }
}
