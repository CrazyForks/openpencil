use super::*;

/// Serializes the three tests that spawn a real child process under a
/// deadline (`*_models_from_exe` against a `write_fake_cli` script).
///
/// Every other test in this file is pure parsing and runs freely in
/// parallel; these three are the only ones whose PASS/FAIL depends on wall
/// clock. The libtest harness runs them concurrently by default, so on a
/// loaded machine they were racing EACH OTHER for the same cores while each
/// one measured whether a fresh `exec` beat its own timeout — the exact
/// contention the escalating budget was invented to absorb. Serializing them
/// removes the one contention source this file controls (external `cargo
/// build` load is still absorbed by the escalation), so the budget only ever
/// has to cover genuine machine load, never sibling tests in the same binary.
///
/// Same shape as the established in-file guards elsewhere in the workspace
/// (`op-editor-core/src/agent_indicators_tests.rs`,
/// `op-host-native/src/widget_host/preview_edge_swipe_tests.rs`): a
/// `LazyLock<Mutex<()>>` whose guard is held for the body of the test, with
/// poisoning ignored so one failing test reports its own failure instead of
/// cascading into the siblings.
#[cfg(unix)]
static SUBPROCESS_PROBE_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

#[cfg(unix)]
fn subprocess_probe_lock() -> std::sync::MutexGuard<'static, ()> {
    SUBPROCESS_PROBE_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

#[test]
fn parses_normal_grok_ids_and_custom_aliases_from_catalog_rows() {
    let text = "Available models:\n\
                * grok-code-fast-1 (default)\n\
                * my-model (custom)\n\
                | grok-4.1-fast | ready |\n\
                | company/sonnet:prod | configured |";
    let models = parse_grok_models(text);
    assert_eq!(
        models
            .iter()
            .map(|model| model.value.as_str())
            .collect::<Vec<_>>(),
        [
            "company/sonnet:prod",
            "grok-4.1-fast",
            "grok-code-fast-1",
            "my-model",
        ]
    );

    let models =
        parse_grok_models(r#"{"models":[{"id":"grok-code-fast-1"},{"alias":"my-model"}]}"#);
    assert_eq!(
        models
            .iter()
            .map(|model| model.value.as_str())
            .collect::<Vec<_>>(),
        ["grok-code-fast-1", "my-model"]
    );
}

#[test]
fn parses_custom_aliases_from_headered_tables() {
    let text = "Model | Status\n------|-------\nmy-model | default\ngrok-4.5 | ready";
    let models = parse_grok_models(text);
    assert_eq!(
        models
            .iter()
            .map(|model| model.value.as_str())
            .collect::<Vec<_>>(),
        ["grok-4.5", "my-model"]
    );
}

#[test]
fn parses_antigravity_display_names_without_losing_effort_suffixes() {
    let text = "Available models:\n* Gemini 3.5 Flash (Medium)\n\
                * Claude Opus 4.6 (Thinking)\n* GPT-OSS 120B (Medium)";
    let models = parse_antigravity_models(text);
    assert_eq!(
        models
            .iter()
            .map(|model| model.value.as_str())
            .collect::<Vec<_>>(),
        [
            "Claude Opus 4.6 (Thinking)",
            "GPT-OSS 120B (Medium)",
            "Gemini 3.5 Flash (Medium)",
        ]
    );
    assert!(models.iter().all(|model| model.value == model.display_name));
}

#[test]
fn parses_antigravity_v1_1_5_slug_catalog() {
    let text = "gemini-3.6-flash-high\ngemini-3.6-flash-medium\ngemini-3.6-flash-low\ngemini-3.5-flash-high\ngemini-3.5-flash-medium\ngemini-3.5-flash-low\ngemini-3.1-pro-high\ngemini-3.1-pro-low\nclaude-sonnet-4-6\nclaude-opus-4-6-thinking\ngpt-oss-120b-medium";
    let models = parse_antigravity_models(text);
    assert_eq!(models.len(), 11);
    assert!(models.iter().any(|m| m.value == "gemini-3.6-flash-high"));
    assert!(models.iter().any(|m| m.value == "claude-opus-4-6-thinking"));
}

#[test]
fn parses_antigravity_json_and_ignores_auth_prose() {
    let models = parse_antigravity_models(
        r#"{"models":[{"displayName":"Gemini 3.1 Pro (High)"},{"name":"Claude Sonnet 4.6 (Thinking)"}]}"#,
    );
    assert_eq!(models.len(), 2);
    assert!(parse_antigravity_models("Please sign in to view available models").is_empty());
    assert!(parse_antigravity_models(
        "Available models:\n* Gemini authentication required\n* Claude login failed"
    )
    .is_empty());
    assert!(parse_antigravity_models(r#"{"name":"Gemini authentication required"}"#).is_empty());
}

#[test]
fn human_catalog_does_not_resume_after_its_blank_terminator() {
    let antigravity = parse_antigravity_models(
        "Available models:\n* Gemini 3.5 Flash (High)\n\n* Claude CLI troubleshooting",
    );
    assert_eq!(antigravity.len(), 1);
    assert_eq!(antigravity[0].value, "Gemini 3.5 Flash (High)");

    let grok = parse_grok_models("Available models:\n* grok-code-fast-1\n\n* release-notes-model");
    assert_eq!(grok.len(), 1);
    assert_eq!(grok[0].value, "grok-code-fast-1");
}

#[test]
fn ignores_catalog_headings_and_unrelated_prose() {
    assert!(parse_grok_models("Available models:\nDefault model: automatic").is_empty());
    assert!(parse_grok_models(
        "Available models:\nStatus: ready\nconnected\nAuthentication required"
    )
    .is_empty());
    assert!(parse_grok_models("Please sign in to continue").is_empty());
    assert!(parse_grok_models(r#""connected""#).is_empty());
    assert!(parse_grok_models(r#"{"name":"grok-diagnostic"}"#).is_empty());
    assert!(parse_grok_models("Available models:\n* request failed\n* loading-models").is_empty());
}

#[test]
fn verified_catalogs_reject_empty_auth_and_unknown_output() {
    let empty = require_antigravity_models("", "").unwrap_err().to_string();
    assert!(empty.contains("no model catalog"));

    let auth = require_antigravity_models("", "Please sign in to continue")
        .unwrap_err()
        .to_string();
    assert!(auth.contains("requires authentication"));

    let unknown = require_grok_models("Available models:\nautomatic", "")
        .unwrap_err()
        .to_string();
    assert!(unknown.contains("unrecognized model catalog"));

    let auth = require_grok_models("", "Authentication required")
        .unwrap_err()
        .to_string();
    assert!(auth.contains("requires authentication"));
}

// Large-output draining without deadlock is `bounded_cli_output`'s
// contract, exercised in `cli_probe_support`'s own test module
// (`bounded_cli_output_drains_large_stdout_and_stderr_before_exit`).

/// Starting probe budget for the hung-CLI tests, and the ceiling the
/// escalation in [`probe_until_captured`] may reach.
///
/// These are TEST HARNESS numbers only — no production timeout is derived
/// from them. What the hung-CLI tests prove is that a probe (a) reaches its
/// timeout branch and (b) still carries the output the CLI printed before the
/// deadline. Both are races against process startup: spawning a
/// just-written temp executable is milliseconds idle, but under concurrent
/// cargo builds the exec can stall long enough that a tight deadline fires
/// with an EMPTY capture, and the assert flakes. A 200 ms window flaked, then
/// 2 s flaked.
///
/// Rather than pick a bigger fixed number and hope, the tests start here and
/// RETRY with a doubled budget whenever the capture came back empty, up to
/// the cap. Idle machines pay the floor; loaded ones escalate. The cap keeps
/// a genuinely broken probe failing instead of looping, and stays well under
/// the fake CLI's own sleep so the timeout branch is still guaranteed.
///
/// The escalation is no longer the only line of defence. Two other things
/// now absorb load before it has to:
///
/// 1. The three tests that use it hold [`subprocess_probe_lock`], so they no
///    longer contend with EACH OTHER — only with load from outside this test
///    binary.
/// 2. [`write_fake_cli`] warms the spawn path before handing back a script,
///    so the first measured probe does not also pay cold `/bin/sh` + dyld +
///    temp-dir page-cache cost.
///
/// Measured, in order:
///
/// - Trio alone, five repeats, against three concurrent
///   `cargo build -p op-editor-ui` jobs (each in its own `CARGO_TARGET_DIR`
///   so they compete for CPU instead of blocking on cargo's build lock):
///   5/5 passed. Four runs took ~8.3 s — the no-escalation cost of two 4 s
///   deadlines back to back — and only the first, coldest run escalated,
///   once, to 8 s. That single cold-run escalation is what motivated the
///   warm-up in (2).
/// - Whole `cargo test -p op-host-services` (731 sibling tests in parallel)
///   during a burst of concurrent agent builds — a dozen simultaneous
///   `cargo check` / `clippy` / `test` invocations across the workspace —
///   **escalated all the way to the old 16 s cap and still captured
///   nothing**. Even the success-path test, whose script exits immediately,
///   could not spawn inside 16 s.
/// - Same trio + three-build load repeated after raising the cap to 32 s:
///   5/5 passed again. Runs 2-5 took ~8.8 s with no escalation; run 1, with
///   cold caches AND all three load builds compiling the dependency tree
///   from scratch, escalated deep and still passed. Under the old 16 s cap
///   that run would have failed.
///
/// The second measurement is why the ceiling below moved 16 s → 32 s: the
/// old cap was demonstrably reachable under real load, and a test that fails
/// at the cap is exactly the flake this is meant to remove. The floor stays
/// at 4 s so an idle machine still pays ~8 s for the pair.
#[cfg(unix)]
const PROBE_BUDGET: Duration = Duration::from_secs(4);
#[cfg(unix)]
const PROBE_BUDGET_CAP: Duration = Duration::from_secs(32);

/// How long the fake CLIs hang after printing. Must outlast
/// `PROBE_BUDGET_CAP` by a wide margin — otherwise a slow escalation could
/// see the script EXIT and the probe would report success where the test
/// demands a timeout. Raised alongside the cap (30 s → 150 s) to keep that
/// margin at ~5x rather than letting it collapse to under 2x.
///
/// Spelled `exec sleep N` by the script builders below: a forked `sleep`
/// inherits the probe's stdout/stderr pipes, so killing the shell on the
/// deadline would leave them open and the probe's reader-thread `join` would
/// block for the whole hang. `exec` makes the sleeping process the very pid
/// the probe kills, so the probe really does return on its deadline (and
/// nothing outlives the test).
#[cfg(unix)]
const FAKE_CLI_HANG_SECS: u32 = 150;

/// Run `probe` under an escalating deadline until its message shows the
/// script's output actually made it into the capture (`captured_marker`), or
/// the budget hits [`PROBE_BUDGET_CAP`]. Returns the message and the budget
/// that produced it, because the timeout wording embeds that budget's
/// seconds.
///
/// Each attempt also asserts the probe returned on its OWN deadline rather
/// than by waiting out the child: the fake CLI hangs for
/// [`FAKE_CLI_HANG_SECS`], so a return inside a small multiple of the budget
/// is proof the deadline — not the process — ended the probe. That keeps
/// "probes are deadline-bounded" under test even though the budget moved.
#[cfg(unix)]
fn probe_until_captured(
    mut probe: impl FnMut(Duration) -> String,
    captured_marker: &str,
) -> (String, Duration) {
    let mut budget = PROBE_BUDGET;
    loop {
        let started = std::time::Instant::now();
        let message = probe(budget);
        let elapsed = started.elapsed();
        // Two bounds, because neither alone is sufficient evidence at both
        // ends of the escalation. `budget * 4` is the tight one at the floor;
        // at the cap it grows past the script's own hang, so it would also
        // accept a probe that was ended by the CHILD exiting rather than by
        // its deadline. The absolute bound rules that out at every budget:
        // returning before the script could possibly have finished is what
        // "deadline-bounded" means here.
        assert!(
            elapsed < budget * 4,
            "probe must return on its own deadline ({budget:?}), not outlast the \
             {FAKE_CLI_HANG_SECS}s script; took {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_secs(u64::from(FAKE_CLI_HANG_SECS)),
            "probe must be ended by its {budget:?} deadline, not by the \
             {FAKE_CLI_HANG_SECS}s script exiting; took {elapsed:?}"
        );
        if message.contains(captured_marker) || budget >= PROBE_BUDGET_CAP {
            return (message, budget);
        }
        budget = (budget * 2).min(PROBE_BUDGET_CAP);
    }
}

/// Writes an executable `/bin/sh` script standing in for a real CLI so
/// `*_models_from_exe` can be pointed at it directly — the discover
/// chain's fixed `&["models"]` args rule out the `/bin/sh -c <script>`
/// trick `cli_probe_support`'s own tests use.
#[cfg(unix)]
fn write_fake_cli(label: &str, script_body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::temp_dir().join(format!(
        "openpencil-cli-model-discovery-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, format!("#!/bin/sh\n{script_body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    warm_up_spawn_path();
    path
}

/// Spawn-and-reap one trivial script from the same temp dir, so the probe
/// that follows is not the first `/bin/sh` exec of the test run.
///
/// The measured evidence for this: with the trio serialized, four of five
/// repeats needed no escalation at all, but the FIRST run consistently
/// escalated once. Nothing about the first run differs except that it pays
/// the cold costs — resolving and mapping `/bin/sh`, the dyld shared-cache
/// warm-up, and the first write+stat round trip in the temp directory. Under
/// load those are exactly the costs that push a fresh exec past a tight
/// deadline and leave the capture empty.
///
/// Warming a THROWAWAY script rather than the caller's is deliberate: the
/// scripts this module hands out hang for [`FAKE_CLI_HANG_SECS`] on purpose,
/// so running one to completion is not an option. Everything cold here is
/// shared between the two anyway.
///
/// Failures are ignored — this is an optimisation, not a precondition. If
/// the warm-up cannot run, the escalation still covers the test.
#[cfg(unix)]
fn warm_up_spawn_path() {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::temp_dir().join(format!(
        "openpencil-cli-model-discovery-warmup-{}",
        std::process::id()
    ));
    if std::fs::write(&path, "#!/bin/sh\nexit 0\n").is_ok()
        && std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).is_ok()
    {
        let _ = std::process::Command::new(&path).status();
    }
    let _ = std::fs::remove_file(&path);
}

#[cfg(unix)]
#[test]
fn antigravity_query_surfaces_auth_prompt_when_it_hangs_mid_oauth() {
    let _serialized = subprocess_probe_lock();
    // Regression coverage for the discover chain's own timeout branch:
    // the retired `command_output` (`Option<Output>`) discarded
    // whatever a hung `agy models` had already printed, so a CLI stuck
    // on first-run OAuth surfaced only a generic "failed or timed out".
    // Routed through the shared `diagnose_timeout` it now names the fix.
    let exe = write_fake_cli(
        "agy-hang",
        &format!(
            "printf 'Authentication required. Please visit the URL to log in:\\n'; \
             exec sleep {FAKE_CLI_HANG_SECS}"
        ),
    );
    // The budget must outlast spawning a just-written temp executable, not
    // just the probe's own polling: a 200ms window let a cold exec reach the
    // deadline before `printf` ran, so the assert flaked on an empty capture.
    // `probe_until_captured` escalates instead of guessing — see its docs.
    let (message, _budget) = probe_until_captured(
        |budget| {
            antigravity_models_from_exe(&exe, budget)
                .unwrap_err()
                .to_string()
        },
        "not authenticated",
    );
    assert_eq!(
        message,
        "Antigravity is not authenticated. Run `agy` once in a terminal."
    );
}

#[cfg(unix)]
#[test]
fn grok_query_falls_back_to_truncated_tail_when_timeout_has_no_auth_marker() {
    let _serialized = subprocess_probe_lock();
    let exe = write_fake_cli(
        "grok-hang",
        &format!(
            "printf 'initializing sandbox...\\nstill working\\n'; exec sleep {FAKE_CLI_HANG_SECS}"
        ),
    );
    let (message, budget) = probe_until_captured(
        |budget| grok_models_from_exe(&exe, budget).unwrap_err().to_string(),
        "still working",
    );
    // The reported duration is the budget the probe actually ran under, so
    // read it back from the attempt that produced this message rather than
    // hardcoding it.
    assert!(
        message.contains(&format!(
            "Grok Build CLI timed out after {}s",
            budget.as_secs()
        )),
        "{message}"
    );
    assert!(message.contains("`grok`"));
    assert!(message.contains("still working"));
}

#[cfg(unix)]
#[test]
fn grok_query_success_path_parses_catalog_unchanged_when_process_exits_in_time() {
    let _serialized = subprocess_probe_lock();
    // Pins the completed-output branch (parse + non-empty-catalog check)
    // exactly as it behaved before the shared bounded-probe migration.
    let exe = write_fake_cli("grok-ok", "printf 'Available models:\\n* grok-4.5\\n'");
    // The script exits immediately, so the probe returns as soon as it does —
    // a generous budget costs nothing here and removes the last way machine
    // load can turn this success-path assertion into a timeout.
    let models = grok_models_from_exe(&exe, PROBE_BUDGET_CAP).unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].value, "grok-4.5");
}
