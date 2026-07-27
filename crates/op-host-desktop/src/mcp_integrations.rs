//! Terminal-side MCP client configuration for the desktop settings panel.
//!
//! The live server is owned by the GUI process. CLI integrations point
//! at that process over streamable HTTP so terminal agents can reach the
//! same canvas state the user is editing.

use std::fs;
use std::path::{Path, PathBuf};

use op_editor_core::agent_settings::McpCli;
use serde_json::{Map, Value};

use crate::mcp_config_error::McpConfigError;
use crate::mcp_config_io::{
    atomic_write, grok_config_has_openpencil, update_grok_config, FileSnapshot,
};

const SERVER_NAME: &str = "openpencil";
const ANTIGRAVITY_MCP_PERMISSION: &str = "mcp(openpencil/*)";

pub(crate) fn set_cli_enabled(
    cli: McpCli,
    enabled: bool,
    port: u16,
) -> Result<PathBuf, McpConfigError> {
    let home = dirs::home_dir().ok_or(McpConfigError::HomeDirUnavailable)?;
    if cli == McpCli::Antigravity {
        return set_antigravity_enabled_at_home(enabled, port, &home);
    }
    let path = config_path(cli, &home, true);
    set_cli_enabled_at_path(cli, enabled, port, path)
}

pub(crate) fn detect_enabled_clis() -> [bool; 7] {
    let Some(home) = dirs::home_dir() else {
        return [false; 7];
    };
    detect_enabled_clis_for_home(&home, true)
}

/// Like [`set_cli_enabled`] but against an explicit home dir and without
/// reading `CODEX_HOME` (`use_env = false`). Used by tests to redirect CLI
/// config writes to a temp dir WITHOUT mutating process-global env.
pub(crate) fn set_cli_enabled_at_home(
    cli: McpCli,
    enabled: bool,
    port: u16,
    home: &Path,
) -> Result<PathBuf, McpConfigError> {
    if cli == McpCli::Antigravity {
        return set_antigravity_enabled_at_home(enabled, port, home);
    }
    let path = config_path(cli, home, false);
    set_cli_enabled_at_path(cli, enabled, port, path)
}

/// Like [`detect_enabled_clis`] but against an explicit home dir (no env).
pub(crate) fn detect_enabled_clis_at_home(home: &Path) -> [bool; 7] {
    detect_enabled_clis_for_home(home, false)
}

fn detect_enabled_clis_for_home(home: &Path, use_env: bool) -> [bool; 7] {
    let mut flags = [false; 7];
    for (idx, cli) in McpCli::ALL.iter().copied().enumerate() {
        flags[idx] = if cli == McpCli::Antigravity {
            antigravity_config_has_openpencil(&config_path(cli, home, use_env))
                && antigravity_permission_is_present(&antigravity_permissions_path(home))
        } else {
            let path = config_path(cli, home, use_env);
            cli_config_has_openpencil(cli, &path)
        };
    }
    flags
}

fn set_cli_enabled_at_path(
    cli: McpCli,
    enabled: bool,
    port: u16,
    path: PathBuf,
) -> Result<PathBuf, McpConfigError> {
    match cli {
        McpCli::Codex => update_codex_config(&path, enabled, port)?,
        McpCli::GrokBuild => update_grok_config(&path, enabled, &endpoint(port))?,
        McpCli::Antigravity => return Err(McpConfigError::AntigravityNeedsHome),
        McpCli::ClaudeCode | McpCli::OpenCode | McpCli::Kiro | McpCli::GithubCopilot => {
            update_json_config(&path, enabled, port)?
        }
    }
    Ok(path)
}

fn cli_config_has_openpencil(cli: McpCli, path: &Path) -> bool {
    match cli {
        McpCli::Codex => fs::read_to_string(path)
            .map(|text| codex_config_has_openpencil(&text))
            .unwrap_or(false),
        McpCli::GrokBuild => fs::read_to_string(path)
            .map(|text| grok_config_has_openpencil(&text))
            .unwrap_or(false),
        McpCli::Antigravity => antigravity_config_has_openpencil(path),
        McpCli::ClaudeCode | McpCli::OpenCode | McpCli::Kiro | McpCli::GithubCopilot => {
            json_config_has_openpencil(path)
        }
    }
}

fn config_path(cli: McpCli, home: &Path, use_env: bool) -> PathBuf {
    match cli {
        McpCli::ClaudeCode => home.join(".claude.json"),
        McpCli::Codex => {
            if use_env {
                std::env::var_os("CODEX_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home.join(".codex"))
                    .join("config.toml")
            } else {
                home.join(".codex").join("config.toml")
            }
        }
        McpCli::OpenCode => home.join(".opencode").join("config.json"),
        McpCli::Kiro => home.join(".kiro").join("settings.json"),
        McpCli::GithubCopilot => home.join(".config").join("github-copilot").join("mcp.json"),
        McpCli::Antigravity => home.join(".gemini").join("config").join("mcp_config.json"),
        McpCli::GrokBuild => {
            if use_env {
                std::env::var_os("GROK_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home.join(".grok"))
                    .join("config.toml")
            } else {
                home.join(".grok").join("config.toml")
            }
        }
    }
}

fn set_antigravity_enabled_at_home(
    enabled: bool,
    port: u16,
    home: &Path,
) -> Result<PathBuf, McpConfigError> {
    let config = config_path(McpCli::Antigravity, home, false);
    let permissions = antigravity_permissions_path(home);
    let config_snapshot = FileSnapshot::capture(&config)?;
    let permissions_snapshot = FileSnapshot::capture(&permissions)?;

    update_antigravity_config(&config, enabled, port)?;
    if let Err(error) = update_antigravity_permissions(&permissions, enabled) {
        let mut rollback_errors = Vec::new();
        if let Err(rollback_error) = config_snapshot.restore(&config) {
            rollback_errors.push(rollback_error);
        }
        if let Err(rollback_error) = permissions_snapshot.restore(&permissions) {
            rollback_errors.push(rollback_error);
        }
        return if rollback_errors.is_empty() {
            Err(error)
        } else {
            Err(McpConfigError::Rollback {
                cause: Box::new(error),
                failures: rollback_errors,
            })
        };
    }
    Ok(config)
}

fn antigravity_permissions_path(home: &Path) -> PathBuf {
    home.join(".gemini")
        .join("antigravity-cli")
        .join("settings.json")
}

fn update_antigravity_permissions(path: &Path, enabled: bool) -> Result<(), McpConfigError> {
    if !enabled && !path.exists() {
        return Ok(());
    }
    let mut root = read_json_object(path)?;
    if enabled {
        let permissions = root
            .entry("permissions")
            .or_insert_with(|| Value::Object(Map::new()));
        let permissions = permissions
            .as_object_mut()
            .ok_or(McpConfigError::PermissionsNotAnObject)?;
        let allow = permissions
            .entry("allow")
            .or_insert_with(|| Value::Array(Vec::new()));
        let allow = allow
            .as_array_mut()
            .ok_or(McpConfigError::PermissionsAllowNotAnArray)?;
        allow.retain(|rule| rule.as_str() != Some(ANTIGRAVITY_MCP_PERMISSION));
        allow.push(Value::String(ANTIGRAVITY_MCP_PERMISSION.into()));
        if let Some(deny) = permissions.get_mut("deny").and_then(Value::as_array_mut) {
            deny.retain(|rule| rule.as_str() != Some(ANTIGRAVITY_MCP_PERMISSION));
        }
    } else if let Some(allow) = root
        .get_mut("permissions")
        .and_then(Value::as_object_mut)
        .and_then(|permissions| permissions.get_mut("allow"))
        .and_then(Value::as_array_mut)
    {
        allow.retain(|rule| rule.as_str() != Some(ANTIGRAVITY_MCP_PERMISSION));
    }
    write_json_object(path, &root)
}

fn antigravity_permission_is_present(path: &Path) -> bool {
    read_json_object(path)
        .ok()
        .and_then(|root| {
            let permissions = root.get("permissions").and_then(Value::as_object)?;
            let denied = permissions
                .get("deny")
                .and_then(Value::as_array)
                .is_some_and(|rules| {
                    rules
                        .iter()
                        .any(|rule| rule.as_str() == Some(ANTIGRAVITY_MCP_PERMISSION))
                });
            permissions
                .get("allow")
                .and_then(Value::as_array)
                .map(|allow| {
                    !denied
                        && allow
                            .iter()
                            .any(|rule| rule.as_str() == Some(ANTIGRAVITY_MCP_PERMISSION))
                })
        })
        .unwrap_or(false)
}

fn json_config_has_openpencil(path: &Path) -> bool {
    read_json_object(path)
        .ok()
        .and_then(|root| {
            root.get("mcpServers")
                .and_then(Value::as_object)
                .map(|servers| servers.contains_key(SERVER_NAME))
        })
        .unwrap_or(false)
}

fn antigravity_config_has_openpencil(path: &Path) -> bool {
    let Some(server) = read_json_object(path).ok().and_then(|root| {
        root.get("mcpServers")
            .and_then(Value::as_object)
            .and_then(|servers| servers.get(SERVER_NAME))
            .and_then(Value::as_object)
            .cloned()
    }) else {
        return false;
    };
    if server.get("disabled").and_then(Value::as_bool) == Some(true) {
        return false;
    }
    let Some(server_url) = server.get("serverUrl").and_then(Value::as_str) else {
        return false;
    };
    reqwest::Url::parse(server_url)
        .map(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
        .unwrap_or(false)
}

fn update_json_config(path: &Path, enabled: bool, port: u16) -> Result<(), McpConfigError> {
    let mut root = read_json_object(path)?;
    if enabled {
        let servers = root
            .entry("mcpServers")
            .or_insert_with(|| Value::Object(Map::new()));
        if !servers.is_object() {
            *servers = Value::Object(Map::new());
        }
        let Some(servers) = servers.as_object_mut() else {
            return Err(McpConfigError::McpServersNotAnObject);
        };
        servers.insert(
            SERVER_NAME.into(),
            serde_json::json!({
                "type": "http",
                "url": endpoint(port),
            }),
        );
    } else if let Some(servers) = root.get_mut("mcpServers").and_then(Value::as_object_mut) {
        servers.remove(SERVER_NAME);
        if servers.is_empty() {
            root.remove("mcpServers");
        }
    }
    write_json_object(path, &root)
}

fn update_antigravity_config(path: &Path, enabled: bool, port: u16) -> Result<(), McpConfigError> {
    if !enabled && !path.exists() {
        return Ok(());
    }
    let mut root = read_json_object(path)?;
    if enabled {
        let servers = root
            .entry("mcpServers")
            .or_insert_with(|| Value::Object(Map::new()));
        if !servers.is_object() {
            *servers = Value::Object(Map::new());
        }
        let servers = servers
            .as_object_mut()
            .ok_or(McpConfigError::McpServersNotAnObject)?;
        let mut server = Map::new();
        server.insert("serverUrl".into(), Value::String(endpoint(port)));
        servers.insert(SERVER_NAME.into(), Value::Object(server));
    } else if let Some(servers) = root.get_mut("mcpServers").and_then(Value::as_object_mut) {
        servers.remove(SERVER_NAME);
        if servers.is_empty() {
            root.remove("mcpServers");
        }
    }
    write_json_object(path, &root)
}

fn read_json_object(path: &Path) -> Result<Map<String, Value>, McpConfigError> {
    // `std::io::Error` / `serde_json::Error` come from crates this pass does
    // not own, so their messages ride along as text.
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(e) => {
            return Err(McpConfigError::Read {
                path: path.to_path_buf(),
                message: e.to_string(),
            })
        }
    };
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    let value: Value = serde_json::from_str(&text).map_err(|e| McpConfigError::Parse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| McpConfigError::NotAJsonObject {
            path: path.to_path_buf(),
        })
}

fn write_json_object(path: &Path, root: &Map<String, Value>) -> Result<(), McpConfigError> {
    let text = serde_json::to_string_pretty(root).map_err(|e| McpConfigError::Serialize {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    atomic_write(path, format!("{text}\n").as_bytes())
}

fn update_codex_config(path: &Path, enabled: bool, port: u16) -> Result<(), McpConfigError> {
    let existing = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            return Err(McpConfigError::Read {
                path: path.to_path_buf(),
                message: e.to_string(),
            })
        }
    };
    let mut text = remove_codex_server_block(&existing);
    if enabled {
        let prefix = text.trim_end();
        text = String::from(prefix);
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str("[mcp_servers.openpencil]\n");
        text.push_str(&format!(
            "url = \"{}\"\n",
            toml_basic_string_escape(&endpoint(port))
        ));
    }
    atomic_write(path, text.as_bytes())
}

fn codex_config_has_openpencil(input: &str) -> bool {
    input.lines().map(str::trim).any(is_codex_openpencil_table)
}

fn remove_codex_server_block(input: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in input.split_inclusive('\n') {
        let trimmed = line.trim();
        if is_codex_openpencil_table(trimmed) {
            skipping = true;
            continue;
        }
        if skipping && trimmed.starts_with('[') {
            skipping = false;
        }
        if !skipping {
            out.push_str(line);
        }
    }
    out
}

fn is_codex_openpencil_table(line: &str) -> bool {
    matches!(
        line,
        "[mcp_servers.openpencil]"
            | "[mcp_servers.\"openpencil\"]"
            | "[\"mcp_servers\".\"openpencil\"]"
    )
}

fn endpoint(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

fn toml_basic_string_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
#[path = "mcp_integrations_tests.rs"]
mod tests;
