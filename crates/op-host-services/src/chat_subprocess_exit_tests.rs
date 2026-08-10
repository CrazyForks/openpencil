//! Failure-diagnosability regressions for the subprocess CLI bridge.
//!
//! Every case here drives the real [`SubprocessProvider::send`] loop
//! against a stand-in binary reproducing a stdio shape measured from a
//! real CLI, and asserts on the `ChatDelta::Error` a user would see.
//!
//! Unix-only: the stand-ins are `/bin/sh` scripts. The behaviour under
//! test is platform-independent.
#![cfg(unix)]

use super::*;

/// Antigravity's unauthenticated output, measured 2026-08-07 by running
/// the production argv (`--sandbox --print-timeout 90s --mode plan`)
/// with a private `--gemini_dir` and piped stdio. Two facts this fixture
/// encodes: the whole block lands on **stderr** (piped stdout came back
/// empty, 0 bytes), and the process exits 1.
///
/// The OAuth parameters are FAKE placeholders — the shape is what the
/// redaction has to survive, and no real credential belongs in a test.
const AGY_UNAUTHENTICATED: &str = r#"#!/bin/sh
cat >&2 <<'EOT'
Authentication required. Please visit the URL to log in:
  https://accounts.google.com/o/oauth2/auth?access_type=offline&client_id=000000000000-fakefakefakefake.apps.googleusercontent.com&code_challenge=FAKECODECHALLENGE0000&code_challenge_method=S256&prompt=consent&response_type=code&state=FAKESTATE0000

Waiting for authentication (timeout 60s)...
Or, paste the authorization code here and press Enter:
Error: authentication timed out.
Error: authentication failed or timed out
EOT
exit 1
"#;

/// A failure with no keyword the classifier knows — the case that used
/// to reach the user as a bare exit status. Carries a credential in the
/// same breath, because real CLIs dump their config when they crash.
const AGY_UNCLASSIFIABLE_CRASH: &str = r#"#!/bin/sh
cat >&2 <<'EOT'
panic: runtime error: index out of range [3] with length 0
  loaded profile from /tmp/turn/gemini/settings.json
  upstream=https://agent.example.test/v1/plan?api_key=fake-key-000111222&trace=abc
goroutine 1 [running]:
main.planOnce(0x14000112000)
EOT
exit 3
"#;

/// A stand-in `agy` on disk. Returns the containing directory (for
/// cleanup) and the executable path.
fn stub_cli(body: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    // A counter, not just a timestamp: these tests run in parallel and
    // the clock is coarse enough that two of them collided on one
    // directory, so one test silently executed another's stub.
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "openpencil-exit-tests-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("agy");
    std::fs::write(&path, body).expect("write stub");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    (dir, path)
}

/// Run one generation turn against a stand-in and return the error text
/// the user would see.
///
/// Retries a spawn that races on `ETXTBSY`. Writing a stub and exec'ing
/// it from many threads at once means one thread's still-open write fd
/// can be inherited across another thread's `fork()` and briefly hold
/// the freshly-written stub open for write, so `execve` reports "Text
/// file busy". That is an artifact of the parallel write-then-exec
/// harness, not the stderr-drain behaviour under test, and the window is
/// microseconds — a retry clears it.
fn turn_error(body: &str) -> String {
    for _ in 0..16 {
        let message = turn_error_once(body);
        if !message.contains("Text file busy") && !message.contains("os error 26") {
            return message;
        }
    }
    turn_error_once(body)
}

fn turn_error_once(body: &str) -> String {
    let (dir, binary) = stub_cli(body);
    let provider = SubprocessProvider::for_cli_generation(CliName::Antigravity)
        .expect("antigravity has a subprocess template")
        .with_test_binary(binary.to_string_lossy().into_owned());
    let deltas: Vec<ChatDelta> = provider
        .send(ChatRequest {
            user_message: "design a landing page".into(),
            ..Default::default()
        })
        .collect();
    let _ = std::fs::remove_dir_all(dir);
    assert!(
        deltas
            .iter()
            .any(|delta| matches!(delta, ChatDelta::Done { stop_reason } if *stop_reason == StopReason::Aborted)),
        "a failed turn must end Aborted, got {deltas:?}"
    );
    deltas
        .iter()
        .find_map(|delta| match delta {
            ChatDelta::Error(message) => Some(message.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected an Error delta, got {deltas:?}"))
}

#[test]
fn unauthenticated_antigravity_reads_as_an_auth_problem_not_an_exit_code() {
    let message = turn_error(AGY_UNAUTHENTICATED);
    assert!(
        message.starts_with("Antigravity is not authenticated. Run `agy` once in a terminal."),
        "the auth block arrives on stderr; classifying only stdout leaves \
         the user with a bare exit status: {message}"
    );
    assert!(!message.contains("exited with status"), "{message}");
}

#[test]
fn a_classified_failure_still_shows_what_the_cli_actually_said() {
    // The verdict and the evidence travel together. Without the tail a
    // misclassification is undetectable — the reader sees a confident
    // sentence and nothing to check it against.
    let message = turn_error(AGY_UNAUTHENTICATED);
    assert!(message.contains("Authentication required"), "{message}");
    assert!(
        message.contains("authentication failed or timed out"),
        "{message}"
    );
    // Still redacted: the quoted block carries a live OAuth URL.
    assert!(
        message.contains("accounts.google.com/o/oauth2/auth?<redacted>"),
        "{message}"
    );
    for secret in ["client_id=", "code_challenge=", "state=FAKESTATE"] {
        assert!(!message.contains(secret), "leaked {secret:?} in {message}");
    }
}

#[test]
fn unclassifiable_failure_quotes_the_child_instead_of_only_its_exit_code() {
    let message = turn_error(AGY_UNCLASSIFIABLE_CRASH);
    assert!(
        message.starts_with("CLI exited with status 3"),
        "the exit status stays in the message: {message}"
    );
    // The evidence the old fallback threw away.
    assert!(message.contains("index out of range"), "{message}");
    assert!(message.contains("goroutine 1"), "{message}");
    // …with every credential-shaped fragment scrubbed out of it.
    for secret in [
        "api_key=fake",
        "fake-key-000111222",
        "?api_key",
        "trace=abc",
    ] {
        assert!(!message.contains(secret), "leaked {secret:?} in {message}");
    }
    assert!(
        message.contains("agent.example.test/v1/plan?<redacted>"),
        "{message}"
    );
}

#[test]
fn a_silent_child_says_so_rather_than_quoting_nothing() {
    let message = turn_error("#!/bin/sh\nexit 9\n");
    assert_eq!(message, "CLI exited with status 9 (no output captured)");
}

#[test]
fn quoted_output_is_length_capped_however_much_the_child_prints() {
    // 40k lines of stderr; the surfaced message must not grow with it.
    let body = "#!/bin/sh\nawk 'BEGIN{for(i=0;i<40000;i++) \
                print \"stderr noise line \" i > \"/dev/stderr\"}'\nexit 4\n";
    let message = turn_error(body);
    assert!(
        message.chars().count() <= 64 + op_util::cli_output::TAIL_MAX_CHARS,
        "message was {} chars: {message}",
        message.chars().count()
    );
    // Bounded, but still the part that matters: the child's last words.
    assert!(message.contains("stderr noise line 39999"), "{message}");
}

#[test]
fn a_childs_stderr_is_never_lost_to_the_drain_task_still_being_in_flight() {
    // The child writes stderr and dies in the same breath, so its two
    // pipes hit EOF together and the read loop races the drain task.
    // On an idle machine the drain always wins; under real load it does
    // not, and the failure mode is silent — a child that explained
    // itself reported as `(no output captured)`. Found for real by
    // running four crates' test binaries concurrently.
    //
    // The test has to CREATE the contention rather than hope for it: on
    // an idle machine the drain wins every time and this case passes
    // even with the fix reverted. Concurrent turns saturate the shared
    // runtime's workers the way the orchestrator's parallel subtasks do,
    // and that is exactly when the tail comes back empty.
    const THREADS: usize = 16;
    const TURNS_PER_THREAD: usize = 12;
    let lost = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let mut handles = Vec::new();
    for _ in 0..THREADS {
        let lost = std::sync::Arc::clone(&lost);
        handles.push(std::thread::spawn(move || {
            for _ in 0..TURNS_PER_THREAD {
                let message = turn_error(
                    "#!/bin/sh\necho 'fatal: upstream refused the plan request' >&2\nexit 5\n",
                );
                if !message.contains("upstream refused the plan request") {
                    lost.lock().expect("not poisoned").push(message);
                }
            }
        }));
    }
    for handle in handles {
        handle.join().expect("worker thread");
    }
    let lost = lost.lock().expect("not poisoned");
    assert!(
        lost.is_empty(),
        "{} of {} turns lost the child's stderr, e.g. {:?}",
        lost.len(),
        THREADS * TURNS_PER_THREAD,
        lost.first()
    );
}

#[test]
fn a_cli_that_diagnoses_itself_on_stdout_is_quoted_too() {
    // Same failure reported on stdout instead of stderr — the stream
    // split is not a stable contract across CLIs or across TTY vs pipe.
    let body = "#!/bin/sh\necho 'fatal: workspace policy rejected the request'\nexit 2\n";
    let message = turn_error(body);
    assert!(message.starts_with("CLI exited with status 2"), "{message}");
    assert!(
        message.contains("workspace policy rejected the request"),
        "{message}"
    );
}
