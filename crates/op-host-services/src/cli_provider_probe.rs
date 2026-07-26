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
use op_i18n::Locale;

use crate::chat_subprocess_safety;
use crate::cli_probe_support::{bounded_cli_output, diagnose_timeout, BoundedProbe};
use crate::model_discovery::resolve_cli;
use crate::provider_probe::ProbeOutcome;

const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Translate one `providerProbe.*` key with named placeholders. Mirrors the
/// private `tw` helper in `provider_probe.rs` — these probes produce the same
/// Settings-card copy, so they resolve against the same catalog.
fn tw(locale: Locale, key: &'static str, vars: &[(&str, &str)]) -> String {
    op_i18n::translate_with(locale, key, vars)
}

/// Entry point kept argument-free so `provider_probe::connect_provider` calls
/// it unchanged; the locale is resolved the same way the probe worker resolves
/// it (settings file, then OS locale).
///
/// TODO(follow-up, needs `provider_probe.rs`): have `connect_provider` forward
/// the `locale` it already holds instead of re-resolving here.
pub fn connect_antigravity() -> ProbeOutcome {
    connect_antigravity_localized(crate::provider_probe::resolved_ui_locale())
}

/// See [`connect_antigravity`]. Split out so tests can pin the locale.
pub fn connect_antigravity_localized(locale: Locale) -> ProbeOutcome {
    let Some(exe) = resolve_cli("agy") else {
        return not_installed(
            tw(
                locale,
                "providerProbe.cliNotFound",
                &[("name", "Antigravity")],
            ),
            AgentProvider::Antigravity,
        );
    };
    let version = match cli_version(
        locale,
        CliName::Antigravity,
        &exe,
        &["--version"],
        "Antigravity",
        "`agy`",
    ) {
        Ok(version) => version,
        Err(error) => return failed(error),
    };
    let models = match query_models(
        locale,
        CliName::Antigravity,
        &exe,
        "Antigravity",
        "`agy`",
        crate::cli_model_discovery::parse_antigravity_models,
    ) {
        Ok(models) => models,
        Err(error) => return failed(error),
    };
    ProbeOutcome {
        connected: true,
        models,
        connection_info: Some(tw(
            locale,
            "providerProbe.connectedViaCli",
            &[("name", "Antigravity")],
        )),
        hint_path: Some("~/.gemini/antigravity-cli/settings.json".to_string()),
        version: Some(version),
        ..ProbeOutcome::default()
    }
}

/// See [`connect_antigravity`] for why this resolves its own locale.
pub fn connect_grok_build() -> ProbeOutcome {
    connect_grok_build_localized(crate::provider_probe::resolved_ui_locale())
}

/// See [`connect_grok_build`]. Split out so tests can pin the locale.
pub fn connect_grok_build_localized(locale: Locale) -> ProbeOutcome {
    let Some(exe) = resolve_cli("grok") else {
        return not_installed(
            tw(
                locale,
                "providerProbe.cliNotFound",
                &[("name", "Grok Build")],
            ),
            AgentProvider::GrokBuild,
        );
    };
    let version = match cli_version(
        locale,
        CliName::GrokBuild,
        &exe,
        &["version"],
        "Grok Build",
        "`grok`",
    ) {
        Ok(version) => version,
        Err(error) => return failed(error),
    };
    let models = match query_models(
        locale,
        CliName::GrokBuild,
        &exe,
        "Grok Build",
        "`grok`",
        crate::cli_model_discovery::parse_grok_models,
    ) {
        Ok(models) => models,
        Err(error) => return failed(error),
    };
    ProbeOutcome {
        connected: true,
        models,
        connection_info: Some(tw(
            locale,
            "providerProbe.connectedViaCli",
            &[("name", "Grok Build")],
        )),
        hint_path: Some("~/.grok/config.toml".to_string()),
        version: Some(version),
        ..ProbeOutcome::default()
    }
}

fn not_installed(error: String, provider: AgentProvider) -> ProbeOutcome {
    ProbeOutcome {
        error: Some(error),
        not_installed: true,
        install_command: Some(crate::provider_probe::install_command(provider).to_string()),
        ..ProbeOutcome::default()
    }
}

fn failed(error: String) -> ProbeOutcome {
    ProbeOutcome {
        error: Some(error),
        ..ProbeOutcome::default()
    }
}

fn cli_version(
    locale: Locale,
    cli: CliName,
    exe: &Path,
    args: &[&str],
    provider: &str,
    login_command: &str,
) -> Result<String, String> {
    match bounded_cli_output(cli, exe, args, PROBE_TIMEOUT) {
        BoundedProbe::Completed(output) => {
            if !output.status.success() {
                return Err(tw(
                    locale,
                    "providerProbe.cliExitedWithError",
                    &[("name", provider)],
                ));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let version = if stdout.trim().is_empty() {
                stderr.trim()
            } else {
                stdout.trim()
            };
            if version.is_empty() {
                Err(tw(
                    locale,
                    "providerProbe.cliNoVersionOutput",
                    &[("name", provider)],
                ))
            } else {
                Ok(version.to_string())
            }
        }
        // `diagnose_timeout` echoes the CLI's own (English) auth prompt back to
        // the user, so it stays untranslated on purpose — see its doc comment.
        BoundedProbe::TimedOut { stdout, stderr } => Err(diagnose_timeout(
            cli,
            provider,
            login_command,
            PROBE_TIMEOUT,
            &stdout,
            &stderr,
        )),
        BoundedProbe::Failed => Err(tw(
            locale,
            "providerProbe.cliNotResponding",
            &[("name", provider)],
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn query_models(
    locale: Locale,
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
        BoundedProbe::Failed => {
            return Err(tw(
                locale,
                "providerProbe.modelQueryFailed",
                &[("name", provider)],
            ))
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        if let Some(message) = chat_subprocess_safety::friendly_stderr_error(Some(cli), &stderr) {
            return Err(message);
        }
        return Err(if stderr.trim().is_empty() {
            tw(
                locale,
                "providerProbe.modelQueryFailedRunLogin",
                &[("name", provider), ("command", login_command)],
            )
        } else {
            // The CLI's own diagnostic — surfaced verbatim, not translated.
            stderr.trim().to_string()
        });
    }

    let models = parse(&stdout);
    if !models.is_empty() {
        return Ok(models);
    }
    Err(catalog_error(
        locale,
        provider,
        login_command,
        &stdout,
        &stderr,
    ))
}

fn catalog_error(
    locale: Locale,
    provider: &str,
    login_command: &str,
    stdout: &str,
    stderr: &str,
) -> String {
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
        tw(
            locale,
            "providerProbe.modelQueryNeedsAuth",
            &[("name", provider), ("command", login_command)],
        )
    } else if stdout.trim().is_empty() {
        tw(locale, "providerProbe.noModelList", &[("name", provider)])
    } else {
        tw(
            locale,
            "providerProbe.unrecognizedModelCatalog",
            &[("name", provider)],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai::agent_settings_state::AgentProvider;

    #[test]
    fn not_installed_outcome_carries_provider_guidance() {
        let outcome = not_installed("missing".to_string(), AgentProvider::Antigravity);
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
        assert!(
            catalog_error(Locale::EnUs, "Grok Build", "`grok`", "", "login required")
                .contains("requires authentication")
        );
        assert_eq!(
            catalog_error(Locale::EnUs, "Grok Build", "`grok`", "unexpected prose", ""),
            "Grok Build returned an unrecognized model catalog"
        );
        // Empty stdout is a different failure shape: no catalog at all.
        assert_eq!(
            catalog_error(Locale::EnUs, "Antigravity", "`agy`", "", ""),
            "No models found. Antigravity did not return a model list."
        );
    }

    #[test]
    fn probe_strings_resolve_through_the_locale_catalog() {
        // The regression this replaced: hardcoded English in a Settings card
        // that the rest of the modal renders in the chrome language.
        assert_eq!(
            tw(
                Locale::ZhCn,
                "providerProbe.connectedViaCli",
                &[("name", "Antigravity")]
            ),
            "已通过 Antigravity CLI 连接"
        );
        assert_eq!(
            tw(
                Locale::Ja,
                "providerProbe.connectedViaCli",
                &[("name", "Grok Build")]
            ),
            "Grok Build CLI 経由で接続しました"
        );
        // Every new key must resolve directly (not via the English fallback)
        // in every shipped locale.
        for key in [
            "providerProbe.connectedViaCli",
            "providerProbe.cliExitedWithError",
            "providerProbe.cliNoVersionOutput",
            "providerProbe.modelQueryFailed",
            "providerProbe.modelQueryFailedRunLogin",
            "providerProbe.modelQueryNeedsAuth",
            "providerProbe.unrecognizedModelCatalog",
        ] {
            for locale in Locale::ALL {
                let rendered = tw(
                    locale,
                    key,
                    &[("name", "Grok Build"), ("command", "`grok`")],
                );
                assert!(
                    rendered.contains("Grok Build") && !rendered.contains("{{"),
                    "locale {locale:?} left `{key}` unresolved: {rendered}"
                );
            }
        }
    }

    // `BoundedProbe` / `bounded_cli_output` / `diagnose_timeout` /
    // `tail_snippet` are shared with `cli_model_discovery` and covered by
    // `cli_probe_support`'s own test module — no need to duplicate that
    // coverage here.
}
