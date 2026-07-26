use super::*;

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
    let empty = require_antigravity_models("", "").unwrap_err();
    assert!(empty.contains("no model catalog"));

    let auth = require_antigravity_models("", "Please sign in to continue").unwrap_err();
    assert!(auth.contains("requires authentication"));

    let unknown = require_grok_models("Available models:\nautomatic", "").unwrap_err();
    assert!(unknown.contains("unrecognized model catalog"));

    let auth = require_grok_models("", "Authentication required").unwrap_err();
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
#[cfg(unix)]
const PROBE_BUDGET: Duration = Duration::from_secs(4);
#[cfg(unix)]
const PROBE_BUDGET_CAP: Duration = Duration::from_secs(16);

/// How long the fake CLIs hang after printing. Must outlast
/// `PROBE_BUDGET_CAP` by a wide margin — otherwise a slow escalation could
/// see the script EXIT and the probe would report success where the test
/// demands a timeout.
///
/// Spelled `exec sleep N` by the script builders below: a forked `sleep`
/// inherits the probe's stdout/stderr pipes, so killing the shell on the
/// deadline would leave them open and the probe's reader-thread `join` would
/// block for the whole hang. `exec` makes the sleeping process the very pid
/// the probe kills, so the probe really does return on its deadline (and
/// nothing outlives the test).
#[cfg(unix)]
const FAKE_CLI_HANG_SECS: u32 = 30;

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
        assert!(
            elapsed < budget * 4,
            "probe must return on its own deadline ({budget:?}), not outlast the \
             {FAKE_CLI_HANG_SECS}s script; took {elapsed:?}"
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
    path
}

#[cfg(unix)]
#[test]
fn antigravity_query_surfaces_auth_prompt_when_it_hangs_mid_oauth() {
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
        |budget| antigravity_models_from_exe(&exe, budget).unwrap_err(),
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
    let exe = write_fake_cli(
        "grok-hang",
        &format!(
            "printf 'initializing sandbox...\\nstill working\\n'; exec sleep {FAKE_CLI_HANG_SECS}"
        ),
    );
    let (message, budget) = probe_until_captured(
        |budget| grok_models_from_exe(&exe, budget).unwrap_err(),
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
