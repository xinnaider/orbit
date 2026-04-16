use super::{Provider, ProviderSpawnConfig};
use crate::journal::JournalState;
use crate::models::{JournalEntry, JournalEntryType, SlashCommand};
use crate::services::spawn_manager::{find_cli_in_path, SpawnHandle};

/// ACP provider for agents that support the Agent Client Protocol (JSON-RPC over stdio).
///
/// This provider spawns agent CLIs with `--acp` and communicates via JSON-RPC 2.0.
/// It handles initialize → new_session → prompt handshake on spawn, then reads
/// `session/update` notifications and maps them to `JournalEntry` variants.
pub struct AcpProvider {
    provider_id: String,
    display: String,
    cli: String,
    extra_args: Vec<String>,
}

impl AcpProvider {
    pub fn new(id: &str, display: &str, cli: &str, extra_args: &[&str]) -> Self {
        Self {
            provider_id: id.to_string(),
            display: display.to_string(),
            cli: cli.to_string(),
            extra_args: extra_args.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl Provider for AcpProvider {
    fn id(&self) -> &str {
        &self.provider_id
    }

    fn display_name(&self) -> &str {
        &self.display
    }

    fn spawn(&self, config: ProviderSpawnConfig) -> Result<SpawnHandle, String> {
        let cli_path = self
            .find_cli()
            .ok_or_else(|| format!("{} not found — {}", self.cli, self.install_hint()))?;

        let mut cmd = std::process::Command::new(&cli_path);

        for arg in &self.extra_args {
            cmd.arg(arg);
        }

        cmd.current_dir(&config.cwd);
        cmd.env("PATH", crate::services::spawn_manager::extended_path());

        for (k, v) in &config.extra_env {
            cmd.env(k, v);
        }

        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd.spawn().map_err(|e| format!("acp spawn failed: {e}"))?;
        let pid = child.id();

        // Write JSON-RPC handshake to stdin
        let mut stdin = child.stdin.take().ok_or("no stdin")?;
        let cwd_str = config.cwd.to_string_lossy().to_string();
        let prompt = config.prompt.clone();
        let session_id_str = format!("orbit-{}", config.session_id);

        // Spawn a thread to write the handshake so we don't block
        std::thread::spawn(move || {
            let _ = write_jsonrpc_handshake(&mut stdin, &cwd_str, &prompt, &session_id_str);
            // Keep stdin open — the agent reads from it throughout the session.
            // Leaking stdin handle is intentional; it closes when child exits.
            std::mem::forget(stdin);
        });

        let stdout = child.stdout.take().ok_or("no stdout")?;
        let stderr = child.stderr.take().ok_or("no stderr")?;

        Ok(SpawnHandle {
            pid,
            reader: Box::new(stdout),
            stderr: Box::new(stderr),
            child,
            _askpass: None,
        })
    }

    fn process_line(&self, state: &mut JournalState, line: &str) {
        process_acp_line(state, line);
    }

    fn context_window(&self, _model: &str) -> Option<u64> {
        None
    }

    fn slash_commands(&self) -> Vec<SlashCommand> {
        vec![]
    }

    fn supports_effort(&self) -> bool {
        false
    }

    fn supports_ssh(&self) -> bool {
        false
    }

    fn line_processor(&self) -> fn(&mut JournalState, &str) {
        process_acp_line
    }

    fn format_model(&self, raw_model: &str, _provider_id: &str) -> String {
        raw_model.to_string()
    }

    fn cli_name(&self) -> &str {
        &self.cli
    }

    fn find_cli(&self) -> Option<String> {
        find_cli_in_path(&self.cli)
    }

    fn install_hint(&self) -> &str {
        "install the agent CLI and ensure it supports --acp"
    }
}

/// Write the JSON-RPC initialize → new_session → prompt sequence to stdin.
fn write_jsonrpc_handshake(
    stdin: &mut impl std::io::Write,
    cwd: &str,
    prompt: &str,
    session_id: &str,
) -> Result<(), String> {
    // 1. initialize
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1",
            "clientInfo": {
                "name": "orbit",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    });
    write_jsonrpc_message(stdin, &init_req)?;

    // 2. new_session
    let new_session_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "session/new",
        "params": {
            "workingDirectory": cwd
        }
    });
    write_jsonrpc_message(stdin, &new_session_req)?;

    // 3. prompt
    let prompt_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "session/prompt",
        "params": {
            "sessionId": session_id,
            "messages": [
                {
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": prompt
                    }
                }
            ]
        }
    });
    write_jsonrpc_message(stdin, &prompt_req)?;

    stdin.flush().map_err(|e| format!("flush failed: {e}"))?;
    Ok(())
}

fn write_jsonrpc_message(
    writer: &mut impl std::io::Write,
    msg: &serde_json::Value,
) -> Result<(), String> {
    let line = serde_json::to_string(msg).map_err(|e| format!("serialize failed: {e}"))?;
    writer
        .write_all(line.as_bytes())
        .map_err(|e| format!("write failed: {e}"))?;
    writer
        .write_all(b"\n")
        .map_err(|e| format!("write newline failed: {e}"))?;
    Ok(())
}

/// Parse a JSON-RPC line from an ACP agent and update journal state.
///
/// Handles two main message types:
/// - `session/update` notifications with content chunks → Assistant entries
/// - JSON-RPC responses (id-bearing) → System entries for errors
pub fn process_acp_line(state: &mut JournalState, line: &str) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return;
    }

    let msg: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return,
    };

    let session_id = String::new(); // ACP entries get session_id assigned by caller
    let now = chrono::Utc::now().to_rfc3339();

    // JSON-RPC notification (no "id" field)
    if msg.get("id").is_none() {
        if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
            match method {
                "session/update" => {
                    process_session_update(state, &msg, &session_id, &now);
                }
                _ => {
                    // Unknown notification — log as system entry
                    state.entries.push(JournalEntry {
                        session_id: session_id.clone(),
                        timestamp: now,
                        entry_type: JournalEntryType::System,
                        text: Some(format!("ACP notification: {method}")),
                        ..JournalEntry::default()
                    });
                }
            }
        }
        return;
    }

    // JSON-RPC request from agent (has "id" + "method") — e.g. permission requests
    if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
        if method == "requestPermission" || method == "client/requestPermission" {
            let tool = msg
                .pointer("/params/title")
                .or_else(|| msg.pointer("/params/name"))
                .and_then(|t| t.as_str())
                .unwrap_or("unknown tool");
            let desc = msg
                .pointer("/params/description")
                .and_then(|d| d.as_str())
                .unwrap_or("");
            state.pending_approval = Some(format!("Allow {tool}? {desc}"));
            state.status = crate::models::AgentStatus::Input;
            state.attention = crate::models::AttentionState {
                requires_attention: true,
                reason: Some(crate::models::AttentionReason::Permission),
                since: Some(now.clone()),
            };
            state.entries.push(JournalEntry {
                session_id: session_id.clone(),
                timestamp: now,
                entry_type: JournalEntryType::System,
                text: Some(format!("Permission requested: {tool}")),
                ..JournalEntry::default()
            });
        }
        return;
    }

    // JSON-RPC response (has "id" but no "method") — error responses
    if let Some(error) = msg.get("error") {
        let err_msg = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        state.entries.push(JournalEntry {
            session_id,
            timestamp: now,
            entry_type: JournalEntryType::System,
            text: Some(format!("ACP error: {err_msg}")),
            ..JournalEntry::default()
        });
    }
}

fn process_session_update(
    state: &mut JournalState,
    msg: &serde_json::Value,
    session_id: &str,
    now: &str,
) {
    let params = match msg.get("params") {
        Some(p) => p,
        None => return,
    };

    let update = match params.get("update") {
        Some(u) => u,
        None => return,
    };

    let update_type = update
        .get("kind")
        .or_else(|| update.get("type"))
        .and_then(|k| k.as_str())
        .unwrap_or("");

    match update_type {
        "agentMessageChunk" => {
            if let Some(content) = update.get("content") {
                let text = content.get("text").and_then(|t| t.as_str()).unwrap_or("");
                if !text.is_empty() {
                    // Append to last assistant entry if it exists, or create new one
                    if let Some(last) = state.entries.last_mut() {
                        if last.entry_type == JournalEntryType::Assistant {
                            if let Some(ref mut t) = last.text {
                                t.push_str(text);
                            }
                            return;
                        }
                    }
                    state.entries.push(JournalEntry {
                        session_id: session_id.to_string(),
                        timestamp: now.to_string(),
                        entry_type: JournalEntryType::Assistant,
                        text: Some(text.to_string()),
                        ..JournalEntry::default()
                    });
                }
            }
        }
        "toolCallStart" | "toolCall" => {
            let tool_name = update
                .get("name")
                .or_else(|| update.get("toolName"))
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");
            let input = update.get("input").cloned();
            state.entries.push(JournalEntry {
                session_id: session_id.to_string(),
                timestamp: now.to_string(),
                entry_type: JournalEntryType::ToolCall,
                tool: Some(tool_name.to_string()),
                tool_input: input,
                ..JournalEntry::default()
            });
        }
        "toolCallResult" | "toolResult" => {
            let output = update
                .get("output")
                .or_else(|| update.get("result"))
                .and_then(|o| o.as_str())
                .map(|s| s.to_string());
            state.entries.push(JournalEntry {
                session_id: session_id.to_string(),
                timestamp: now.to_string(),
                entry_type: JournalEntryType::ToolResult,
                output,
                ..JournalEntry::default()
            });
        }
        "thinking" => {
            state.entries.push(JournalEntry {
                session_id: session_id.to_string(),
                timestamp: now.to_string(),
                entry_type: JournalEntryType::Thinking,
                thinking: update
                    .get("text")
                    .and_then(|t| t.as_str())
                    .map(String::from),
                ..JournalEntry::default()
            });
        }
        "usage" => {
            if let Some(usage) = update.get("usage") {
                let input_tokens = usage
                    .get("inputTokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let output_tokens = usage
                    .get("outputTokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                state.input_tokens = input_tokens;
                state.output_tokens = output_tokens;
            }
        }
        _ => {}
    }
}
