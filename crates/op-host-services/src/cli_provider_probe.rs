//! Connect probes for Antigravity and Grok Build. Split from
//! `provider_probe.rs` to preserve that module's 800-line cap.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use op_ai::agent_settings_state::AgentProvider;
use op_ai::chat_models::ModelEntry;
use op_ai::chat_provider::CliName;

use crate::chat_subprocess_safety;
use crate::model_discovery::resolve_cli;
use crate::provider_probe::ProbeOutcome;

const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_PROBE_OUTPUT_BYTES: usize = 1024 * 1024;

pub fn connect_antigravity() -> ProbeOutcome {
    let Some(exe) = resolve_cli("agy") else {
        return not_installed("Antigravity CLI not found", AgentProvider::Antigravity);
    };
    let version = match cli_version(
        CliName::Antigravity,
        &exe,
        &["--version"],
        "Antigravity",
        "`agy`",
    ) {
        Ok(version) => version,
        Err(error) => return failed(&error),
    };
    let models = match query_models(
        CliName::Antigravity,
        &exe,
        "Antigravity",
        "`agy`",
        crate::cli_model_discovery::parse_antigravity_models,
    ) {
        Ok(models) => models,
        Err(error) => return failed(&error),
    };
    ProbeOutcome {
        connected: true,
        models,
        connection_info: Some("Connected via Antigravity CLI".to_string()),
        hint_path: Some("~/.gemini/antigravity-cli/settings.json".to_string()),
        version: Some(version),
        ..ProbeOutcome::default()
    }
}

pub fn connect_grok_build() -> ProbeOutcome {
    let Some(exe) = resolve_cli("grok") else {
        return not_installed("Grok Build CLI not found", AgentProvider::GrokBuild);
    };
    let version = match cli_version(
        CliName::GrokBuild,
        &exe,
        &["version"],
        "Grok Build",
        "`grok`",
    ) {
        Ok(version) => version,
        Err(error) => return failed(&error),
    };
    let models = match query_models(
        CliName::GrokBuild,
        &exe,
        "Grok Build",
        "`grok`",
        crate::cli_model_discovery::parse_grok_models,
    ) {
        Ok(models) => models,
        Err(error) => return failed(&error),
    };
    ProbeOutcome {
        connected: true,
        models,
        connection_info: Some("Connected via Grok Build CLI".to_string()),
        hint_path: Some("~/.grok/config.toml".to_string()),
        version: Some(version),
        ..ProbeOutcome::default()
    }
}

fn not_installed(error: &str, provider: AgentProvider) -> ProbeOutcome {
    ProbeOutcome {
        error: Some(error.to_string()),
        not_installed: true,
        install_command: Some(crate::provider_probe::install_command(provider).to_string()),
        ..ProbeOutcome::default()
    }
}

fn failed(error: &str) -> ProbeOutcome {
    ProbeOutcome {
        error: Some(error.to_string()),
        ..ProbeOutcome::default()
    }
}

fn cli_version(
    cli: CliName,
    exe: &Path,
    args: &[&str],
    provider: &str,
    login_command: &str,
) -> Result<String, String> {
    match bounded_cli_output(cli, exe, args, PROBE_TIMEOUT) {
        BoundedProbe::Completed(output) => {
            if !output.status.success() {
                return Err(format!("{provider} CLI exited with an error"));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let version = if stdout.trim().is_empty() {
                stderr.trim()
            } else {
                stdout.trim()
            };
            if version.is_empty() {
                Err(format!("{provider} CLI produced no version output"))
            } else {
                Ok(version.to_string())
            }
        }
        BoundedProbe::TimedOut { stdout, stderr } => Err(diagnose_timeout(
            cli,
            provider,
            login_command,
            PROBE_TIMEOUT,
            &stdout,
            &stderr,
        )),
        BoundedProbe::Failed => Err(format!("{provider} CLI not responding")),
    }
}

fn query_models(
    cli: CliName,
    exe: &Path,
    provider: &str,
    login_command: &str,
    parse: fn(&str) -> Vec<ModelEntry>,
) -> Result<Vec<ModelEntry>, String> {
    let output = match bounded_cli_output(cli, exe, &["models"], PROBE_TIMEOUT) {
        BoundedProbe::Completed(output) => output,
        BoundedProbe::TimedOut { stdout, stderr } => {
            return Err(diagnose_timeout(
                cli,
                provider,
                login_command,
                PROBE_TIMEOUT,
                &stdout,
                &stderr,
            ))
        }
        BoundedProbe::Failed => return Err(format!("{provider} model query failed or timed out")),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        if let Some(message) = chat_subprocess_safety::friendly_stderr_error(Some(cli), &stderr) {
            return Err(message);
        }
        return Err(if stderr.trim().is_empty() {
            format!("{provider} model query failed. Run {login_command} once to authenticate.")
        } else {
            stderr.trim().to_string()
        });
    }

    let models = parse(&stdout);
    if !models.is_empty() {
        return Ok(models);
    }
    Err(catalog_error(provider, login_command, &stdout, &stderr))
}

fn catalog_error(provider: &str, login_command: &str, stdout: &str, stderr: &str) -> String {
    let diagnostics = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    let auth_required = [
        "sign in",
        "signin",
        "log in",
        "login",
        "authenticate",
        "authentication",
        "unauthorized",
        "credential",
        "api key",
    ]
    .iter()
    .any(|marker| diagnostics.contains(marker));
    if auth_required {
        format!(
            "{provider} model query requires authentication. Run {login_command} once to sign in."
        )
    } else if stdout.trim().is_empty() {
        format!("{provider} model query returned no model catalog")
    } else {
        format!("{provider} returned an unrecognized model catalog")
    }
}

/// How much of a timed-out probe's captured output to surface verbatim when
/// no known auth/permission marker matched, so a Settings-card error is at
/// least diagnosable instead of a bare "timed out".
const TIMEOUT_TAIL_CHARS: usize = 200;

/// Turn a timed-out probe's retained stdout/stderr into an actionable
/// message. Checked first against the same auth-prompt vocabulary a
/// completed, non-zero-exit probe uses (`friendly_stdout_error` /
/// `friendly_stderr_error`) — a CLI that is mid first-run OAuth typically
/// never exits within the probe budget, so the auth prompt only ever shows
/// up here, never on the completed-output path. Falls back to a generic
/// timeout message carrying a truncated tail of whatever the CLI printed.
fn diagnose_timeout(
    cli: CliName,
    provider: &str,
    login_command: &str,
    timeout: Duration,
    stdout: &[u8],
    stderr: &[u8],
) -> String {
    let stdout_text = String::from_utf8_lossy(stdout);
    let stderr_text = String::from_utf8_lossy(stderr);
    for line in stdout_text.lines() {
        if let Some(message) = chat_subprocess_safety::friendly_stdout_error(Some(cli), line) {
            return message;
        }
    }
    if let Some(message) = chat_subprocess_safety::friendly_stderr_error(Some(cli), &stderr_text) {
        return message;
    }
    let tail = tail_snippet(&stdout_text, &stderr_text);
    let timeout_secs = timeout.as_secs();
    if tail.is_empty() {
        format!(
            "{provider} CLI timed out after {timeout_secs}s with no output. \
             Run {login_command} once in a terminal to authenticate."
        )
    } else {
        format!(
            "{provider} CLI timed out after {timeout_secs}s. \
             Run {login_command} once in a terminal to authenticate. Last output: {tail}"
        )
    }
}

/// Last `TIMEOUT_TAIL_CHARS` characters of the combined stdout+stderr text,
/// trimmed — the freshest signal from a hung CLI, capped so a chatty
/// process can't blow up the Settings card's error text.
fn tail_snippet(stdout: &str, stderr: &str) -> String {
    let combined = format!("{stdout}{stderr}");
    let trimmed = combined.trim();
    let char_count = trimmed.chars().count();
    if char_count <= TIMEOUT_TAIL_CHARS {
        trimmed.to_string()
    } else {
        trimmed
            .chars()
            .skip(char_count - TIMEOUT_TAIL_CHARS)
            .collect()
    }
}

/// Outcome of a bounded, piped-output CLI probe.
enum BoundedProbe {
    /// The process exited within the timeout; `Output` carries whatever
    /// stdout/stderr was captured up to `MAX_PROBE_OUTPUT_BYTES`.
    Completed(Output),
    /// The process was still running at the deadline (or `try_wait` errored)
    /// and was killed. Carries whatever stdout/stderr was captured before
    /// the kill — a CLI waiting on first-run OAuth typically already
    /// printed its auth prompt by then.
    TimedOut { stdout: Vec<u8>, stderr: Vec<u8> },
    /// Never got a running process to observe (env lookup, spawn, or pipe
    /// setup failed) — no output exists to retain.
    Failed,
}

/// Run a connection/version/model probe with the same explicit environment
/// policy as a real chat turn. This is intentionally separate from the legacy
/// catalog runner: an `env_clear` is essential here so Settings probes cannot
/// expose unrelated host secrets to a third-party coding-agent CLI.
fn bounded_cli_output(cli: CliName, exe: &Path, args: &[&str], timeout: Duration) -> BoundedProbe {
    let Some(env) = chat_subprocess_safety::child_env(Some(cli)) else {
        return BoundedProbe::Failed;
    };
    let mut command = Command::new(exe);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(env);
    crate::chat_spawn::hide_console_window(&mut command);
    let Ok(mut child) = command.spawn() else {
        return BoundedProbe::Failed;
    };
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        return BoundedProbe::Failed;
    };
    let stdout_reader = drain_pipe(stdout);
    let stderr_reader = drain_pipe(stderr);
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return BoundedProbe::Completed(Output {
                    status,
                    stdout: stdout_reader.join().unwrap_or_default(),
                    stderr: stderr_reader.join().unwrap_or_default(),
                });
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return BoundedProbe::TimedOut {
                    stdout: stdout_reader.join().unwrap_or_default(),
                    stderr: stderr_reader.join().unwrap_or_default(),
                };
            }
        }
    }
}

/// Continue draining after the retained output cap so a verbose CLI cannot
/// fill an OS pipe and deadlock the bounded probe.
fn drain_pipe<R>(mut pipe: R) -> JoinHandle<Vec<u8>>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut retained = Vec::new();
        let mut chunk = [0_u8; 8192];
        loop {
            let Ok(count) = pipe.read(&mut chunk) else {
                break;
            };
            if count == 0 {
                break;
            }
            let remaining = MAX_PROBE_OUTPUT_BYTES.saturating_sub(retained.len());
            retained.extend_from_slice(&chunk[..count.min(remaining)]);
        }
        retained
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai::agent_settings_state::AgentProvider;

    #[test]
    fn not_installed_outcome_carries_provider_guidance() {
        let outcome = not_installed("missing", AgentProvider::Antigravity);
        assert!(outcome.not_installed);
        assert_eq!(outcome.error.as_deref(), Some("missing"));
        assert_eq!(
            outcome.install_command.as_deref(),
            Some(crate::provider_probe::install_command(
                AgentProvider::Antigravity
            ))
        );
    }

    #[test]
    fn provider_variants_are_the_expected_catalog_owners() {
        assert_eq!(
            crate::cli_model_discovery::antigravity_default_model()[0].provider,
            AgentProvider::Antigravity
        );
    }

    #[test]
    fn empty_catalog_error_distinguishes_auth_from_bad_output() {
        assert!(catalog_error("Grok Build", "`grok`", "", "login required")
            .contains("requires authentication"));
        assert_eq!(
            catalog_error("Grok Build", "`grok`", "unexpected prose", ""),
            "Grok Build returned an unrecognized model catalog"
        );
    }

    #[test]
    fn tail_snippet_truncates_to_last_n_chars() {
        let long = "a".repeat(500);
        let tail = tail_snippet(&long, "");
        assert_eq!(tail.len(), TIMEOUT_TAIL_CHARS);
        assert!(long.ends_with(&tail));

        // Under the cap: returned verbatim, trimmed.
        assert_eq!(tail_snippet("  short  ", ""), "short");
    }

    #[test]
    fn diagnose_timeout_surfaces_antigravity_stdout_auth_prompt() {
        let message = diagnose_timeout(
            CliName::Antigravity,
            "Antigravity",
            "`agy`",
            Duration::from_secs(10),
            b"Authentication required. Please visit the URL to log in:\n",
            b"",
        );
        assert_eq!(
            message,
            "Antigravity is not authenticated. Run `agy` once in a terminal."
        );
    }

    #[test]
    fn diagnose_timeout_surfaces_grok_stderr_auth_prompt() {
        let message = diagnose_timeout(
            CliName::GrokBuild,
            "Grok Build",
            "`grok login`",
            Duration::from_secs(10),
            b"",
            b"login required to continue",
        );
        assert_eq!(
            message,
            "Grok Build is not authenticated. Run `grok login` in a terminal."
        );
    }

    #[test]
    fn diagnose_timeout_falls_back_to_truncated_tail_when_no_marker_matches() {
        let message = diagnose_timeout(
            CliName::GrokBuild,
            "Grok Build",
            "`grok login`",
            Duration::from_secs(10),
            b"initializing sandbox...\nstill working\n",
            b"",
        );
        assert!(message.contains("timed out after 10s"));
        assert!(message.contains("`grok login`"));
        assert!(message.contains("still working"));
    }

    #[test]
    fn diagnose_timeout_reports_no_output_when_nothing_was_captured() {
        let message = diagnose_timeout(
            CliName::Antigravity,
            "Antigravity",
            "`agy`",
            Duration::from_secs(10),
            b"",
            b"",
        );
        assert!(message.contains("no output"));
        assert!(message.contains("`agy`"));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_cli_output_retains_captured_bytes_on_timeout() {
        // A CLI mid first-run OAuth: prints its prompt, then hangs well
        // past the probe budget. The kill-on-deadline path must not throw
        // away what the reader threads already captured.
        let script =
            "printf 'Authentication required. Please visit the URL to log in:\\n'; sleep 5";
        match bounded_cli_output(
            CliName::Antigravity,
            Path::new("/bin/sh"),
            &["-c", script],
            Duration::from_millis(200),
        ) {
            BoundedProbe::TimedOut { stdout, .. } => {
                assert!(String::from_utf8_lossy(&stdout).contains("Authentication required"));
            }
            BoundedProbe::Completed(_) => panic!("expected the sleep to outlast the timeout"),
            BoundedProbe::Failed => panic!("expected a running process to observe"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn bounded_cli_output_completes_normally_when_process_exits_in_time() {
        // Success path must behave exactly as before: a process that
        // exits inside the budget yields Completed with its output.
        match bounded_cli_output(
            CliName::GrokBuild,
            Path::new("/bin/sh"),
            &["-c", "echo hello"],
            Duration::from_secs(5),
        ) {
            BoundedProbe::Completed(output) => {
                assert!(output.status.success());
                assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
            }
            _ => panic!("expected Completed, not a timeout/failure path"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn cli_version_pipeline_surfaces_actionable_message_when_version_check_hangs() {
        // Regression test for the 0.8.1 Antigravity connect failure,
        // exercised through the exact two-step pipeline `cli_version` runs
        // internally (bounded_cli_output → diagnose_timeout on the
        // TimedOut branch): a CLI stuck on first-run OAuth during
        // `agy --version` used to surface only "CLI not responding"; it
        // must now name the fix.
        let script =
            "printf 'Authentication required. Please visit the URL to log in:\\n'; sleep 5";
        let probe_timeout = Duration::from_millis(200);
        let probe = bounded_cli_output(
            CliName::Antigravity,
            Path::new("/bin/sh"),
            &["-c", script],
            probe_timeout,
        );
        let BoundedProbe::TimedOut { stdout, stderr } = probe else {
            panic!("expected the sleep to outlast the timeout");
        };
        let message = diagnose_timeout(
            CliName::Antigravity,
            "Antigravity",
            "`agy`",
            probe_timeout,
            &stdout,
            &stderr,
        );
        assert_eq!(
            message,
            "Antigravity is not authenticated. Run `agy` once in a terminal."
        );
    }
}
