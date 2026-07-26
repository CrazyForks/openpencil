//! Shared bounded-subprocess plumbing for CLI connect probes
//! (`cli_provider_probe.rs`) and CLI model discovery
//! (`cli_model_discovery.rs`). Both run a short-lived external CLI with a
//! deadline and need the same "don't throw away captured output on timeout"
//! behavior, so the run/kill/diagnose logic lives here once instead of
//! forking in each caller.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use op_ai::chat_provider::CliName;

use crate::chat_subprocess_safety;

pub(crate) const MAX_PROBE_OUTPUT_BYTES: usize = 1024 * 1024;

/// How much of a timed-out probe's captured output to surface verbatim when
/// no known auth/permission marker matched, so a Settings-card error is at
/// least diagnosable instead of a bare "timed out".
const TIMEOUT_TAIL_CHARS: usize = 200;

/// Outcome of a bounded, piped-output CLI probe.
pub(crate) enum BoundedProbe {
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
pub(crate) fn bounded_cli_output(
    cli: CliName,
    exe: &Path,
    args: &[&str],
    timeout: Duration,
) -> BoundedProbe {
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

/// Turn a timed-out probe's retained stdout/stderr into an actionable
/// message. Checked first against the same auth-prompt vocabulary a
/// completed, non-zero-exit probe uses (`friendly_stdout_error` /
/// `friendly_stderr_error`) — a CLI that is mid first-run OAuth typically
/// never exits within the probe budget, so the auth prompt only ever shows
/// up here, never on the completed-output path. Falls back to a generic
/// timeout message carrying a truncated tail of whatever the CLI printed.
pub(crate) fn diagnose_timeout(
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
pub(crate) fn tail_snippet(stdout: &str, stderr: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Starting probe budget for the hung-CLI tests, and the ceiling
    /// [`timed_out_probe_with_output`] may escalate to.
    ///
    /// Test-harness numbers only — no production timeout reads them. These
    /// tests assert that the deadline is reached WITH the script's output
    /// already captured, which races process startup: a tight window lets a
    /// loaded machine reach the deadline before the shell's `printf` runs,
    /// and the capture comes back empty. So the tests start at the floor and
    /// retry with a doubled budget while the capture is empty, up to the cap
    /// (same approach as `cli_model_discovery_tests`).
    #[cfg(unix)]
    const PROBE_BUDGET: Duration = Duration::from_secs(4);
    #[cfg(unix)]
    const PROBE_BUDGET_CAP: Duration = Duration::from_secs(16);

    /// How long the fake CLI hangs after printing — comfortably past
    /// `PROBE_BUDGET_CAP` so the timeout branch is guaranteed even at full
    /// escalation, but finite so nothing can outlive the test run.
    ///
    /// The hang is spelled `exec sleep N`, not `sleep N`, on purpose: a
    /// forked `sleep` INHERITS the stdout/stderr pipes, so `child.kill()`
    /// would not close them and `bounded_cli_output`'s reader-thread `join`
    /// would block until the grandchild exited on its own — the probe would
    /// return after N seconds instead of at its deadline. `exec` replaces the
    /// shell with `sleep`, so the pid the probe kills IS the process holding
    /// the pipes.
    #[cfg(unix)]
    const FAKE_CLI_HANG_SECS: u32 = 30;

    /// Run the auth-prompt-then-hang script under an escalating deadline
    /// until the retained stdout actually holds the prompt, then hand back
    /// the captured streams plus the budget that produced them.
    ///
    /// Each attempt asserts the probe returned on its own deadline instead of
    /// outlasting the `FAKE_CLI_HANG_SECS` child, so "the probe is
    /// deadline-bounded" stays under test.
    #[cfg(unix)]
    fn timed_out_probe_with_output(cli: CliName) -> (Vec<u8>, Vec<u8>, Duration) {
        let script = format!(
            "printf 'Authentication required. Please visit the URL to log in:\\n'; \
             exec sleep {FAKE_CLI_HANG_SECS}"
        );
        let mut budget = PROBE_BUDGET;
        loop {
            let started = Instant::now();
            let probe = bounded_cli_output(cli, Path::new("/bin/sh"), &["-c", &script], budget);
            let elapsed = started.elapsed();
            let BoundedProbe::TimedOut { stdout, stderr } = probe else {
                panic!("expected the sleep to outlast the timeout");
            };
            assert!(
                elapsed < budget * 4,
                "probe must return on its own deadline ({budget:?}), not outlast the \
                 {FAKE_CLI_HANG_SECS}s script; took {elapsed:?}"
            );
            if String::from_utf8_lossy(&stdout).contains("Authentication required")
                || budget >= PROBE_BUDGET_CAP
            {
                return (stdout, stderr, budget);
            }
            budget = (budget * 2).min(PROBE_BUDGET_CAP);
        }
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
        let (stdout, _stderr, _budget) = timed_out_probe_with_output(CliName::Antigravity);
        assert!(String::from_utf8_lossy(&stdout).contains("Authentication required"));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_cli_output_drains_large_stdout_and_stderr_before_exit() {
        // A verbose CLI (e.g. a chatty catalog dump) must not deadlock the
        // probe by filling an OS pipe buffer before the process exits, and
        // draining must cap at MAX_PROBE_OUTPUT_BYTES per stream.
        let script = "i=0; while [ $i -lt 40000 ]; do \
                      printf '0123456789abcdef0123456789abcdef\\n'; \
                      printf 'fedcba9876543210fedcba9876543210\\n' >&2; \
                      i=$((i+1)); done";
        match bounded_cli_output(
            CliName::GrokBuild,
            Path::new("/bin/sh"),
            &["-c", script],
            // The script exits on its own, so a generous ceiling costs
            // nothing and keeps machine load from turning this
            // completed-output assertion into a timeout.
            PROBE_BUDGET_CAP,
        ) {
            BoundedProbe::Completed(output) => {
                assert!(output.status.success());
                assert_eq!(output.stdout.len(), MAX_PROBE_OUTPUT_BYTES);
                assert_eq!(output.stderr.len(), MAX_PROBE_OUTPUT_BYTES);
            }
            _ => panic!("large piped output should not deadlock or time out"),
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
            PROBE_BUDGET_CAP,
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
        // exercised through the exact two-step pipeline callers run
        // internally (bounded_cli_output → diagnose_timeout on the
        // TimedOut branch): a CLI stuck on first-run OAuth during
        // `agy --version` used to surface only "CLI not responding"; it
        // must now name the fix.
        let (stdout, stderr, probe_timeout) = timed_out_probe_with_output(CliName::Antigravity);
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
