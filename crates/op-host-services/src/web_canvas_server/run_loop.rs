//! Daemon lifecycle: bind the listener, spawn per-connection threads, honor
//! the managed-mode stdin lease, and the request-auth / CORS gates the
//! connection loop consults. Split out of `web_canvas_server.rs` to keep the
//! spine under the 800-line cap.

use super::*;

/// Run the web-canvas daemon per `options` (host/port default `127.0.0.1`),
/// backed by the document at `options.path` (or the starter document when
/// `None`). Serves the static host page + bundle, the whole-document REST
/// sync + health routes, and falls through to the JSON-RPC `/mcp` tool
/// dispatch (applied against the in-memory document). Blocks until a
/// token-authed shutdown request (or, in managed mode, stdin EOF).
///
/// Managed mode (`options.managed`) layers on the parent-death lease
/// contract used by a supervising process (e.g. the VS Code extension):
/// once the listener is bound, a single-line handshake JSON
/// (`{"ok":true,"port":..,"token":..,"version":..}`) is printed to stdout so
/// the supervisor learns the actual port (relevant for `--port 0`) and a
/// per-instance token; a background thread then reads stdin to EOF/error and
/// raises the same `shutdown` flag the token-authed `openpencil/shutdown`
/// path uses, waking the accept loop by connecting back to the bound
/// address. Non-managed mode is untouched: no token, no handshake output, no
/// stdin thread.
pub fn run_web_canvas(options: ServeWebOptions) -> std::result::Result<(), String> {
    // Public entry point (`cli_modes.rs`) — `String` contract preserved.
    run_web_canvas_typed(options).map_err(|e| e.to_string())
}

pub(super) fn run_web_canvas_typed(options: ServeWebOptions) -> Result<()> {
    let ServeWebOptions {
        port,
        path,
        host,
        managed,
        allow_origins,
    } = options;
    let current_path = path.clone();
    let credential_persistence = crate::web_credential_policy::from_env();
    let mut editor = startup_editor_for_web_canvas_with_policy(path, credential_persistence)?;
    enforce_credential_persistence_policy(
        &mut editor,
        credential_persistence,
        crate::settings_io::save_checked,
    )?;
    // Device-login proxy: init the shared auth runtime and restore the
    // session the desktop GUI may already have persisted. Never on a
    // non-loopback bind outside managed mode — the proxy session belongs
    // to the daemon owner, not to whoever can reach the port.
    let loopback_bind = matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1");
    crate::web_auth::init(&mut editor, managed || loopback_bind);
    let listener = TcpListener::bind((host.as_str(), port))
        .map_err(|e| WebCanvasError::Config(format!("bind {host}:{port}: {e}")))?;
    let local_addr = listener
        .local_addr()
        .map_err(|e| WebCanvasError::Config(e.to_string()))?;
    let bound = local_addr.port();
    eprintln!("openpencil-desktop --serve-web: listening on {host}:{bound}");
    match crate::web_static::resolve_bundle_dir() {
        Some(dir) => eprintln!(
            "openpencil-desktop --serve-web: serving web bundle from {}",
            dir.display()
        ),
        None => eprintln!(
            "openpencil-desktop --serve-web: no web bundle found — `/` serves build \
             instructions (tools/check-wasm-bundle.sh, or set OPENPENCIL_WEB_BUNDLE_DIR)"
        ),
    }
    // Shared across connection threads: the document authority (one writer at a
    // time via the Mutex) + the SSE broadcast hub. Thread-per-connection so a
    // long-lived SSE stream (or a slow client) never blocks other clients.
    let state = Arc::new(Mutex::new(WebCanvasState::new_with_path_and_policy(
        editor,
        bound,
        current_path,
        credential_persistence,
    )));
    let hub = Arc::new(SseHub::default());
    let conn_count = Arc::new(AtomicUsize::new(0));
    // Raised by a connection thread that accepted a token-authed
    // `openpencil/shutdown`; the accept loop checks it per iteration. The
    // raiser also pokes the listener with a throwaway connection so a blocked
    // `accept` wakes up and observes the flag.
    let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
    // Managed mode only: per-instance token + handshake + parent-death
    // lease (stdin-EOF watcher). Non-managed mode never touches this branch
    // — it keeps the existing `OPENPENCIL_MCP_TOKEN` shutdown contract as
    // the only lifecycle signal, byte-for-byte as before.
    let managed_token = managed.then(random_token);
    if let Some(token) = &managed_token {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{}", handshake_json(bound, token));
        let _ = out.flush();
        drop(out);
        let shutdown_stdin = Arc::clone(&shutdown);
        // Detached on purpose — there is NO portable way to cancel a thread
        // parked in a blocking `Stdin::read`. A channel or flag can only be
        // observed between reads, and putting fd 0 into non-blocking mode
        // would need platform `fcntl`/`SetNamedPipeHandleState` calls (a new
        // dependency or unsafe per-OS code) AND would change what "EOF" means
        // for the parent-death lease, which is this thread's whole purpose.
        // So the exit path is: (a) the parent closes stdin — the loop ends and
        // raises `shutdown` itself, or (b) some other path raised `shutdown`
        // first, in which case the checks below make this thread a no-op and
        // the process exit reaps it. The flag check per iteration is what
        // makes (b) prompt rather than "whenever the parent next writes".
        let _ = std::thread::Builder::new()
            .name("op-serve-web-stdin".into())
            .spawn(move || {
                let mut sink = [0u8; 64];
                let mut stdin = std::io::stdin();
                while !shutdown_stdin.load(Ordering::Acquire)
                    && matches!(stdin.read(&mut sink), Ok(n) if n > 0)
                {}
                // Only raise + wake when nobody else already shut the daemon
                // down; a redundant wake connect against an already-closed
                // listener is harmless but pointlessly noisy.
                if !shutdown_stdin.swap(true, Ordering::AcqRel) {
                    // Wake the (possibly blocked) accept loop — reconnect to
                    // the bound address exactly (works for IPv6 / custom
                    // --host, unlike the loopback-only wake used by the
                    // token-authed shutdown path below).
                    let _ = std::net::TcpStream::connect(local_addr);
                }
            });
    }
    // Stash the managed token + allow-origins on the shared state. `serve_one`
    // reads them via `RequestAuth` gate (token) and `cors_origin_for` (CORS
    // allowlist) to enforce auth and CORS policies on all incoming requests.
    if managed_token.is_some() || !allow_origins.is_empty() {
        let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
        guard.managed_token = managed_token;
        guard.allow_origins = allow_origins;
    }
    for stream in listener.incoming() {
        if shutdown.load(Ordering::Acquire) {
            break;
        }
        let mut s = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("openpencil-desktop --serve-web: accept: {e}");
                continue;
            }
        };
        if conn_count.load(Ordering::Acquire) >= MAX_CONNS {
            let _ = s.set_write_timeout(Some(IO_TIMEOUT));
            let _ = crate::mcp_serve::write_mcp_http_response(
                &mut s,
                "503 Service Unavailable",
                r#"{"ok":false,"error":"server busy"}"#,
            );
            continue;
        }
        conn_count.fetch_add(1, Ordering::AcqRel);
        let state = Arc::clone(&state);
        let hub = Arc::clone(&hub);
        let conns = Arc::clone(&conn_count);
        let shutdown_flag = Arc::clone(&shutdown);
        let spawned = thread::Builder::new()
            .name("op-serve-web-conn".into())
            .spawn(move || {
                let _conn_guard = ConnGuard(conns);
                let _ = s.set_read_timeout(Some(IO_TIMEOUT));
                let _ = s.set_write_timeout(Some(IO_TIMEOUT));
                match serve_one(&mut s, &state, &hub) {
                    Ok(true) => {
                        shutdown_flag.store(true, Ordering::Release);
                        // Wake the (possibly blocked) accept loop. Loopback
                        // reaches the listener for both the 127.0.0.1 and the
                        // 0.0.0.0 binds.
                        let _ = std::net::TcpStream::connect(("127.0.0.1", bound));
                    }
                    Ok(false) => {}
                    Err(e) => eprintln!("openpencil-desktop --serve-web: {e}"),
                }
            });
        if spawned.is_err() {
            conn_count.fetch_sub(1, Ordering::AcqRel);
        }
    }
    eprintln!("openpencil-desktop --serve-web: shutdown requested; exiting");
    Ok(())
}

pub(super) fn enforce_credential_persistence_policy<F>(
    editor: &mut EditorState,
    policy: WebCredentialPersistence,
    save: F,
) -> Result<()>
where
    // `settings_io::save_checked`'s `String` shape is preserved (unowned);
    // only the outcome is retyped.
    F: FnOnce(&EditorState) -> std::result::Result<(), String>,
{
    if !policy.server_persistence()
        && crate::web_credentials::remove_browser_owned_credentials(editor)
    {
        save(editor).map_err(|_| {
            WebCanvasError::Config(
                "failed to remove browser-owned credentials while server persistence is disabled"
                    .into(),
            )
        })?;
    }
    Ok(())
}

/// Managed-mode request gate (see `ServeWebOptions::managed` /
/// `WebCanvasState::managed_token`). Non-managed daemons construct
/// `RequestAuth { managed: false, .. }`, which `allows` always satisfies —
/// the legacy fire-and-forget daemon stays byte-for-byte tokenless.
pub(crate) struct RequestAuth {
    pub managed: bool,
    pub token: String,
}

impl RequestAuth {
    /// Whether `method path` may proceed without (or with a mismatched)
    /// `presented` token. Mirrors `web_static.rs`'s static route table
    /// (`web_static.rs:188`): everything the static layer serves stays
    /// tokenless — the page cannot know the token before the postMessage
    /// bootstrap hands it over — plus `OPTIONS` preflight. Every other
    /// request (the `POST /` JSON-RPC alias, `/mcp`, `/api/*` including the
    /// SSE `/api/mcp/events` and AI stream endpoints) requires the exact
    /// per-instance token.
    pub(crate) fn allows(&self, method: &str, path: &str, presented: Option<&str>) -> bool {
        if !self.managed {
            return true;
        }
        let static_get = method.eq_ignore_ascii_case("GET")
            && (path == "/"
                || path == "/index.html"
                || path.starts_with("/pkg/")
                || path.starts_with("/smoke/")
                || path.starts_with("/canvaskit/")
                || path.starts_with("/assets/"));
        let exempt = method.eq_ignore_ascii_case("OPTIONS") || static_get;
        exempt || presented == Some(self.token.as_str())
    }
}

/// Managed-mode CORS allowlist check: echoes `origin` back only when it
/// exactly matches an entry in `allow`, otherwise omits the header
/// (`None`). Unmanaged mode never calls this — it keeps the permissive
/// `*` inline at each call site instead.
pub(crate) fn cors_origin_for(allow: &[String], origin: Option<&str>) -> Option<String> {
    origin
        .filter(|o| allow.iter().any(|a| a == o))
        .map(str::to_string)
}
