//! `--serve-web` invocation parsing (`ServeWebOptions`), the managed-mode
//! handshake line + token, and the startup-document loader. Split out of
//! `web_canvas_server.rs` to keep the spine under the 800-line cap.

use super::*;

/// Fully parsed `--serve-web` invocation, covering both the legacy
/// positional syntax and the new `--managed` flag syntax (see
/// [`parse_serve_web_args`]).
pub struct ServeWebOptions {
    pub port: u16,
    pub path: Option<PathBuf>,
    pub host: String,
    /// `--managed`: the daemon was spawned by a supervising process (e.g.
    /// the VS Code extension) that expects the handshake-JSON + stdin-EOF
    /// lifecycle contract instead of the legacy fire-and-forget daemon.
    pub managed: bool,
    /// `--allow-origin <origin>` (repeatable), managed mode only. Enforced by
    /// `serve_one` via `cors_origin_for` to gate which `Origin` headers are
    /// echoed back in `Access-Control-Allow-Origin` responses.
    pub allow_origins: Vec<String>,
}

/// Parse the argv tail after `--serve-web` itself. Pure, so the flag shape is
/// unit-testable without spawning the binary. Supports two syntaxes:
///
/// - Legacy positional (unchanged): `<port> [doc] [--host <addr>]`.
/// - Managed flag form: `--managed --port <n|0> [--file <path>]
///   [--host <addr>] [--allow-origin <origin>]...` — used by supervising
///   processes that want the handshake-JSON / stdin-EOF lifecycle contract
///   (see [`run_web_canvas`]).
///
/// The host defaults to loopback; `--host 0.0.0.0` is the LAN/Docker opt-in
/// (no TLS — deploy behind a proxy for anything beyond a trusted network).
pub fn parse_serve_web_args<I: Iterator<Item = String>>(
    args: I,
) -> std::result::Result<ServeWebOptions, String> {
    // Public entry point consumed by `cli_modes.rs` and the host binaries,
    // which are outside this conversion's scope — keep the `String` contract
    // and adapt the typed error here rather than rippling outward.
    parse_serve_web_args_typed(args).map_err(|e| e.to_string())
}

pub(super) fn parse_serve_web_args_typed<I: Iterator<Item = String>>(
    mut args: I,
) -> Result<ServeWebOptions> {
    let Some(first) = args.next() else {
        return Err(WebCanvasError::Config("missing <port> arg".into()));
    };
    if first.starts_with("--") {
        return parse_serve_web_args_managed(first, args);
    }
    let Ok(port) = first.parse::<u16>() else {
        return Err(WebCanvasError::Config(format!(
            "<port> must be a u16, got {first:?}"
        )));
    };
    let mut path: Option<PathBuf> = None;
    let mut host = "127.0.0.1".to_string();
    while let Some(arg) = args.next() {
        if arg == "--host" {
            host = args.next().ok_or_else(|| {
                WebCanvasError::Config("--host needs a value (e.g. 0.0.0.0)".into())
            })?;
        } else if let Some(value) = arg.strip_prefix("--host=") {
            host = value.to_string();
        } else if path.is_none() {
            // The document path is optional — without it the daemon starts
            // from the same starter document the web shell paints locally.
            path = Some(PathBuf::from(arg));
        } else {
            return Err(WebCanvasError::Config(format!("unexpected arg {arg:?}")));
        }
    }
    if host.is_empty() {
        return Err(WebCanvasError::Config("--host must not be empty".into()));
    }
    Ok(ServeWebOptions {
        port,
        path,
        host,
        managed: false,
        allow_origins: Vec::new(),
    })
}

/// Parse the flag-style `--managed --port <n|0> [--file <path>]
/// [--host <addr>] [--allow-origin <origin>]...` form. `first_flag` is the
/// already-consumed first token (always `--managed` in practice, but any
/// leading `--`-prefixed token routes here so an unknown flag reports a
/// useful error instead of misparsing as a port).
pub(super) fn parse_serve_web_args_managed<I: Iterator<Item = String>>(
    first_flag: String,
    mut args: I,
) -> Result<ServeWebOptions> {
    let mut managed = false;
    let mut port: Option<u16> = None;
    let mut path: Option<PathBuf> = None;
    let mut host = "127.0.0.1".to_string();
    let mut allow_origins: Vec<String> = Vec::new();
    let mut next_flag = Some(first_flag);
    while let Some(arg) = next_flag.take().or_else(|| args.next()) {
        match arg.as_str() {
            "--managed" => managed = true,
            "--port" => {
                let value = args
                    .next()
                    .ok_or_else(|| WebCanvasError::Config("--port needs a value".into()))?;
                port = Some(value.parse::<u16>().map_err(|_| {
                    WebCanvasError::Config(format!("--port must be a u16, got {value:?}"))
                })?);
            }
            "--file" => {
                path = Some(PathBuf::from(args.next().ok_or_else(|| {
                    WebCanvasError::Config("--file needs a value".into())
                })?));
            }
            "--host" => {
                host = args.next().ok_or_else(|| {
                    WebCanvasError::Config("--host needs a value (e.g. 0.0.0.0)".into())
                })?;
            }
            "--allow-origin" => {
                allow_origins.push(args.next().ok_or_else(|| {
                    WebCanvasError::Config("--allow-origin needs a value".into())
                })?);
            }
            other => return Err(WebCanvasError::Config(format!("unexpected arg {other:?}"))),
        }
    }
    let Some(port) = port else {
        return Err(WebCanvasError::Config("missing --port <n>".into()));
    };
    if host.is_empty() {
        return Err(WebCanvasError::Config("--host must not be empty".into()));
    }
    Ok(ServeWebOptions {
        port,
        path,
        host,
        managed,
        allow_origins,
    })
}

/// Build the single-line handshake JSON printed to stdout in managed mode
/// once the listener is bound: `{"ok":true,"port":<n>,"token":"<hex32>",
/// "version":"<crate version>"}`. The supervising process reads exactly one
/// line from the child's stdout to learn the actual bound port (relevant
/// when `--port 0` requested an OS-assigned port) and the per-instance auth
/// token.
pub(crate) fn handshake_json(port: u16, token: &str) -> String {
    format!(
        r#"{{"ok":true,"port":{port},"token":"{token}","version":"{}"}}"#,
        env!("CARGO_PKG_VERSION")
    )
}

/// Generate a per-instance token for managed mode. Not a cryptographic PRNG —
/// `RandomState`'s per-process keying plus a nanosecond timestamp and the pid
/// give a token that's unguessable across separate daemon invocations. The real
/// access control is enforced by `serve_one` via `RequestAuth` gate on every
/// request and `cors_origin_for` on privileged endpoints like `/mcp`.
pub(super) fn random_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let s = RandomState::new();
    let mut h1 = s.build_hasher();
    h1.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    );
    let mut h2 = s.build_hasher();
    h2.write_u64(std::process::id() as u64);
    format!("{:016x}{:016x}", h1.finish(), h2.finish())
}

pub(super) fn startup_editor_from_base_for_web_canvas(
    base: EditorState,
    path: Option<PathBuf>,
) -> Result<EditorState> {
    match path {
        Some(p) => {
            let mut next = crate::mcp_serve::load_editor_state(&p)?;
            preserve_web_canvas_preferences(&base, &mut next);
            set_file_name_display(&mut next, &p);
            next.editor_ui.touch_recent_file(
                p.to_string_lossy().into_owned(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            );
            Ok(next)
        }
        None => Ok(base),
    }
}

pub(super) fn startup_editor_for_web_canvas_with_loader<Checked>(
    path: Option<PathBuf>,
    _policy: WebCredentialPersistence,
    checked_load: Checked,
) -> Result<EditorState>
where
    // `settings_io::load_checked` is outside this conversion's scope and
    // still reports `String`; keep its shape and adapt at the call.
    Checked: FnOnce(&mut EditorState) -> std::result::Result<(), String>,
{
    let mut base = EditorState::starter();
    checked_load(&mut base).map_err(WebCanvasError::Config)?;
    startup_editor_from_base_for_web_canvas(base, path)
}

pub(super) fn startup_editor_for_web_canvas_with_policy(
    path: Option<PathBuf>,
    policy: WebCredentialPersistence,
) -> Result<EditorState> {
    startup_editor_for_web_canvas_with_loader(path, policy, crate::settings_io::load_checked)
}

/// Public entry point (host binaries) — keeps the `String` contract and
/// adapts the typed error at the boundary.
pub fn startup_editor_for_web_canvas(
    path: Option<PathBuf>,
) -> std::result::Result<EditorState, String> {
    startup_editor_for_web_canvas_with_policy(path, crate::web_credential_policy::from_env())
        .map_err(|e| e.to_string())
}
