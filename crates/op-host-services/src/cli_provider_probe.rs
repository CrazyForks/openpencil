//! Connect probes for Antigravity and Grok Build. Split from
//! `provider_probe.rs` to preserve that module's 800-line cap.
//!
//! The bounded-subprocess run/kill/diagnose plumbing (`BoundedProbe`,
//! `bounded_cli_output`, `diagnose_timeout`, `tail_snippet`) lives in
//! `cli_probe_support` and is shared with `cli_model_discovery`'s discover
//! chain, which hits the exact same "CLI hangs mid first-run OAuth" failure
//! mode.

use std::path::Path;
use std::time::Duration;

use op_ai::agent_settings_state::AgentProvider;
use op_ai::chat_models::ModelEntry;
use op_ai::chat_provider::CliName;

use crate::chat_subprocess_safety;
use crate::cli_probe_support::{bounded_cli_output, diagnose_timeout, BoundedProbe};
use crate::model_discovery::resolve_cli;
use crate::provider_probe::ProbeOutcome;

const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

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

    // `BoundedProbe` / `bounded_cli_output` / `diagnose_timeout` /
    // `tail_snippet` are shared with `cli_model_discovery` and covered by
    // `cli_probe_support`'s own test module — no need to duplicate that
    // coverage here.
}
