use std::cell::Cell;
use std::sync::{Arc, RwLock};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::agent_tree;
use crate::commands::providers::{build_cli_backends, normalize_session_provider_model};
use crate::models::{JournalEntryType, SessionId, SessionStatus};
use crate::providers::ProviderRegistry;
use crate::services::session_manager::SessionManager;

thread_local! {
    static CONNECTION_ORBIT_SESSION: Cell<Option<i64>> = const { Cell::new(None) };
}

pub struct McpHandler {
    session_manager: Arc<RwLock<SessionManager>>,
    registry: Arc<ProviderRegistry>,
    app: AppHandle,
}

const SERVER_NAME: &str = "orbit-mcp";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

impl McpHandler {
    pub fn new(
        session_manager: Arc<RwLock<SessionManager>>,
        registry: Arc<ProviderRegistry>,
        app: AppHandle,
    ) -> Self {
        Self {
            session_manager,
            registry,
            app,
        }
    }

    pub fn handle(&self, request: &str) -> String {
        let parsed: Value = match serde_json::from_str(request) {
            Ok(v) => v,
            Err(_) => return jsonrpc_error(None, -32700, "Parse error"),
        };

        let id = parsed.get("id").cloned();
        let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = parsed.get("params").cloned().unwrap_or(json!({}));

        match method {
            "initialize" => {
                if let Some(sid) = params.get("_orbitSessionId").and_then(|v| v.as_i64()) {
                    CONNECTION_ORBIT_SESSION.with(|c| c.set(Some(sid)));
                }
                jsonrpc_ok(
                    id,
                    json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": {} },
                        "serverInfo": {
                            "name": SERVER_NAME,
                            "version": SERVER_VERSION,
                        }
                    }),
                )
            }

            "notifications/initialized" => String::new(),

            "tools/list" => jsonrpc_ok(id, json!({ "tools": tools_schema() })),

            "tools/call" => {
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                self.dispatch_tool(id, name, args)
            }

            _ => jsonrpc_error(id, -32601, &format!("Method not found: {method}")),
        }
    }

    fn dispatch_tool(&self, id: Option<Value>, name: &str, args: Value) -> String {
        let result = match name {
            "orbit_create_agent" => self.tool_create_agent(&args),
            "orbit_get_status" => self.tool_get_status(&args),
            "orbit_send_message" => self.tool_send_message(&args),
            "orbit_cancel_agent" => self.tool_cancel_agent(&args),
            "orbit_list_providers" => self.tool_list_providers(),
            "orbit_list_sessions" => self.tool_list_sessions(&args),
            "orbit_get_subagents" => self.tool_get_subagents(&args),
            _ => Err(format!("Unknown tool: {name}")),
        };

        match result {
            Ok(content) => jsonrpc_ok(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": content.to_string()
                    }]
                }),
            ),
            Err(e) => jsonrpc_ok(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": e
                    }],
                    "isError": true
                }),
            ),
        }
    }

    fn tool_create_agent(&self, args: &Value) -> Result<Value, String> {
        let cwd = args
            .get("cwd")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: cwd")?;
        let prompt = args
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: prompt")?;
        let provider_id = args.get("provider").and_then(|v| v.as_str());
        let model = args.get("model").and_then(|v| v.as_str());
        let (provider_id, model) =
            normalize_session_provider_model(self.registry.as_ref(), provider_id, model)?;
        let session_name = args.get("name").and_then(|v| v.as_str());
        let wait = args.get("wait").and_then(|v| v.as_bool()).unwrap_or(true);
        let timeout_secs = args
            .get("timeoutSecs")
            .and_then(|v| v.as_u64())
            .unwrap_or(300);
        let parent_session_id = args
            .get("parentSessionId")
            .and_then(|v| v.as_i64())
            .or_else(|| CONNECTION_ORBIT_SESSION.with(|c| c.get()));

        let mut session = {
            let mut m = self
                .session_manager
                .write()
                .unwrap_or_else(|e| e.into_inner());
            m.init_session(
                cwd,
                session_name,
                "ignore",
                model.as_deref(),
                false,
                provider_id.as_deref(),
                None,
                None,
                None,
            )?
        };

        let session_id = session.id;

        if let Some(parent_id) = parent_session_id {
            let desc = if prompt.len() > 80 {
                format!("{}…", &prompt[..80])
            } else {
                prompt.to_string()
            };
            let prov = provider_id.as_deref().unwrap_or("claude-code");
            let mut m = self
                .session_manager
                .write()
                .unwrap_or_else(|e| e.into_inner());
            let _ = m.db.set_session_parent(session_id, parent_id, 1);
            m.register_mcp_subagent(parent_id, session_id, &desc, prov);
            drop(m);
            session.parent_session_id = Some(parent_id);
            session.depth = 1;
        }

        let _ = self.app.emit("session:created", &session);

        let manager = Arc::clone(&self.session_manager);
        let reg = Arc::clone(&self.registry);
        let app = self.app.clone();
        let prompt_owned = prompt.to_string();
        std::thread::spawn(move || {
            SessionManager::do_spawn(manager, app, session_id, prompt_owned, &reg);
        });

        if !wait {
            return Ok(json!({
                "sessionId": session_id,
                "status": "running"
            }));
        }

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));

            let m = self
                .session_manager
                .read()
                .unwrap_or_else(|e| e.into_inner());

            let session = m.db.get_session(session_id).ok().flatten();
            let status = session
                .as_ref()
                .map(|s| s.status.clone())
                .unwrap_or(SessionStatus::Error);

            let is_terminal = matches!(
                status,
                SessionStatus::Completed | SessionStatus::Stopped | SessionStatus::Error
            );

            let (tokens, context_percent) = if let Some(js) = m.journal_states.get(&session_id) {
                let resolved_model = js
                    .model
                    .as_deref()
                    .or_else(|| session.as_ref().and_then(|s| s.model.as_deref()));
                let provider_id = session
                    .as_ref()
                    .map(|s| s.provider.as_str())
                    .unwrap_or("claude-code");
                let (_window, pct) = crate::services::session_manager::resolve_context_metrics(
                    provider_id,
                    resolved_model,
                    js,
                );
                (
                    json!({
                        "input": js.input_tokens,
                        "output": js.output_tokens,
                        "cacheRead": js.cache_read,
                        "cacheWrite": js.cache_write,
                    }),
                    pct,
                )
            } else {
                (json!(null), 0.0)
            };

            drop(m);

            if is_terminal {
                self.session_manager
                    .write()
                    .unwrap_or_else(|e| e.into_inner())
                    .update_mcp_subagent_status(session_id, status.as_str());
                let output = extract_output(&self.session_manager, session_id);
                let claude_sid = self
                    .session_manager
                    .read()
                    .unwrap_or_else(|e| e.into_inner())
                    .db
                    .get_claude_session_id(session_id)
                    .ok()
                    .flatten();
                let subagents = claude_sid
                    .as_deref()
                    .map(agent_tree::read_subagents)
                    .unwrap_or_default();

                return Ok(json!({
                    "sessionId": session_id,
                    "status": status.as_str(),
                    "output": output,
                    "tokens": tokens,
                    "contextPercent": context_percent,
                    "subagents": subagents,
                }));
            }

            if std::time::Instant::now() >= deadline {
                let output = extract_output(&self.session_manager, session_id);
                return Ok(json!({
                    "sessionId": session_id,
                    "status": status.as_str(),
                    "output": output,
                    "tokens": tokens,
                    "contextPercent": context_percent,
                    "timedOut": true,
                }));
            }
        }
    }

    fn tool_get_status(&self, args: &Value) -> Result<Value, String> {
        let session_id = parse_session_id(args)?;

        let m = self
            .session_manager
            .read()
            .unwrap_or_else(|e| e.into_inner());

        let session =
            m.db.get_session(session_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Session {session_id} not found"))?;

        let (tokens, context_percent, mini_log) =
            if let Some(js) = m.journal_states.get(&session_id) {
                let resolved_model = js.model.as_deref().or(session.model.as_deref());
                let (_window, pct) = crate::services::session_manager::resolve_context_metrics(
                    &session.provider,
                    resolved_model,
                    js,
                );
                (
                    json!({
                        "input": js.input_tokens,
                        "output": js.output_tokens,
                        "cacheRead": js.cache_read,
                        "cacheWrite": js.cache_write,
                    }),
                    pct,
                    js.mini_log.clone(),
                )
            } else {
                (json!(null), 0.0, vec![])
            };

        let claude_sid = m.db.get_claude_session_id(session_id).ok().flatten();

        drop(m);

        let output = extract_output(&self.session_manager, session_id);

        let subagents = claude_sid
            .as_deref()
            .map(agent_tree::read_subagents)
            .unwrap_or_default();

        Ok(json!({
            "sessionId": session_id,
            "status": session.status.as_str(),
            "model": session.model,
            "provider": session.provider,
            "output": output,
            "tokens": tokens,
            "contextPercent": context_percent,
            "miniLog": mini_log,
            "subagents": subagents,
        }))
    }

    fn tool_send_message(&self, args: &Value) -> Result<Value, String> {
        let session_id = parse_session_id(args)?;
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: message")?
            .to_string();

        SessionManager::send_message(
            Arc::clone(&self.session_manager),
            self.app.clone(),
            session_id,
            message,
            Arc::clone(&self.registry),
        )?;

        Ok(json!({ "sessionId": session_id, "status": "message_sent" }))
    }

    fn tool_cancel_agent(&self, args: &Value) -> Result<Value, String> {
        let session_id = parse_session_id(args)?;

        self.session_manager
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .stop_session(session_id)?;

        let _ = self
            .app
            .emit("session:stopped", json!({ "sessionId": session_id }));

        Ok(json!({ "sessionId": session_id, "status": "stopped" }))
    }

    fn tool_list_providers(&self) -> Result<Value, String> {
        let backends = build_cli_backends(&self.registry);
        serde_json::to_value(backends).map_err(|e| e.to_string())
    }

    fn tool_list_sessions(&self, args: &Value) -> Result<Value, String> {
        let status_filter = args.get("status").and_then(|v| v.as_str());

        let mut m = self
            .session_manager
            .write()
            .unwrap_or_else(|e| e.into_inner());

        let mut sessions = m.get_sessions();

        if let Some(filter) = status_filter {
            sessions.retain(|s| s.status.as_str() == filter);
        }

        serde_json::to_value(sessions).map_err(|e| e.to_string())
    }

    fn tool_get_subagents(&self, args: &Value) -> Result<Value, String> {
        let session_id = parse_session_id(args)?;

        let m = self
            .session_manager
            .read()
            .unwrap_or_else(|e| e.into_inner());

        let claude_sid =
            m.db.get_claude_session_id(session_id)
                .ok()
                .flatten()
                .ok_or_else(|| format!("No Claude session ID found for session {session_id}"))?;

        drop(m);

        let subagents = agent_tree::read_subagents(&claude_sid);
        serde_json::to_value(subagents).map_err(|e| e.to_string())
    }
}

fn extract_output(manager: &Arc<RwLock<SessionManager>>, session_id: SessionId) -> String {
    let m = manager.read().unwrap_or_else(|e| e.into_inner());
    let entries = match m.journal_states.get(&session_id) {
        Some(js) => &js.entries,
        None => return String::new(),
    };
    entries
        .iter()
        .filter(|e| e.entry_type == JournalEntryType::Assistant)
        .filter_map(|e| e.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_session_id(args: &Value) -> Result<SessionId, String> {
    args.get("sessionId")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing required parameter: sessionId".to_string())
}

fn jsonrpc_ok(id: Option<Value>, result: Value) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
    .to_string()
}

fn jsonrpc_error(id: Option<Value>, code: i32, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        }
    })
    .to_string()
}

pub fn tools_schema() -> Value {
    json!([
        {
            "name": "orbit_create_agent",
            "description": "Create a new agent session in the Orbit dashboard. IMPORTANT: Before calling this, call orbit_list_providers to discover available providers and their exact model IDs — do NOT guess model names. With wait=true (default), this blocks until the agent completes or times out and returns the full output. With wait=false, it returns immediately with a sessionId — you MUST then poll orbit_get_status in a loop until status is 'completed', 'stopped', or 'error' to get the result.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Display name for the session in the Orbit sidebar. If omitted, defaults to the project folder name."
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory for the agent session"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "Initial prompt to send to the agent"
                    },
                    "provider": {
                        "type": "string",
                        "description": "Provider ID (e.g. 'claude-code', 'codex') or OpenCode subProviders[].id (e.g. 'ollama-cloud', 'openrouter'). Defaults to 'claude-code'."
                    },
                    "model": {
                        "type": "string",
                        "description": "Model ID to use. For OpenCode, use the exact model under the chosen subProvider; prefixed forms like 'ollama-cloud/kimi-k2.6:cloud' are accepted."
                    },
                    "wait": {
                        "type": "boolean",
                        "description": "Wait for session to complete before returning. Default: true."
                    },
                    "timeoutSecs": {
                        "type": "integer",
                        "description": "Max seconds to wait when wait=true. Default: 300."
                    },
                    "parentSessionId": {
                        "type": "integer",
                        "description": "If set, the created session appears as a subagent of this parent session instead of a top-level session in the sidebar."
                    }
                },
                "required": ["cwd", "prompt"]
            }
        },
        {
            "name": "orbit_get_status",
            "description": "Get the current status of an agent session, including tokens, context usage, output text, and subagents. Use this to poll a session created with wait=false — keep calling until status is 'completed', 'stopped', or 'error'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sessionId": {
                        "type": "integer",
                        "description": "The session ID returned by orbit_create_agent"
                    }
                },
                "required": ["sessionId"]
            }
        },
        {
            "name": "orbit_send_message",
            "description": "Send a follow-up message to an existing agent session. The session resumes with the new prompt.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sessionId": {
                        "type": "integer",
                        "description": "The session ID to send the message to"
                    },
                    "message": {
                        "type": "string",
                        "description": "The message text to send"
                    }
                },
                "required": ["sessionId", "message"]
            }
        },
        {
            "name": "orbit_cancel_agent",
            "description": "Cancel a running agent session. Kills the CLI process and marks the session as stopped.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sessionId": {
                        "type": "integer",
                        "description": "The session ID to cancel"
                    }
                },
                "required": ["sessionId"]
            }
        },
        {
            "name": "orbit_list_providers",
            "description": "List all available CLI providers with their capabilities, supported models, OpenCode subProviders, and installation status. ALWAYS call this before orbit_create_agent to discover valid provider IDs, subProviders[].id values, and exact model IDs. Returns id, name, cliAvailable, models[], subProviders[], effortLevels, and capability flags.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "orbit_list_sessions",
            "description": "List all sessions in the dashboard, optionally filtered by status.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "description": "Filter by status: 'initializing', 'running', 'waiting', 'completed', 'stopped', 'error'"
                    }
                }
            }
        },
        {
            "name": "orbit_get_subagents",
            "description": "Get the list of subagents spawned by a session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sessionId": {
                        "type": "integer",
                        "description": "The session ID to get subagents for"
                    }
                },
                "required": ["sessionId"]
            }
        }
    ])
}
