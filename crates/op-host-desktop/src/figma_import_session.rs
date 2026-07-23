//! Two-stage background `.fig` import: prepare once, optionally wait
//! for a page choice, then convert the selected scope off-thread.

use op_editor_core::EditorState;
use op_host_native::WidgetHostNative;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::persistence::show_error_dialog_public;
use op_host_services::doc_io::{commit_staged_document, save_to_path, ErrorKind};

mod image_sources;
use image_sources::{bind_import_thumbnails, PendingImportThumbs};
mod output_guard;
pub(crate) use output_guard::{capture_output_state, OutputEntryState};
mod worker_control;
use worker_control::CancellationToken;
#[cfg(test)]
mod cancellation_tests;

/// Successful Skia-free worker output. `LayoutScene` stays on the UI thread.
pub struct PreparedImport {
    pub state: EditorState,
    pub warnings: Vec<String>,
}

type PrepareResult = Result<op_figma::PreparedFig, String>;
type ConvertResult = Result<PreparedImport, String>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ImportOutputMode {
    CreateFixed,
    ReplaceFixed { expected: OutputEntryState },
    NumberedCopy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExistingOutputDecision {
    Replace,
    NumberedCopy,
    Cancel,
}

struct PersistedFile {
    // Publication is durable once the completed sibling atomically replaces
    // the fixed output. If UI ownership changes in the tiny post-publish race,
    // leave that valid file in place; deleting by path could race another
    // importer replacing the same destination.
    output_path: PathBuf,
}

impl PersistedFile {
    fn new(output_path: PathBuf) -> Self {
        Self { output_path }
    }

    fn path(&self) -> &Path {
        &self.output_path
    }

    fn commit(self) -> PathBuf {
        self.output_path
    }
}

struct CompletedImport {
    prepared: PreparedImport,
    persisted: Result<PersistedFile, String>,
}

type PersistResult = Result<CompletedImport, String>;

static IMPORT_OUTPUT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

enum SessionStage {
    Preparing(Receiver<PrepareResult>),
    AwaitingSelection(Option<op_figma::PreparedFig>),
    Converting(Receiver<PersistResult>),
}

/// One import; its prepared tree is consumed once after page selection.
pub struct FigmaImportSession {
    path: PathBuf,
    stage: SessionStage,
    cancellation: CancellationToken,
    output_mode: ImportOutputMode,
}

impl FigmaImportSession {
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_worker_pending(&self) -> bool {
        matches!(
            self.stage,
            SessionStage::Preparing(_) | SessionStage::Converting(_)
        )
    }
}

impl Drop for FigmaImportSession {
    fn drop(&mut self) {
        self.cancellation.cancel();
    }
}

fn select_output_mode(
    source_path: &Path,
    choose_existing: impl FnOnce(&Path) -> ExistingOutputDecision,
) -> Result<Option<ImportOutputMode>, String> {
    let output_path = adjacent_op_base_path(source_path)?;
    if capture_output_state(&output_path)?.is_missing() {
        return Ok(Some(ImportOutputMode::CreateFixed));
    }
    Ok(match choose_existing(&output_path) {
        ExistingOutputDecision::Replace => Some(ImportOutputMode::ReplaceFixed {
            // Capture after the modal returns Yes so edits made while the
            // dialog was open are part of the state the user approved.
            expected: capture_output_state(&output_path)?,
        }),
        ExistingOutputDecision::NumberedCopy => Some(ImportOutputMode::NumberedCopy),
        ExistingOutputDecision::Cancel => None,
    })
}

pub(crate) fn prompt_output_mode(
    host: &WidgetHostNative,
    source_path: &Path,
) -> Option<ImportOutputMode> {
    let locale = host.editor_state().editor_ui.locale;
    match select_output_mode(source_path, |output_path| {
        let name = output_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| output_path.display().to_string());
        let body = op_i18n::translate(locale, "figma.overwriteBody").replace("{{name}}", &name);
        match rfd::MessageDialog::new()
            .set_title(op_i18n::translate(locale, "figma.overwriteTitle"))
            .set_description(&body)
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show()
        {
            rfd::MessageDialogResult::Yes => ExistingOutputDecision::Replace,
            rfd::MessageDialogResult::No => ExistingOutputDecision::NumberedCopy,
            _ => ExistingOutputDecision::Cancel,
        }
    }) {
        Ok(mode) => mode,
        Err(error) => {
            show_error_dialog_public(host, ErrorKind::Save, Some(source_path), &error);
            None
        }
    }
}

/// Spawn an import whose adjacent-file decision has already been approved.
/// Keeping the prompt outside this function lets callers leave the current
/// import and document untouched when the user cancels the dialog.
pub(crate) fn spawn_approved(
    host: &mut WidgetHostNative,
    path: PathBuf,
    output_mode: ImportOutputMode,
) -> FigmaImportSession {
    let (tx, rx) = mpsc::channel();
    let cancellation = CancellationToken::default();
    // The overlay reads this UI flag directly. Do not dirty the old document's
    // layout: the import will replace `editor_state` whole-cloth.
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.import_source = op_editor_core::figma_import_state::ImportSource::Figma;
    ui.figma_import_open = false;
    ui.figma_import_in_progress = true;
    clear_page_selector(ui);

    let path_for_thread = path.clone();
    let worker_cancellation = cancellation.clone();
    thread::Builder::new()
        .name("op-figma-import".into())
        .spawn(move || {
            let Some(_permit) = worker_cancellation.worker_permit() else {
                return;
            };
            let result = prepare_path(&path_for_thread, &worker_cancellation);
            // Recv side may be gone if the user closed the app
            // mid-parse; tolerate the SendError silently.
            if !worker_cancellation.is_cancelled() {
                let _ = tx.send(result);
            }
        })
        .expect("spawn op-figma-import worker");

    FigmaImportSession {
        path,
        stage: SessionStage::Preparing(rx),
        cancellation,
        output_mode,
    }
}

fn prepare_path(path: &Path, cancellation: &CancellationToken) -> PrepareResult {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if cancellation.is_cancelled() {
        return Err("Figma import was cancelled".to_string());
    }
    let file_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Figma Import");
    let prepared =
        op_figma::prepare_fig_binary(&bytes, file_name, op_figma::FigLayoutMode::Preserve)
            .map_err(|e| e.to_string())?;
    if cancellation.is_cancelled() {
        return Err("Figma import was cancelled".to_string());
    }
    Ok(prepared)
}

fn convert_prepared(
    prepared: op_figma::PreparedFig,
    selection: op_editor_core::FigmaImportSelection,
    cancellation: &CancellationToken,
) -> ConvertResult {
    if cancellation.is_cancelled() {
        return Err("Figma import was cancelled".to_string());
    }
    let pending_thumbs = RefCell::new(PendingImportThumbs::default());
    let transform = |bytes: &[u8]| {
        if cancellation.is_cancelled() {
            return Some(Vec::new());
        }
        let prepared = crate::image_downscale::prepare_figma_import_image(bytes);
        if cancellation.is_cancelled() {
            return Some(Vec::new());
        }
        if let Some(thumb) = prepared.thumbnail {
            let final_bytes = prepared.replacement.as_deref().unwrap_or(bytes);
            pending_thumbs.borrow_mut().record(final_bytes, thumb);
        }
        prepared.replacement
    };
    let import = match selection {
        op_editor_core::FigmaImportSelection::Page(index) => {
            prepared.into_page_with_images(index, Some(&transform))
        }
        op_editor_core::FigmaImportSelection::All => {
            prepared.into_all_pages_with_images(Some(&transform))
        }
        op_editor_core::FigmaImportSelection::Cancel => {
            return Err("Figma import was cancelled".to_string());
        }
    }
    .map_err(|e| e.to_string())?;
    if cancellation.is_cancelled() {
        return Err("Figma import was cancelled".to_string());
    }
    let mut state = EditorState::from_document(import.document);
    if cancellation.is_cancelled() {
        return Err("Figma import was cancelled".to_string());
    }
    bind_import_thumbnails(&state.doc, &mut pending_thumbs.borrow_mut());
    state.editor_ui.preserve_authored_geometry = true;
    Ok(PreparedImport {
        state,
        warnings: import.warnings,
    })
}

#[cfg(test)]
fn parse_path(path: &Path) -> Result<PreparedImport, String> {
    let cancellation = CancellationToken::default();
    convert_prepared(
        prepare_path(path, &cancellation)?,
        op_editor_core::FigmaImportSelection::All,
        &cancellation,
    )
}

fn conversion_receiver(
    prepared: op_figma::PreparedFig,
    selection: op_editor_core::FigmaImportSelection,
    cancellation: CancellationToken,
    source_path: PathBuf,
    output_mode: ImportOutputMode,
) -> Receiver<PersistResult> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("op-figma-convert".into())
        .spawn(move || {
            let Some(_permit) = cancellation.worker_permit() else {
                return;
            };
            let result = convert_prepared(prepared, selection, &cancellation).map(|prepared| {
                persist_import_next_to_source(prepared, &source_path, output_mode, &cancellation)
            });
            if !cancellation.is_cancelled() {
                let _ = tx.send(result);
            }
        })
        .expect("spawn op-figma-convert worker");
    rx
}

/// The primary imported document is the fixed sibling `Design.op`. Re-imports
/// replace it only after confirmation; keeping both publishes `Design (N).op`.
fn adjacent_op_base_path(source_path: &Path) -> Result<PathBuf, String> {
    if source_path.file_name().is_none() {
        return Err("Figma import path has no file name".to_string());
    }
    let output_path = source_path.with_extension("op");
    if output_path == source_path {
        return Err("Figma import source already has the .op extension".to_string());
    }
    Ok(output_path)
}

fn import_staging_path(source_path: &Path) -> Result<PathBuf, String> {
    let base = adjacent_op_base_path(source_path)?;
    let parent = base.parent().unwrap_or_else(|| Path::new(""));
    let name = base
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "Figma Import.op".into());
    for _ in 0..100 {
        let sequence = IMPORT_OUTPUT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".{name}.op-import-{}-{sequence}",
            std::process::id()
        ));
        if !candidate.try_exists().map_err(|error| {
            format!(
                "could not inspect import staging path {}: {error}",
                candidate.display()
            )
        })? {
            return Ok(candidate);
        }
    }
    Err(format!(
        "could not allocate an import staging file beside {}",
        source_path.display()
    ))
}

fn remove_import_artifact(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => eprintln!(
            "[import-figma] could not remove staging artifact {}: {error}",
            path.display()
        ),
    }
}

fn remove_legacy_sidecar(path: &Path) {
    match std::fs::remove_file(op_host_services::doc_io::sidecar_path(path)) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => eprintln!(
            "[import-figma] stale view-state sidecar cleanup failed for {}: {error}",
            path.display()
        ),
    }
}

fn publish_new_link(staging_path: &Path, output_path: &Path) -> Result<PathBuf, std::io::Error> {
    std::fs::hard_link(staging_path, output_path)?;
    remove_legacy_sidecar(output_path);
    Ok(output_path.to_path_buf())
}

fn publish_numbered_copy(staging_path: &Path, source_path: &Path) -> Result<PathBuf, String> {
    let base = adjacent_op_base_path(source_path)?;
    let parent = base.parent().unwrap_or_else(|| Path::new(""));
    let stem = base
        .file_stem()
        .map(|stem| stem.to_string_lossy())
        .unwrap_or_else(|| "Figma Import".into());
    for suffix in 1..=10_000 {
        let candidate = parent.join(format!("{stem} ({suffix}).op"));
        match publish_new_link(staging_path, &candidate) {
            Ok(path) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "could not publish converted OpenPencil document {}: {error}",
                    candidate.display()
                ));
            }
        }
    }
    Err(format!(
        "could not find an unused OP file name beside {}",
        source_path.display()
    ))
}

fn publish_staged_op(
    staging_path: &Path,
    source_path: &Path,
    output_mode: ImportOutputMode,
    cancellation: &CancellationToken,
) -> Result<PathBuf, String> {
    let output_path = adjacent_op_base_path(source_path)?;
    if cancellation.is_cancelled() {
        return Err("Figma import was cancelled".to_string());
    }
    match output_mode {
        ImportOutputMode::ReplaceFixed { expected } => {
            let unchanged = capture_output_state(&output_path)
                .map(|current| current == expected)
                .unwrap_or_else(|error| {
                    eprintln!(
                        "[import-figma] fixed output could not be revalidated; preserving it: {error}"
                    );
                    false
                });
            if !unchanged {
                // Consent applied to the exact entry observed after Yes. A
                // later edit, replacement, deletion, or creation is a new
                // state; preserve it and publish without another worker-side
                // prompt.
                return publish_numbered_copy(staging_path, source_path);
            }
            if expected.is_missing() {
                return match publish_new_link(staging_path, &output_path) {
                    Ok(path) => Ok(path),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        publish_numbered_copy(staging_path, source_path)
                    }
                    Err(error) => Err(format!(
                        "could not publish converted OpenPencil document {}: {error}",
                        output_path.display()
                    )),
                };
            }
            commit_staged_document(staging_path, &output_path).map_err(|error| {
                format!(
                    "could not publish converted OpenPencil document {}: {error}",
                    output_path.display()
                )
            })?;
            Ok(output_path)
        }
        ImportOutputMode::NumberedCopy => publish_numbered_copy(staging_path, source_path),
        ImportOutputMode::CreateFixed => match publish_new_link(staging_path, &output_path) {
            Ok(path) => Ok(path),
            // A fixed file appeared after the initial existence check. Never
            // overwrite it without consent; preserve both with a numbered copy.
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                publish_numbered_copy(staging_path, source_path)
            }
            Err(error) => Err(format!(
                "could not publish converted OpenPencil document {}: {error}",
                output_path.display()
            )),
        },
    }
}

/// Persist on the conversion worker before moving the imported state to the
/// UI thread. This keeps the large canonical serialization off the event loop
/// and makes the returned path a truthful, already-committed `current_path`.
fn persist_import_next_to_source(
    prepared: PreparedImport,
    source_path: &Path,
    output_mode: ImportOutputMode,
    cancellation: &CancellationToken,
) -> CompletedImport {
    let persisted = (|| {
        if cancellation.is_cancelled() {
            return Err("Figma import was cancelled".to_string());
        }
        let staging_path = import_staging_path(source_path)?;
        if let Err(error) = save_to_path(&prepared.state, &staging_path) {
            remove_import_artifact(&staging_path);
            return Err(format!(
                "could not write converted OpenPencil document beside {}: {error}",
                source_path.display()
            ));
        }
        if cancellation.is_cancelled() {
            remove_import_artifact(&staging_path);
            return Err("Figma import was cancelled".to_string());
        }
        let output_path = publish_staged_op(&staging_path, source_path, output_mode, cancellation);
        remove_import_artifact(&staging_path);
        Ok(PersistedFile::new(output_path?))
    })();
    CompletedImport {
        prepared,
        persisted,
    }
}

fn clear_page_selector(ui: &mut op_editor_core::editor_ui_state::EditorUiState) {
    ui.figma_import_pages.clear();
    ui.figma_import_page_select = Default::default();
    ui.figma_import_hover = None;
}

fn start_conversion(
    host: &mut WidgetHostNative,
    session: &mut FigmaImportSession,
    prepared: op_figma::PreparedFig,
    selection: op_editor_core::FigmaImportSelection,
) {
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.figma_import_open = false;
    ui.figma_import_in_progress = true;
    clear_page_selector(ui);
    let source_path = session.path.clone();
    session.stage = SessionStage::Converting(conversion_receiver(
        prepared,
        selection,
        session.cancellation.clone(),
        source_path,
        session.output_mode,
    ));
    host.mark_editor_state_dirty();
}

/// Consume the prepared tree after a modal page/all/cancel choice.
/// Returns true when an awaiting session handled the choice.
pub fn finish_selection(
    host: &mut WidgetHostNative,
    session: &mut Option<FigmaImportSession>,
    selection: op_editor_core::FigmaImportSelection,
) -> bool {
    if selection == op_editor_core::FigmaImportSelection::Cancel {
        let handled = session.is_some();
        cancel(host, session);
        return handled;
    }
    let Some(sess) = session.as_mut() else {
        return false;
    };
    let SessionStage::AwaitingSelection(prepared) = &mut sess.stage else {
        return false;
    };
    let Some(prepared) = prepared.take() else {
        return false;
    };
    start_conversion(host, sess, prepared, selection);
    true
}

fn apply_completed_import<F>(
    host: &mut WidgetHostNative,
    completed: CompletedImport,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
    report_save_failure: F,
) -> PumpOutcome
where
    F: FnOnce(&WidgetHostNative, &str),
{
    let CompletedImport {
        mut prepared,
        persisted,
    } = completed;
    for warning in &prepared.warnings {
        eprintln!("[import-figma] warning: {warning}");
    }
    if let Ok(saved) = &persisted {
        op_host_services::doc_io::set_file_name_display(&mut prepared.state, Some(saved.path()));
        prepared.state.mark_saved_revision();
    } else {
        // Conversion succeeded but the generated OP did not. Keep the
        // imported design open as unsaved work so Save routes through Save As
        // and the close prompt cannot discard it silently.
        prepared.state.mark_document_changed();
    }
    // Swap in the parsed state. The worker deliberately did not build a
    // LayoutScene because that touches Skia / FontMgr and can block the
    // main-thread progress overlay.
    host.install_imported_state_with_drop_hook(prepared.state, || {
        crate::heap_pressure::schedule_relief("Figma replaced-document drop");
    });
    match persisted {
        Ok(saved) => {
            let output_path = saved.commit();
            crate::settings_io::touch_recent(host, &output_path);
            *current_path = Some(output_path);
            refresh_title(current_path, window);
            PumpOutcome::CompletedSaved
        }
        Err(error) => {
            eprintln!("[import-figma] adjacent OP save failed: {error}");
            *current_path = None;
            refresh_title(current_path, window);
            let detail = format!(
                "{error}\n\nThe imported design is open but unsaved. Use Save As to choose another location."
            );
            report_save_failure(host, &detail);
            PumpOutcome::CompletedOk
        }
    }
}

/// Non-blocking session drain. A successful conversion applies the imported
/// document and binds its generated OP path. A conversion failure leaves the
/// old document in place; an adjacent-file write failure installs the result
/// as dirty, clears `current_path`, and asks the user to Save As. Every terminal
/// outcome clears the `*session` slot.
pub fn pump(
    host: &mut WidgetHostNative,
    session: &mut Option<FigmaImportSession>,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> PumpOutcome {
    let Some(sess) = session.as_mut() else {
        return PumpOutcome::Idle;
    };
    enum Event {
        Prepared(op_figma::PreparedFig),
        Converted(Box<CompletedImport>),
        Error(String),
        Disconnected,
        Pending,
        Cancelled,
    }
    let event = match &mut sess.stage {
        SessionStage::Preparing(rx) => match rx.try_recv() {
            Ok(Ok(prepared)) => Event::Prepared(prepared),
            Ok(Err(error)) => Event::Error(error),
            Err(TryRecvError::Empty) => Event::Pending,
            Err(TryRecvError::Disconnected) => Event::Disconnected,
        },
        SessionStage::AwaitingSelection(_) => {
            if host.editor_state().editor_ui.figma_import_open {
                Event::Pending
            } else {
                Event::Cancelled
            }
        }
        SessionStage::Converting(rx) => match rx.try_recv() {
            Ok(Ok(prepared)) => Event::Converted(Box::new(prepared)),
            Ok(Err(error)) => Event::Error(error),
            Err(TryRecvError::Empty) => Event::Pending,
            Err(TryRecvError::Disconnected) => Event::Disconnected,
        },
    };
    match event {
        Event::Prepared(prepared) if prepared.pages().len() > 1 => {
            let pages = prepared
                .pages()
                .iter()
                .map(|page| op_editor_core::FigmaImportPage {
                    name: page.name.clone(),
                    layer_count: page.child_count,
                })
                .collect();
            let ui = &mut host.editor_state_mut().editor_ui;
            ui.import_source = op_editor_core::figma_import_state::ImportSource::Figma;
            ui.figma_import_pages = pages;
            ui.figma_import_page_select = Default::default();
            ui.figma_import_page_select.open = true;
            ui.figma_import_open = true;
            ui.figma_import_in_progress = false;
            sess.stage = SessionStage::AwaitingSelection(Some(prepared));
            host.mark_editor_state_dirty();
            PumpOutcome::SelectionReady
        }
        Event::Prepared(prepared) => {
            start_conversion(
                host,
                sess,
                prepared,
                op_editor_core::FigmaImportSelection::All,
            );
            PumpOutcome::StillPending
        }
        Event::Converted(completed) => {
            let outcome =
                apply_completed_import(host, *completed, current_path, window, |host, detail| {
                    show_error_dialog_public(host, ErrorKind::Save, None, detail);
                });
            *session = None;
            outcome
        }
        Event::Error(e) => {
            eprintln!("[import-figma] {e}");
            show_error_dialog_public(host, ErrorKind::Open, Some(&sess.path), &e);
            host.editor_state_mut().editor_ui.figma_import_in_progress = false;
            host.mark_editor_state_dirty();
            *session = None;
            PumpOutcome::CompletedErr
        }
        Event::Pending => PumpOutcome::StillPending,
        Event::Cancelled => {
            cancel(host, session);
            PumpOutcome::Cancelled
        }
        Event::Disconnected => {
            // Worker thread panicked or dropped without sending —
            // pop the same native dialog the explicit error path
            // uses so the user gets feedback instead of a silently
            // vanishing progress overlay.
            eprintln!("[import-figma] worker thread terminated without sending a result");
            let detail = "Figma import worker exited unexpectedly";
            show_error_dialog_public(host, ErrorKind::Open, Some(&sess.path), detail);
            host.editor_state_mut().editor_ui.figma_import_in_progress = false;
            host.mark_editor_state_dirty();
            *session = None;
            PumpOutcome::CompletedErr
        }
    }
}

/// Cancel the active session and clear its UI. A non-interruptible decode may
/// finish, but its token suppresses later phases and the worker gate prevents
/// a replacement import from overlapping it in memory.
pub fn cancel(host: &mut WidgetHostNative, session: &mut Option<FigmaImportSession>) {
    let had_session = session.is_some();
    let ui_needs_clear = {
        let ui = &host.editor_state().editor_ui;
        ui.figma_import_in_progress
            || !ui.figma_import_pages.is_empty()
            || ui.figma_import_page_select.open
    };
    if had_session {
        eprintln!("[import-figma] cancelling in-flight session — superseded");
        session.as_ref().unwrap().cancellation.cancel();
        *session = None;
    }
    if had_session || ui_needs_clear {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.figma_import_in_progress = false;
        ui.figma_import_open = false;
        if matches!(
            ui.pending_file_action,
            Some(op_editor_core::FileAction::FinishFigmaImport(_))
        ) {
            ui.pending_file_action = None;
        }
        clear_page_selector(ui);
        host.mark_editor_state_dirty();
    }
}

/// Outcome of `pump` — used by the caller to decide whether to mark
/// the next frame dirty and whether to reset the document-path state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PumpOutcome {
    /// No active session.
    Idle,
    /// Worker thread still running.
    StillPending,
    /// Preparation finished and the page selector opened.
    SelectionReady,
    /// The selector was dismissed and its prepared tree was dropped.
    Cancelled,
    /// Worker finished and the document was applied.
    CompletedOk,
    /// Worker persisted the imported document before applying it.
    CompletedSaved,
    /// Worker finished with an error (dialog already shown).
    CompletedErr,
}

fn refresh_title(current_path: &Option<PathBuf>, window: Option<&winit::window::Window>) {
    let Some(window) = window else { return };
    let title = match current_path.as_ref().and_then(|p| p.file_name()) {
        Some(name) => format!("{} — OpenPencil", name.to_string_lossy()),
        None => "OpenPencil".to_string(),
    };
    window.set_title(&title);
}

#[cfg(test)]
#[path = "figma_import_session/tests.rs"]
mod tests;
