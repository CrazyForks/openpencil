//! End-to-end coverage for a saved local ACP agent.
//!
//! The fake agent is this test executable itself, launched through the same
//! `tokio::process::Command` stdio path as a user-configured local agent. That
//! keeps the fixture cross-platform while proving the process boundary rather
//! than stopping at an in-memory duplex stream or an injected probe outcome.

use super::*;
use op_editor_core::{AcpAgentConnectPhase, ChatRole};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::thread;
use std::time::{Duration, Instant};

const FIXTURE_ENV: &str = "OPENPENCIL_LOCAL_ACP_E2E_FIXTURE";
const FIXTURE_TEST: &str = "acp_agent_probe_host::local_e2e_tests::fake_local_acp_agent_process";
const FIXTURE_AGENT_NAME: &str = "OpenPencil Local ACP Fixture";
const FIXTURE_AGENT_VERSION: &str = "1.0";
const FIXTURE_SESSION_ID: &str = "fixture-session";
const FIXTURE_PROMPT: &str = "LOCAL_ACP_E2E_7C1: reply with the fixture greeting.";
const FIXTURE_REPLY: &str = "Hello from the real local ACP subprocess.";
const FIXTURE_MCP_PORT: u16 = 4_123;

#[test]
fn local_acp_save_connect_initialize_picker_prompt_disconnect_e2e() {
    let executable = std::env::current_exe().expect("resolve current test executable");
    let mut app = DesktopApp::new(None);

    // DesktopApp restores machine settings even in tests. Reset only the
    // in-memory agent/chat catalog so this process E2E is deterministic and
    // never reads or writes a user's configured agents.
    {
        let state = app.host.editor_state_mut();
        state.editor_ui.agent_settings = Default::default();
        state.chat.discovered_models.clear();
        state.chat.available_models.clear();
        state.chat.messages.clear();
        state.chat.pending_send = None;
    }

    // Save through the same draft seam used by Settings → Add ACP Agent.
    let agent_id = {
        let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
        settings.begin_acp_agent_draft();
        let draft = settings
            .acp_agent_draft
            .as_mut()
            .expect("draft should be created");
        draft.display_name = FIXTURE_AGENT_NAME.into();
        draft.command = executable.to_string_lossy().into_owned();
        draft.args = vec![
            FIXTURE_TEST.into(),
            "--exact".into(),
            "--ignored".into(),
            "--nocapture".into(),
            "--test-threads=1".into(),
        ];
        draft.env.insert(FIXTURE_ENV.into(), "enabled".into());
        settings
            .save_acp_agent_draft()
            .expect("ready local ACP draft should save")
    };
    let agent_index = app
        .host
        .editor_state()
        .editor_ui
        .agent_settings
        .acp_agents
        .iter()
        .position(|agent| agent.id == agent_id)
        .expect("saved ACP agent");

    app.host.editor_state_mut().rebuild_chat_models();
    assert!(
        app.host
            .editor_state()
            .chat
            .available_models
            .iter()
            .all(|model| model.value != format!("acp:{agent_id}")),
        "saving a configuration must not make it selectable before a real probe"
    );

    // Explicit Connect must spawn the command and complete ACP initialize
    // before the current configuration is marked verified.
    assert_eq!(
        app.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .begin_acp_agent_connect(agent_index)
            .as_deref(),
        Some(agent_id.as_str())
    );
    let connect_deadline = Instant::now() + Duration::from_secs(15);
    loop {
        app.drain_acp_agent_connect();
        if app
            .host
            .editor_state()
            .editor_ui
            .agent_settings
            .acp_agent_verified_connected(&agent_id)
        {
            break;
        }
        assert!(
            Instant::now() < connect_deadline,
            "local ACP connect timed out: {:?}",
            app.host
                .editor_state()
                .editor_ui
                .agent_settings
                .acp_agent_connection_for(&agent_id)
        );
        thread::sleep(Duration::from_millis(10));
    }

    let connection = app
        .host
        .editor_state()
        .editor_ui
        .agent_settings
        .acp_agent_connection_for(&agent_id);
    assert_eq!(connection.phase, AcpAgentConnectPhase::Connected);
    assert_eq!(
        connection.info.as_deref(),
        Some(format!("{FIXTURE_AGENT_NAME} {FIXTURE_AGENT_VERSION}").as_str())
    );

    // Rebuilding after the verified initialize adds the ACP entry; selecting
    // it through the open picker exercises the actual picker/model seam.
    let model_index = app
        .host
        .editor_state()
        .chat
        .available_models
        .iter()
        .position(|model| model.value == format!("acp:{agent_id}"))
        .expect("verified ACP agent should be present in the picker");
    {
        let state = app.host.editor_state_mut();
        assert!(state.editor_ui.toggle_chat_model_picker());
        assert!(state.editor_ui.chat_model_picker.open);
        state.select_chat_model(model_index);
        assert!(!state.editor_ui.chat_model_picker.open);
        assert_eq!(
            state
                .chat
                .selected_model_entry()
                .map(|model| model.value.as_str()),
            Some(format!("acp:{agent_id}").as_str())
        );
        state.editor_ui.agent_settings.mcp_server.running = true;
        state.editor_ui.agent_settings.mcp_server.port = FIXTURE_MCP_PORT;
        state.chat.set_input_text(FIXTURE_PROMPT);
        assert!(state.chat.begin_send());
    }

    // ACP chat reconnects to the saved command, then drives session/new and
    // session/prompt. The child refuses malformed MCP/session/prompt payloads,
    // so receiving this reply proves the full wire sequence.
    assert!(crate::chat_session::launch_if_pending(
        &mut app.host,
        &mut app.current_chat,
        &mut app.current_design,
    ));
    assert!(
        app.current_chat.is_some(),
        "ACP selection should launch a real chat session"
    );
    let prompt_deadline = Instant::now() + Duration::from_secs(15);
    while app.current_chat.is_some() {
        crate::chat_session::pump(
            &mut app.host,
            &mut app.current_chat,
            None,
            None,
            (1_200.0, 800.0),
        );
        assert!(
            Instant::now() < prompt_deadline,
            "local ACP prompt timed out; transcript: {:?}",
            app.host.editor_state().chat.messages
        );
        if app.current_chat.is_some() {
            thread::sleep(Duration::from_millis(10));
        }
    }
    let assistant = app
        .host
        .editor_state()
        .chat
        .messages
        .iter()
        .rev()
        .find(|message| message.role == ChatRole::Assistant)
        .expect("assistant response");
    assert_eq!(assistant.content, FIXTURE_REPLY);
    assert!(!assistant.streaming);

    // Explicit disconnect invalidates the verified runtime marker and removes
    // the agent from the picker on the same rebuild path used by the UI.
    {
        let state = app.host.editor_state_mut();
        assert_eq!(
            state
                .editor_ui
                .agent_settings
                .disconnect_acp_agent(agent_index)
                .as_deref(),
            Some(agent_id.as_str())
        );
        state.rebuild_chat_models();
    }
    let state = app.host.editor_state();
    let settings = &state.editor_ui.agent_settings;
    assert!(!settings.acp_agent_verified_connected(&agent_id));
    assert_eq!(
        settings.acp_agent_connection_for(&agent_id),
        Default::default()
    );
    assert!(state
        .chat
        .available_models
        .iter()
        .all(|model| model.value != format!("acp:{agent_id}")));
}

/// Ignored during normal test runs; the parent E2E launches this exact test
/// through `current_exe` and talks ACP ndJSON over its stdin/stdout.
#[test]
#[ignore = "stdio fixture launched by the local ACP process E2E"]
fn fake_local_acp_agent_process() {
    if std::env::var_os(FIXTURE_ENV).is_none() {
        return;
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    // libtest prints `test <name> ... ` without a newline before invoking an
    // uncaptured test. Terminate that prefix so op-acp's tolerant ndJSON
    // reader can skip it before the first real response.
    stdout
        .write_all(b"\n")
        .expect("terminate libtest stdout prefix");
    stdout.flush().expect("flush fixture prelude");

    let mut initialized = false;
    let mut session_open = false;
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        let Ok(frame) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let id = frame.get("id").cloned().unwrap_or(Value::Null);
        let method = frame
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let params = frame.get("params").cloned().unwrap_or(Value::Null);

        match method {
            "initialize" => {
                let valid = params.get("protocolVersion").and_then(Value::as_u64) == Some(1)
                    && params.pointer("/clientInfo/name").and_then(Value::as_str)
                        == Some("openpencil");
                if !valid {
                    write_rpc_error(&mut stdout, id, "invalid initialize payload");
                    continue;
                }
                initialized = true;
                write_frame(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "protocolVersion": 1,
                            "agentInfo": {
                                "name": FIXTURE_AGENT_NAME,
                                "version": FIXTURE_AGENT_VERSION
                            }
                        }
                    }),
                );
            }
            "session/new" => {
                let server = params
                    .get("mcpServers")
                    .and_then(Value::as_array)
                    .and_then(|servers| servers.first());
                let valid = initialized
                    && params
                        .get("cwd")
                        .and_then(Value::as_str)
                        .is_some_and(|cwd| !cwd.is_empty())
                    && server
                        .and_then(|server| server.get("name"))
                        .and_then(Value::as_str)
                        == Some("openpencil")
                    && server
                        .and_then(|server| server.get("type"))
                        .and_then(Value::as_str)
                        == Some("http")
                    && server
                        .and_then(|server| server.get("url"))
                        .and_then(Value::as_str)
                        == Some("http://127.0.0.1:4123/mcp")
                    && server
                        .and_then(|server| server.get("headers"))
                        .and_then(Value::as_array)
                        .is_some_and(Vec::is_empty)
                    && params
                        .pointer("/_meta/systemPrompt")
                        .and_then(Value::as_str)
                        .is_some_and(|prompt| prompt.contains("mcp__openpencil__"));
                if !valid {
                    write_rpc_error(&mut stdout, id, "invalid session/new payload");
                    continue;
                }
                session_open = true;
                write_frame(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "sessionId": FIXTURE_SESSION_ID }
                    }),
                );
            }
            "session/prompt" => {
                let prompt_text = params
                    .get("prompt")
                    .and_then(Value::as_array)
                    .and_then(|blocks| blocks.first())
                    .and_then(|block| block.get("text"))
                    .and_then(Value::as_str);
                let valid = initialized
                    && session_open
                    && params.get("sessionId").and_then(Value::as_str) == Some(FIXTURE_SESSION_ID)
                    && prompt_text.is_some_and(|prompt| prompt.contains(FIXTURE_PROMPT));
                if !valid {
                    write_rpc_error(&mut stdout, id, "invalid session/prompt payload");
                    continue;
                }
                write_frame(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "method": "session/update",
                        "params": {
                            "sessionId": FIXTURE_SESSION_ID,
                            "update": {
                                "sessionUpdate": "agent_message_chunk",
                                "content": { "type": "text", "text": FIXTURE_REPLY }
                            }
                        }
                    }),
                );
                write_frame(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "stopReason": "end_turn" }
                    }),
                );
                break;
            }
            _ => write_rpc_error(&mut stdout, id, "unsupported method"),
        }
    }
}

fn write_rpc_error(stdout: &mut dyn Write, id: Value, message: &str) {
    write_frame(
        stdout,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32_000, "message": message }
        }),
    );
}

fn write_frame(stdout: &mut dyn Write, frame: Value) {
    serde_json::to_writer(&mut *stdout, &frame).expect("serialize ACP fixture frame");
    stdout.write_all(b"\n").expect("write ACP fixture newline");
    stdout.flush().expect("flush ACP fixture frame");
}
