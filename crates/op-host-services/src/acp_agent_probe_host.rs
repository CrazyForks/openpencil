//! Connect-time ACP agent probe job + outcome — the host-free half
//! carved out of `op-host-desktop`'s `acp_agent_probe_host.rs` (codex
//! Issue 5: the job struct is a `DesktopApp` field, so it lives here
//! for both crates to name it). The `impl DesktopApp` pump stays
//! desktop-side and drives this job through its public API.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use op_editor_core::agent_settings::{
    AcpAgentConfig as CoreAcpAgentConfig, AcpAgentConnectRequest, AcpConnectionType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpAgentProbeOutcome {
    pub connected: bool,
    pub info: Option<String>,
    pub error: Option<String>,
}

impl AcpAgentProbeOutcome {
    /// Public (was private) so the desktop residual's pump tests can
    /// build a success outcome across the crate boundary.
    pub fn connected(info: String) -> Self {
        Self {
            connected: true,
            info: Some(info),
            error: None,
        }
    }

    /// Public (was private) so the desktop residual's pump tests can
    /// build a failure outcome across the crate boundary.
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            connected: false,
            info: None,
            error: Some(error.into()),
        }
    }
}

pub struct AcpAgentConnectJob {
    request: AcpAgentConnectRequest,
    config: CoreAcpAgentConfig,
    rx: Option<Receiver<AcpAgentProbeOutcome>>,
}

impl AcpAgentConnectJob {
    pub fn spawn(request: AcpAgentConnectRequest, agent: CoreAcpAgentConfig) -> Self {
        debug_assert_eq!(request.id, agent.id);
        let config = acp_config_for_probe(&agent);
        let (tx, rx) = mpsc::channel();
        // Detached one-shot: the probe's handshake calls carry op-acp's
        // 30 s `HANDSHAKE_TIMEOUT`, and `AcpConnection::drop` kills the spawned
        // agent + aborts its IO tasks, so this thread always terminates.
        std::thread::spawn(move || {
            let outcome = probe_acp_agent_config(config);
            let _ = tx.send(outcome);
        });
        Self {
            request,
            config: agent,
            rx: Some(rx),
        }
    }

    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    /// The agent id this job is probing. Public accessor for the
    /// desktop-residual pump (private field is unreachable cross-crate).
    pub fn id(&self) -> &str {
        &self.request.id
    }

    pub fn request(&self) -> &AcpAgentConnectRequest {
        &self.request
    }

    /// Exact configuration snapshot that launched this probe.
    pub fn config(&self) -> &CoreAcpAgentConfig {
        &self.config
    }

    /// Test seam: construct a pending job + the sender that feeds it a
    /// fake outcome. Public (not `#[cfg(test)]`) so the desktop residual's
    /// `impl DesktopApp` tests can build one across the crate boundary.
    #[doc(hidden)]
    pub fn pending_for_test(
        request: AcpAgentConnectRequest,
        config: CoreAcpAgentConfig,
    ) -> (Self, mpsc::Sender<AcpAgentProbeOutcome>) {
        debug_assert_eq!(request.id, config.id);
        let (tx, rx) = mpsc::channel();
        (
            Self {
                request,
                config,
                rx: Some(rx),
            },
            tx,
        )
    }

    pub fn poll(&mut self) -> Option<AcpAgentProbeOutcome> {
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(outcome) => {
                self.rx = None;
                Some(outcome)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                Some(AcpAgentProbeOutcome::failed(
                    "ACP probe worker disconnected",
                ))
            }
        }
    }
}

pub fn probe_acp_agent_config(config: op_acp::AcpAgentConfig) -> AcpAgentProbeOutcome {
    // Reached from the probe worker thread today and from tokio workers via
    // the web-canvas server, so bridge through the runtime-aware helper.
    crate::chat_runtime::block_on_anywhere(async move {
        match op_acp::connect_acp_agent(&config).await {
            Ok(conn) => {
                let info = format_acp_agent_info(conn.agent_info(), &config.display_name);
                AcpAgentProbeOutcome::connected(info)
            }
            Err(err) => AcpAgentProbeOutcome::failed(err.to_string()),
        }
    })
}

pub fn acp_config_for_probe(agent: &CoreAcpAgentConfig) -> op_acp::AcpAgentConfig {
    op_acp::AcpAgentConfig {
        id: agent.id.clone(),
        display_name: agent.display_name.clone(),
        connection_type: match agent.connection_type {
            AcpConnectionType::Local => op_acp::ConnectionType::Local,
            AcpConnectionType::Remote => op_acp::ConnectionType::Remote,
        },
        command: match agent.connection_type {
            AcpConnectionType::Local => Some(agent.command.clone()),
            AcpConnectionType::Remote => None,
        },
        args: agent.args.clone(),
        env: agent.env.clone(),
        url: agent.url.clone(),
        enabled: agent.enabled,
    }
}

/// Whether each quick-add preset's command resolves to a real file on
/// this machine, keyed by preset id.
///
/// Advisory only. It resolves against the same merged login-shell PATH
/// the spawn path uses (so a GUI launch sees the user's nvm/homebrew
/// shims), but a `false` here never blocks adding the preset — PATH is a
/// snapshot, and the ACP handshake is the authority on whether the agent
/// actually runs.
pub fn probe_acp_preset_availability() -> std::collections::BTreeMap<String, bool> {
    op_editor_core::acp_agent_presets::ACP_AGENT_PRESETS
        .iter()
        .map(|preset| {
            let resolved = crate::chat_spawn::find_binary(preset.command);
            // `find_binary` echoes the bare name back when it finds
            // nothing, so "resolved to an existing file" is the test.
            let found = std::path::Path::new(&resolved).is_file();
            (preset.id.to_string(), found)
        })
        .collect()
}

pub fn format_acp_agent_info(info: &op_acp::AcpAgentInfo, fallback: &str) -> String {
    let name = if info.name.trim().is_empty() {
        fallback.trim()
    } else {
        info.name.trim()
    };
    match info
        .version
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        Some(version) => format!("{name} {version}"),
        None => name.to_string(),
    }
}
