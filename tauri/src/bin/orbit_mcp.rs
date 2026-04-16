//! Orbit MCP Server — Multi-agent orchestration via Model Context Protocol.
//!
//! Exposes tools that allow any MCP-capable agent (Claude Code, etc.) to create,
//! monitor, message, and cancel other agents through Orbit.
//!
//! Transport: stdio JSON-RPC 2.0 (one message per line).
//!
//! Usage in Claude Code's MCP config:
//! ```json
//! { "mcpServers": { "orbit": { "command": "orbit-mcp" } } }
//! ```

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

// ── Agent state ─────────────────────────────────────────────────────────────

struct AgentProcess {
    child: Child,
    provider: String,
    model: String,
    cwd: String,
    output: Arc<Mutex<Vec<String>>>,
    finished: Arc<Mutex<bool>>,
}

struct McpState {
    agents: HashMap<u64, AgentProcess>,
    next_id: u64,
}

impl McpState {
    fn new() -> Self {
        Self {
            agents: HashMap::new(),
            next_id: 1,
        }
    }
}

// ── CLI resolution ──────────────────────────────────────────────────────────

fn extended_path() -> String {
    let current = std::env::var("PATH").unwrap_or_default();

    #[cfg(windows)]
    {
        let extra: Vec<String> = dirs::home_dir()
            .map(|h| {
                vec![
                    h.join(".local").join("bin").to_string_lossy().into_owned(),
                    h.join("AppData")
                        .join("Roaming")
                        .join("npm")
                        .to_string_lossy()
                        .into_owned(),
                    h.join("AppData")
                        .join("Local")
                        .join("pnpm")
                        .to_string_lossy()
                        .into_owned(),
                ]
            })
            .unwrap_or_default();
        format!("{};{}", extra.join(";"), current)
    }

    #[cfg(not(windows))]
    {
        let extra: Vec<String> = dirs::home_dir()
            .map(|h| {
                vec![
                    h.join(".local").join("bin").to_string_lossy().into_owned(),
                    h.join(".nvm")
                        .join("current")
                        .join("bin")
                        .to_string_lossy()
                        .into_owned(),
                    "/usr/local/bin".to_string(),
                ]
            })
            .unwrap_or_default();
        format!("{}:{}", extra.join(":"), current)
    }
}

fn find_cli(name: &str) -> Option<String> {
    let aug = extended_path();

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let out = Command::new("where")
            .arg(name)
            .env("PATH", &aug)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        stdout.lines().next().map(|l| l.trim().to_string())
    }

    #[cfg(not(windows))]
    {
        let out = Command::new("which")
            .arg(name)
            .env("PATH", &aug)
            .output()
            .ok()?;
        if out.status.success() {
            Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            None
        }
    }
}

fn provider_to_cli(provider: &str) -> &str {
    match provider {
        "claude-code" | "claude" => "claude",
        "codex" => "codex",
        "opencode" => "opencode",
        "gemini-cli" | "gemini" => "gemini",
        "copilot-cli" | "copilot" => "copilot",
        other => other,
    }
}

fn build_spawn_args(provider: &str, model: &str, prompt: &str) -> Vec<String> {
    match provider {
        "claude-code" | "claude" => {
            let mut args = vec![
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--verbose".to_string(),
                "--dangerously-skip-permissions".to_string(),
            ];
            if !model.is_empty() && model != "auto" {
                args.extend(["--model".to_string(), model.to_string()]);
            }
            args.extend(["-p".to_string(), prompt.to_string()]);
            args
        }
        "codex" => {
            let mut args = vec![
                "exec".to_string(),
                "--json".to_string(),
                "--dangerously-bypass-approvals-and-sandbox".to_string(),
            ];
            if !model.is_empty() {
                args.extend(["-m".to_string(), model.to_string()]);
            }
            args.push(prompt.to_string());
            args
        }
        "opencode" => {
            let mut args = vec!["--json".to_string()];
            if !model.is_empty() {
                args.extend(["--model".to_string(), model.to_string()]);
            }
            args.extend(["-p".to_string(), prompt.to_string()]);
            args
        }
        // ACP providers
        _ => vec!["--acp".to_string()],
    }
}

// ── Tool implementations ────────────────────────────────────────────────────

fn tool_create_agent(state: &mut McpState, params: &Value) -> Result<Value, String> {
    let provider = params
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("claude-code");
    let model = params
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let cwd = params
        .get("cwd")
        .and_then(|v| v.as_str())
        .ok_or("missing 'cwd' parameter")?;
    let prompt = params
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or("missing 'prompt' parameter")?;
    let wait = params
        .get("wait")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let timeout_secs = params
        .get("timeoutSecs")
        .and_then(|v| v.as_u64())
        .unwrap_or(300);

    let cli_name = provider_to_cli(provider);
    let cli_path = find_cli(cli_name)
        .ok_or_else(|| format!("{cli_name} not found in PATH. Install it first."))?;

    let args = build_spawn_args(provider, model, prompt);

    let mut cmd = Command::new(&cli_path);
    cmd.args(&args);
    cmd.current_dir(cwd);
    cmd.env("PATH", extended_path());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;
    let pid = child.id();
    let session_id = state.next_id;
    state.next_id += 1;

    let output = Arc::new(Mutex::new(Vec::new()));
    let finished = Arc::new(Mutex::new(false));

    // Background thread to read stdout
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let output_clone = Arc::clone(&output);
    let finished_clone = Arc::clone(&finished);
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let trimmed = l.trim().to_string();
                    if !trimmed.is_empty() {
                        output_clone.lock().unwrap().push(trimmed);
                    }
                }
                Err(_) => break,
            }
        }
        *finished_clone.lock().unwrap() = true;
    });

    state.agents.insert(
        session_id,
        AgentProcess {
            child,
            provider: provider.to_string(),
            model: model.to_string(),
            cwd: cwd.to_string(),
            output: Arc::clone(&output),
            finished: Arc::clone(&finished),
        },
    );

    if wait {
        // Block until finished or timeout
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
        loop {
            if *finished.lock().unwrap() {
                break;
            }
            if std::time::Instant::now() > deadline {
                return Ok(json!({
                    "sessionId": session_id,
                    "pid": pid,
                    "status": "timeout",
                    "output": extract_assistant_text(&output.lock().unwrap()),
                }));
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }

        let text = extract_assistant_text(&output.lock().unwrap());
        // Clean up finished agent
        if let Some(mut agent) = state.agents.remove(&session_id) {
            let _ = agent.child.wait();
        }
        Ok(json!({
            "sessionId": session_id,
            "pid": pid,
            "status": "completed",
            "output": text,
        }))
    } else {
        Ok(json!({
            "sessionId": session_id,
            "pid": pid,
            "status": "running",
        }))
    }
}

fn tool_get_status(state: &McpState, params: &Value) -> Result<Value, String> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'sessionId' parameter")?;

    match state.agents.get(&session_id) {
        Some(agent) => {
            let done = *agent.finished.lock().unwrap();
            let lines = agent.output.lock().unwrap();
            let text = extract_assistant_text(&lines);
            Ok(json!({
                "sessionId": session_id,
                "provider": agent.provider,
                "model": agent.model,
                "status": if done { "completed" } else { "running" },
                "output": text,
                "lineCount": lines.len(),
            }))
        }
        None => Ok(json!({
            "sessionId": session_id,
            "status": "not_found",
        })),
    }
}

fn tool_send_message(state: &mut McpState, params: &Value) -> Result<Value, String> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'sessionId' parameter")?;
    let message = params
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or("missing 'message' parameter")?;

    // Remove old agent and get its metadata
    let old_agent = state
        .agents
        .remove(&session_id)
        .ok_or("agent not found")?;

    // Extract claude_session_id from output (for --resume)
    let lines = old_agent.output.lock().unwrap();
    let resume_id = extract_session_id(&lines);
    drop(lines);

    // Kill old process if still running
    let mut child = old_agent.child;
    let _ = child.kill();
    let _ = child.wait();

    // Re-spawn with --resume
    let cli_name = provider_to_cli(&old_agent.provider);
    let cli_path =
        find_cli(cli_name).ok_or_else(|| format!("{cli_name} not found in PATH"))?;

    let mut args = vec![];
    match old_agent.provider.as_str() {
        "claude-code" | "claude" => {
            args.extend([
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--verbose".to_string(),
                "--dangerously-skip-permissions".to_string(),
            ]);
            if let Some(ref rid) = resume_id {
                args.extend(["--resume".to_string(), rid.clone()]);
            }
            args.extend(["-p".to_string(), message.to_string()]);
        }
        _ => {
            return Err(format!(
                "send_message not yet supported for provider '{}'",
                old_agent.provider
            ));
        }
    }

    let mut cmd = Command::new(&cli_path);
    cmd.args(&args);
    cmd.current_dir(&old_agent.cwd);
    cmd.env("PATH", extended_path());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut new_child = cmd.spawn().map_err(|e| format!("respawn failed: {e}"))?;
    let pid = new_child.id();

    let output = Arc::new(Mutex::new(Vec::new()));
    let finished = Arc::new(Mutex::new(false));

    let stdout = new_child.stdout.take().ok_or("no stdout")?;
    let output_clone = Arc::clone(&output);
    let finished_clone = Arc::clone(&finished);
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let trimmed = l.trim().to_string();
                    if !trimmed.is_empty() {
                        output_clone.lock().unwrap().push(trimmed);
                    }
                }
                Err(_) => break,
            }
        }
        *finished_clone.lock().unwrap() = true;
    });

    state.agents.insert(
        session_id,
        AgentProcess {
            child: new_child,
            provider: old_agent.provider,
            model: old_agent.model,
            cwd: old_agent.cwd,
            output,
            finished,
        },
    );

    Ok(json!({
        "sessionId": session_id,
        "pid": pid,
        "status": "running",
    }))
}

fn tool_cancel_agent(state: &mut McpState, params: &Value) -> Result<Value, String> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'sessionId' parameter")?;

    if let Some(mut agent) = state.agents.remove(&session_id) {
        let _ = agent.child.kill();
        let _ = agent.child.wait();
        Ok(json!({ "sessionId": session_id, "status": "cancelled" }))
    } else {
        Ok(json!({ "sessionId": session_id, "status": "not_found" }))
    }
}

// ── Output parsing helpers ──────────────────────────────────────────────────

/// Extract assistant text from Claude's stream-json output lines.
fn extract_assistant_text(lines: &[String]) -> String {
    let mut texts = Vec::new();
    for line in lines {
        if let Ok(val) = serde_json::from_str::<Value>(line) {
            // Claude stream-json: {"type":"assistant","message":{"content":[{"text":"..."}]}}
            if val.get("type").and_then(|t| t.as_str()) == Some("assistant") {
                if let Some(content) = val.pointer("/message/content") {
                    if let Some(arr) = content.as_array() {
                        for block in arr {
                            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                texts.push(text.to_string());
                            }
                        }
                    }
                }
            }
            // Codex: {"type":"item.completed","item":{"type":"agent_message","text":"..."}}
            if val.pointer("/item/type").and_then(|t| t.as_str()) == Some("agent_message") {
                if let Some(text) = val.pointer("/item/text").and_then(|t| t.as_str()) {
                    texts.push(text.to_string());
                }
            }
        }
    }
    texts.join("\n")
}

/// Extract the Claude session ID from stream-json init message.
fn extract_session_id(lines: &[String]) -> Option<String> {
    for line in lines {
        if let Ok(val) = serde_json::from_str::<Value>(line) {
            // {"type":"system","subtype":"init","session_id":"..."}
            if val.get("type").and_then(|t| t.as_str()) == Some("system") {
                if let Some(sid) = val.get("session_id").and_then(|s| s.as_str()) {
                    return Some(sid.to_string());
                }
            }
        }
    }
    None
}

// ── MCP Protocol ────────────────────────────────────────────────────────────

fn handle_initialize(id: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "orbit-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    })
}

fn handle_tools_list(id: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": [
                {
                    "name": "orbit_create_agent",
                    "description": "Create and spawn a new AI coding agent. By default waits for completion and returns the output. Set wait=false to run in background.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "provider": {
                                "type": "string",
                                "description": "Agent provider: claude-code, codex, opencode, gemini-cli, copilot-cli",
                                "default": "claude-code"
                            },
                            "model": {
                                "type": "string",
                                "description": "Model ID (e.g. claude-sonnet-4-6, gpt-5.4). Empty for default."
                            },
                            "cwd": {
                                "type": "string",
                                "description": "Working directory for the agent"
                            },
                            "prompt": {
                                "type": "string",
                                "description": "Initial prompt/task for the agent"
                            },
                            "wait": {
                                "type": "boolean",
                                "description": "Wait for agent to complete (true) or return immediately (false). Default: true",
                                "default": true
                            },
                            "timeoutSecs": {
                                "type": "integer",
                                "description": "Max seconds to wait (only when wait=true). Default: 300",
                                "default": 300
                            }
                        },
                        "required": ["cwd", "prompt"]
                    }
                },
                {
                    "name": "orbit_get_status",
                    "description": "Check the status of a running agent. Returns status (running/completed/not_found) and output collected so far.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "sessionId": {
                                "type": "integer",
                                "description": "Session ID returned by orbit_create_agent"
                            }
                        },
                        "required": ["sessionId"]
                    }
                },
                {
                    "name": "orbit_send_message",
                    "description": "Send a follow-up message to an existing agent session. Restarts the agent with --resume. Currently only supported for claude-code.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "sessionId": {
                                "type": "integer",
                                "description": "Session ID of the agent"
                            },
                            "message": {
                                "type": "string",
                                "description": "Follow-up message to send"
                            }
                        },
                        "required": ["sessionId", "message"]
                    }
                },
                {
                    "name": "orbit_cancel_agent",
                    "description": "Cancel/kill a running agent.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "sessionId": {
                                "type": "integer",
                                "description": "Session ID of the agent to cancel"
                            }
                        },
                        "required": ["sessionId"]
                    }
                }
            ]
        }
    })
}

fn handle_tools_call(state: &mut McpState, id: &Value, params: &Value) -> Value {
    let tool_name = params
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = match tool_name {
        "orbit_create_agent" => tool_create_agent(state, &arguments),
        "orbit_get_status" => tool_get_status(state, &arguments),
        "orbit_send_message" => tool_send_message(state, &arguments),
        "orbit_cancel_agent" => tool_cancel_agent(state, &arguments),
        _ => Err(format!("unknown tool: {tool_name}")),
    };

    match result {
        Ok(content) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&content).unwrap_or_default()
                }]
            }
        }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{
                    "type": "text",
                    "text": format!("Error: {e}")
                }],
                "isError": true
            }
        }),
    }
}

// ── Main loop ───────────────────────────────────────────────────────────────

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut state = McpState::new();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[orbit-mcp] invalid JSON: {e}");
                continue;
            }
        };

        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = msg.get("id").cloned();

        // Notifications (no id) — just acknowledge
        if id.is_none() {
            continue;
        }

        let id = id.unwrap();
        let params = msg.get("params").cloned().unwrap_or(json!({}));

        let response = match method {
            "initialize" => handle_initialize(&id),
            "tools/list" => handle_tools_list(&id),
            "tools/call" => handle_tools_call(&mut state, &id, &params),
            "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("method not found: {method}")
                }
            }),
        };

        let out = serde_json::to_string(&response).unwrap_or_default();
        let mut stdout = stdout.lock();
        let _ = writeln!(stdout, "{out}");
        let _ = stdout.flush();
    }

    // Cleanup: kill all remaining agents
    for (_, mut agent) in state.agents.drain() {
        let _ = agent.child.kill();
        let _ = agent.child.wait();
    }
}
