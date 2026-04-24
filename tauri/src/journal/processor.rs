use serde_json::Value;

use super::state::{JournalState, RawEntry};
use crate::models::*;

/// Extract a short target description from tool input.
pub(super) fn extract_tool_target(tool: &str, input: &Option<Value>) -> String {
    let input = match input {
        Some(v) => v,
        None => return String::new(),
    };

    match tool {
        "Bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .map(|cmd| {
                let first = cmd.lines().next().unwrap_or(cmd);
                if first.len() > 60 {
                    format!("{}...", char_boundary(first, 60))
                } else {
                    first.to_string()
                }
            })
            .unwrap_or_default(),
        "Read" | "Edit" | "Write" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|p| p.rsplit(&['/', '\\']).next().unwrap_or(p).to_string())
            .unwrap_or_default(),
        "Grep" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .map(|p| {
                if p.len() > 30 {
                    format!("{}...", char_boundary(p, 30))
                } else {
                    p.to_string()
                }
            })
            .unwrap_or_default(),
        "Agent" => input
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("subagent")
            .to_string(),
        _ => String::new(),
    }
}

/// Find the largest char boundary <= max bytes (stable replacement for floor_char_boundary)
pub(super) fn char_boundary(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

pub(super) fn truncate_output(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        format!("{}...", char_boundary(text, max))
    }
}

fn push_mini_log_entry(
    state: &mut JournalState,
    tool: &str,
    target: String,
    success: Option<bool>,
) {
    if state.mini_log.len() >= 4 {
        state.mini_log.remove(0);
    }
    state.mini_log.push(MiniLogEntry {
        tool: tool.to_string(),
        target,
        result: None,
        success,
    });
}

fn summarize_target(target: &str, max: usize) -> String {
    if target.len() > max {
        format!("{}...", char_boundary(target, max))
    } else {
        target.to_string()
    }
}

fn snapshot_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn count_changed_lines(old_text: &str, new_text: &str) -> LinesChanged {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();
    let shared = old_lines
        .iter()
        .zip(new_lines.iter())
        .take_while(|(old, new)| old == new)
        .count();

    LinesChanged {
        added: new_lines.len().saturating_sub(shared) as u32,
        removed: old_lines.len().saturating_sub(shared) as u32,
    }
}

/// Process a single raw JSONL line from PTY stdout and update state.
/// This is the real-time counterpart to parse_journal (which reads files).
pub fn process_line(state: &mut JournalState, line: &str) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return;
    }

    if trimmed.contains("\"<synthetic>\"") {
        return;
    }

    let raw: RawEntry = match serde_json::from_str(trimmed) {
        Ok(r) => r,
        Err(_) => return,
    };

    let ts = raw.timestamp.clone().unwrap_or_default();

    match raw.r#type.as_str() {
        "progress" => {
            // Streaming output from a running tool (e.g. bash stdout lines)
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                let content = val
                    .get("content")
                    .and_then(|v| v.as_str())
                    .or_else(|| val.get("data").and_then(|v| v.as_str()))
                    .or_else(|| val.get("message").and_then(|v| v.as_str()))
                    .unwrap_or("");
                if !content.trim().is_empty() {
                    state.entries.push(JournalEntry {
                        timestamp: ts.clone(),
                        entry_type: JournalEntryType::Progress,
                        text: Some(content.to_string()),
                        ..JournalEntry::default()
                    });
                    state.status = AgentStatus::Working;
                }
            }
        }

        "assistant" => {
            state.last_activity = raw.timestamp.clone();

            let mut has_tool_use = false;
            let mut has_non_bash_tool = false;
            let mut end_turn = false;

            if let Some(msg) = &raw.message {
                if let Some(m) = msg.get("model").and_then(|v| v.as_str()) {
                    state.model = Some(m.to_string());
                }

                if let Some(usage) = msg.get("usage") {
                    state.input_tokens = usage
                        .get("input_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                        + usage
                            .get("cache_creation_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0)
                        + usage
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                    state.output_tokens = usage
                        .get("output_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    state.cache_read = usage
                        .get("cache_read_input_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    state.cache_write = usage
                        .get("cache_creation_input_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                }

                if let Some(stop) = msg.get("stop_reason").and_then(|v| v.as_str()) {
                    if stop == "end_turn" {
                        end_turn = true;
                    }
                }

                if let Some(content) = msg.get("content").and_then(|v| v.as_array()) {
                    for block in content {
                        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        match block_type {
                            "thinking" => {
                                let thinking_text = block
                                    .get("thinking")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if !thinking_text.is_empty() {
                                    state.entries.push(JournalEntry {
                                        timestamp: ts.clone(),
                                        entry_type: JournalEntryType::Thinking,
                                        thinking: Some(thinking_text),
                                        ..JournalEntry::default()
                                    });
                                }
                            }
                            "text" => {
                                let text = block
                                    .get("text")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if !text.is_empty() {
                                    state.entries.push(JournalEntry {
                                        timestamp: ts.clone(),
                                        entry_type: JournalEntryType::Assistant,
                                        text: Some(text),
                                        ..JournalEntry::default()
                                    });
                                }
                            }
                            "tool_use" => {
                                has_tool_use = true;
                                let tool_name = block
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                if tool_name != "Bash" {
                                    has_non_bash_tool = true;
                                }
                                let input = block.get("input").cloned();
                                let target = extract_tool_target(&tool_name, &input);

                                state.mini_log.push(MiniLogEntry {
                                    tool: tool_name.clone(),
                                    target,
                                    result: None,
                                    success: None,
                                });
                                if state.mini_log.len() > 4 {
                                    state.mini_log.remove(0);
                                }

                                state.entries.push(JournalEntry {
                                    timestamp: ts.clone(),
                                    entry_type: JournalEntryType::ToolCall,
                                    tool: Some(tool_name),
                                    tool_input: input,
                                    ..JournalEntry::default()
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }

            if end_turn {
                state.status = AgentStatus::Idle;
            } else if has_non_bash_tool {
                // TODO: Permission bypass enabled — auto-approve all tools instead of showing dialog
                // Need to fix the auto-deny error that occurs when permission dialog is shown
                state.status = AgentStatus::Working;
            } else if has_tool_use {
                // Bash-only tools auto-run with --dangerously-skip-permissions
                state.status = AgentStatus::Working;
                state.pending_approval = None;
            } else {
                state.status = AgentStatus::Working;
            }
        }

        "user" => {
            state.last_activity = raw.timestamp.clone();

            if let Some(msg) = &raw.message {
                if let Some(content) = msg.get("content") {
                    if let Some(arr) = content.as_array() {
                        for block in arr {
                            if block.get("type").and_then(|v| v.as_str()) == Some("tool_result") {
                                let output_text = block
                                    .get("content")
                                    .and_then(|v| v.as_str())
                                    .or_else(|| block.get("text").and_then(|v| v.as_str()))
                                    .unwrap_or("")
                                    .to_string();

                                if let Some(last) = state.mini_log.last_mut() {
                                    last.success = Some(
                                        !output_text.contains("error")
                                            && !output_text.contains("Error"),
                                    );
                                }

                                state.entries.push(JournalEntry {
                                    timestamp: ts.clone(),
                                    entry_type: JournalEntryType::ToolResult,
                                    output: Some(truncate_output(&output_text, 2000)),
                                    ..JournalEntry::default()
                                });
                            }
                        }
                    } else if let Some(text) = content.as_str() {
                        if !text.is_empty() {
                            state.entries.push(JournalEntry {
                                timestamp: ts.clone(),
                                entry_type: JournalEntryType::User,
                                text: Some(text.to_string()),
                                ..JournalEntry::default()
                            });
                        }
                    }
                    // After user message: Claude goes back to working
                    state.status = AgentStatus::Working;
                    state.pending_approval = None;
                }
            }
        }

        "system" => {
            if let Some(subtype) = raw
                .message
                .as_ref()
                .and_then(|m| m.get("subtype"))
                .and_then(|v| v.as_str())
            {
                if subtype == "stop_hook_summary" {
                    state.status = AgentStatus::Idle;
                }
            }
        }

        "rate_limit_event" => {
            // rate_limit_info lives at the top level of the JSON line, not inside message
            let info = if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                val.get("rate_limit_info").cloned()
            } else {
                raw.message
                    .as_ref()
                    .and_then(|m| m.get("rate_limit_info"))
                    .cloned()
            };
            let info = info.as_ref();
            let status = info
                .and_then(|i| i.get("status"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let limit_type = info
                .and_then(|i| i.get("rateLimitType"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let utilization = info
                .and_then(|i| i.get("utilization"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let resets_at = info
                .and_then(|i| i.get("resetsAt"))
                .and_then(|v| v.as_u64())
                .map(|v| v as i64);
            let is_using_overage = info
                .and_then(|i| i.get("isUsingOverage"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let surpassed_threshold = info
                .and_then(|i| i.get("surpassedThreshold"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let rl_entry = crate::models::RateLimitInfo {
                status: status.to_string(),
                rate_limit_type: limit_type.to_string(),
                utilization,
                resets_at,
                is_using_overage,
                surpassed_threshold,
            };
            if let Some(existing) = state
                .rate_limit
                .iter_mut()
                .find(|r| r.rate_limit_type == limit_type)
            {
                *existing = rl_entry;
            } else {
                state.rate_limit.push(rl_entry);
            }
            if status == "exceeded" || status == "blocked" {
                state.attention = crate::models::AttentionState {
                    requires_attention: true,
                    reason: Some(crate::models::AttentionReason::RateLimit),
                    since: Some(ts.clone()),
                };
            } else if matches!(
                state.attention.reason,
                Some(crate::models::AttentionReason::RateLimit)
            ) {
                state.attention = crate::models::AttentionState {
                    requires_attention: false,
                    reason: None,
                    since: None,
                };
            }
        }

        "result" => {
            state.status = AgentStatus::Idle;
            // Check for 429 error results
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                let is_error = val
                    .get("is_error")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let api_error_status = val
                    .get("api_error_status")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                if is_error && api_error_status == 429 {
                    let error_msg = val
                        .get("result")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Rate limit exceeded");
                    state.entries.push(JournalEntry {
                        timestamp: ts.clone(),
                        entry_type: JournalEntryType::System,
                        text: Some(format!("Rate limit: {error_msg}")),
                        ..JournalEntry::default()
                    });
                    state.attention = crate::models::AttentionState {
                        requires_attention: true,
                        reason: Some(crate::models::AttentionReason::RateLimit),
                        since: Some(ts.clone()),
                    };
                } else if let Some(error_msg) = val.get("result").and_then(|v| v.as_str()) {
                    if is_error {
                        state.entries.push(JournalEntry {
                            timestamp: ts.clone(),
                            entry_type: JournalEntryType::System,
                            text: Some(format!("Error: {error_msg}")),
                            ..JournalEntry::default()
                        });
                    }
                }
                // Extract contextWindow and costUSD from modelUsage in result message
                if let Some(model_usage) = val.get("modelUsage").and_then(|v| v.as_object()) {
                    for (_model, info) in model_usage {
                        if let Some(cw) = info.get("contextWindow").and_then(|v| v.as_u64()) {
                            state.context_window = Some(cw);
                        }
                        if let Some(cost) = info.get("costUSD").and_then(|v| v.as_f64()) {
                            state.cost_usd = Some(state.cost_usd.unwrap_or(0.0) + cost);
                        }
                    }
                }
                if let Some(cost) = val.get("total_cost_usd").and_then(|v| v.as_f64()) {
                    state.cost_usd = Some(cost);
                }
            }
        }

        _ => {}
    }
}

/// Process a JSONL line from OpenCode's `run --format json` output.
pub fn process_line_opencode(state: &mut JournalState, line: &str) {
    let val: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return,
    };

    let event_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match event_type {
        "step_start" => {
            state.status = AgentStatus::Working;
        }

        "text" => {
            let text = val
                .pointer("/part/text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !text.is_empty() {
                state.status = AgentStatus::Working;
                state.entries.push(JournalEntry {
                    entry_type: JournalEntryType::Assistant,
                    text: Some(text.to_string()),
                    ..JournalEntry::default()
                });
            }
        }

        "tool_use" => {
            let tool = val
                .pointer("/part/tool")
                .and_then(|v| v.as_str())
                .unwrap_or("tool");
            let command = val
                .pointer("/part/state/input/command")
                .and_then(|v| v.as_str());
            let description = val
                .pointer("/part/state/input/description")
                .and_then(|v| v.as_str());
            let output = val.pointer("/part/state/output").and_then(|v| v.as_str());
            let exit_code = val
                .pointer("/part/state/metadata/exit")
                .and_then(|v| v.as_i64());

            let tool_name = match tool {
                "bash" => "Bash",
                "read" => "Read",
                "edit" => "Edit",
                "write" => "Write",
                "grep" => "Grep",
                "glob" => "Glob",
                other => other,
            };

            let tool_input = command.map(|c| {
                serde_json::json!({
                    "command": c,
                    "description": description.unwrap_or("")
                })
            });

            let target = description
                .unwrap_or_else(|| command.unwrap_or(""))
                .to_string();
            let target_short = if target.len() > 60 {
                format!("{}...", &target[..60])
            } else {
                target
            };

            state.entries.push(JournalEntry {
                entry_type: JournalEntryType::ToolCall,
                tool: Some(tool_name.to_string()),
                tool_input,
                ..JournalEntry::default()
            });

            if let Some(out) = output {
                state.entries.push(JournalEntry {
                    entry_type: JournalEntryType::ToolResult,
                    output: Some(truncate_output(out, 2000)),
                    exit_code: exit_code.map(|c| c as i32),
                    ..JournalEntry::default()
                });
            }

            if state.mini_log.len() >= 4 {
                state.mini_log.remove(0);
            }
            state.mini_log.push(MiniLogEntry {
                tool: tool_name.to_string(),
                target: target_short,
                result: None,
                success: exit_code.map(|c| c == 0),
            });
        }

        "step_finish" => {
            let reason = val
                .pointer("/part/reason")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if let Some(tokens) = val.pointer("/part/tokens") {
                // Each step reports its own totals — overwrite, don't accumulate
                state.input_tokens = tokens.get("input").and_then(|v| v.as_u64()).unwrap_or(0);
                state.output_tokens = tokens.get("output").and_then(|v| v.as_u64()).unwrap_or(0);
                state.cache_write = tokens
                    .pointer("/cache/write")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                state.cache_read = tokens
                    .pointer("/cache/read")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
            }

            if reason == "stop" {
                state.status = AgentStatus::Idle;
            }
        }

        "error" => {
            let msg = val
                .pointer("/error/data/message")
                .or_else(|| val.pointer("/error/name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            state.entries.push(JournalEntry {
                entry_type: JournalEntryType::System,
                text: Some(format!("Error: {msg}")),
                ..JournalEntry::default()
            });
            state.status = AgentStatus::Idle;
        }

        "rate_limit_event" => {
            let info = val.get("rate_limit_info");
            let status = info
                .and_then(|i| i.get("status"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let limit_type = info
                .and_then(|i| i.get("rateLimitType"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let utilization = info
                .and_then(|i| i.get("utilization"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let resets_at = info
                .and_then(|i| i.get("resetsAt"))
                .and_then(|v| v.as_u64())
                .map(|v| v as i64);
            let is_using_overage = info
                .and_then(|i| i.get("isUsingOverage"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let surpassed_threshold = info
                .and_then(|i| i.get("surpassedThreshold"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let rl_entry = crate::models::RateLimitInfo {
                status: status.to_string(),
                rate_limit_type: limit_type.to_string(),
                utilization,
                resets_at,
                is_using_overage,
                surpassed_threshold,
            };
            if let Some(existing) = state
                .rate_limit
                .iter_mut()
                .find(|r| r.rate_limit_type == limit_type)
            {
                *existing = rl_entry;
            } else {
                state.rate_limit.push(rl_entry);
            }

            if status == "exceeded" || status == "blocked" {
                state.attention = crate::models::AttentionState {
                    requires_attention: true,
                    reason: Some(crate::models::AttentionReason::RateLimit),
                    since: Some(chrono::Utc::now().to_rfc3339()),
                };
            } else if matches!(
                state.attention.reason,
                Some(crate::models::AttentionReason::RateLimit)
            ) {
                state.attention = crate::models::AttentionState {
                    requires_attention: false,
                    reason: None,
                    since: None,
                };
            }
        }

        // User messages are stored in Claude format by emit_spawn_started
        _ => {
            process_line(state, line);
        }
    }
}

/// Extract the inner command from a Codex PowerShell wrapper.
/// Codex on Windows wraps commands as:
///   `"C:\\WINDOWS\\...\\powershell.exe" -Command 'echo hello'`
/// This extracts just `echo hello`.
fn extract_codex_command(raw: &str) -> String {
    if let Some(pos) = raw.find("-Command") {
        let after = raw[pos + 8..].trim_start();
        let cmd = after
            .trim_start_matches('\'')
            .trim_start_matches('"')
            .trim_end_matches('\'')
            .trim_end_matches('"');
        if !cmd.is_empty() {
            return cmd.to_string();
        }
    }
    raw.to_string()
}

/// Process a JSONL line from Codex's `exec --json` output.
pub fn process_line_codex(state: &mut JournalState, line: &str) {
    let val: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return,
    };

    let event_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match event_type {
        "turn.started" => {
            state.status = AgentStatus::Working;
        }

        "item.completed" | "item.started" => {
            let item_type = val
                .pointer("/item/type")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            match item_type {
                "agent_message" => {
                    let text = val
                        .pointer("/item/text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !text.is_empty() {
                        state.entries.push(JournalEntry {
                            entry_type: JournalEntryType::Assistant,
                            text: Some(text.to_string()),
                            ..JournalEntry::default()
                        });
                    }
                }

                "command_execution" => {
                    let raw_command = val
                        .pointer("/item/command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let command = extract_codex_command(raw_command);
                    let status = val
                        .pointer("/item/status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let output = val
                        .pointer("/item/aggregated_output")
                        .and_then(|v| v.as_str());
                    let exit_code = val.pointer("/item/exit_code").and_then(|v| v.as_i64());

                    if event_type == "item.started" || status == "in_progress" {
                        state.entries.push(JournalEntry {
                            entry_type: JournalEntryType::ToolCall,
                            tool: Some("Bash".to_string()),
                            tool_input: Some(serde_json::json!({ "command": command })),
                            ..JournalEntry::default()
                        });
                    } else if status == "completed" {
                        let has_pending_call = state
                            .entries
                            .last()
                            .is_some_and(|e| e.entry_type == JournalEntryType::ToolCall);

                        if !has_pending_call {
                            state.entries.push(JournalEntry {
                                entry_type: JournalEntryType::ToolCall,
                                tool: Some("Bash".to_string()),
                                tool_input: Some(serde_json::json!({ "command": command })),
                                ..JournalEntry::default()
                            });
                        }

                        if let Some(out) = output {
                            state.entries.push(JournalEntry {
                                entry_type: JournalEntryType::ToolResult,
                                output: Some(truncate_output(out, 2000)),
                                exit_code: exit_code.map(|c| c as i32),
                                ..JournalEntry::default()
                            });
                        }

                        push_mini_log_entry(
                            state,
                            "Bash",
                            summarize_target(&command, 60),
                            exit_code.map(|c| c == 0),
                        );
                    }
                }
                "file_change" => {
                    let status = val
                        .pointer("/item/status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let changes = match val.pointer("/item/changes").and_then(|v| v.as_array()) {
                        Some(changes) => changes,
                        None => return,
                    };

                    if event_type == "item.started" || status == "in_progress" {
                        for change in changes {
                            let path = change
                                .get("path")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            if !path.is_empty() {
                                state
                                    .pending_file_changes
                                    .insert(path.clone(), snapshot_file(&path));
                            }
                        }
                        return;
                    }

                    if status != "completed" {
                        return;
                    }

                    for change in changes {
                        let path = change
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if path.is_empty() {
                            continue;
                        }

                        let kind = change
                            .get("kind")
                            .and_then(|v| v.as_str())
                            .unwrap_or("update");
                        let old_content = state.pending_file_changes.remove(&path).flatten();
                        let new_content = snapshot_file(&path);

                        let (tool_name, tool_input, lines_changed) = match kind {
                            "create" => {
                                let content = new_content.clone().unwrap_or_default();
                                let file_path = path.clone();
                                (
                                    "Write",
                                    serde_json::json!({
                                        "file_path": file_path,
                                        "content": content.clone(),
                                    }),
                                    Some(count_changed_lines("", &content)),
                                )
                            }
                            "delete" => {
                                let old_text = old_content.clone().unwrap_or_default();
                                let file_path = path.clone();
                                (
                                    "Edit",
                                    serde_json::json!({
                                        "file_path": file_path,
                                        "old_string": old_text.clone(),
                                        "new_string": "",
                                    }),
                                    Some(count_changed_lines(&old_text, "")),
                                )
                            }
                            _ => {
                                let old_text = old_content.clone().unwrap_or_default();
                                let new_text = new_content.clone().unwrap_or_default();
                                let tool_name = if old_content.is_some() {
                                    "Edit"
                                } else {
                                    "Write"
                                };
                                let tool_input = if old_content.is_some() {
                                    let file_path = path.clone();
                                    serde_json::json!({
                                        "file_path": file_path,
                                        "old_string": old_text.clone(),
                                        "new_string": new_text.clone(),
                                    })
                                } else {
                                    let file_path = path.clone();
                                    serde_json::json!({
                                        "file_path": file_path,
                                        "content": new_text.clone(),
                                    })
                                };
                                (
                                    tool_name,
                                    tool_input,
                                    Some(count_changed_lines(&old_text, &new_text)),
                                )
                            }
                        };

                        state.entries.push(JournalEntry {
                            entry_type: JournalEntryType::ToolCall,
                            tool: Some(tool_name.to_string()),
                            tool_input: Some(tool_input),
                            lines_changed,
                            ..JournalEntry::default()
                        });

                        state.entries.push(JournalEntry {
                            entry_type: JournalEntryType::ToolResult,
                            output: Some(format!("{kind}d {path}")),
                            ..JournalEntry::default()
                        });

                        push_mini_log_entry(
                            state,
                            tool_name,
                            summarize_target(&path, 60),
                            Some(true),
                        );
                    }
                }
                _ => {}
            }
        }

        "turn.completed" => {
            if let Some(usage) = val.get("usage") {
                // Each turn reports its own totals — overwrite, don't accumulate
                state.input_tokens = usage
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                state.output_tokens = usage
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                state.cache_read = usage
                    .get("cached_input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
            }
            state.status = AgentStatus::Idle;
        }

        // User messages are stored in Claude format by emit_spawn_started
        _ => {
            process_line(state, line);
        }
    }
}

#[cfg(test)]
mod process_line_tests {
    use super::*;
    use crate::test_utils::{
        assistant_end_turn, assistant_text, assistant_thinking, assistant_tool_use,
        assistant_with_tokens, progress_line, system_stop_hook, tool_result, user_text, TestCase,
    };

    // -- Noop / guard cases

    #[test]
    fn should_be_noop_for_empty_line() {
        let mut t = TestCase::new("should_be_noop_for_empty_line");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(&mut state, "");
        t.phase("Assert");
        t.empty("no entries", &state.entries);
    }

    #[test]
    fn should_be_noop_for_invalid_json() {
        let mut t = TestCase::new("should_be_noop_for_invalid_json");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(&mut state, "not json {{}}");
        t.phase("Assert");
        t.empty("no entries", &state.entries);
    }

    #[test]
    fn should_skip_synthetic_message() {
        let mut t = TestCase::new("should_skip_synthetic_message");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(
            &mut state,
            r#"{"type":"assistant","message":{"content":"<synthetic>"}}"#,
        );
        t.phase("Assert");
        t.empty("no entries", &state.entries);
    }

    // -- Assistant text

    #[test]
    fn should_create_assistant_entry_for_text_block() {
        let mut t = TestCase::new("should_create_assistant_entry_for_text_block");
        t.phase("Seed");
        let mut state = JournalState::default();
        t.phase("Act");
        process_line(&mut state, &assistant_text("Hello!"));
        t.phase("Assert");
        t.len("one entry", &state.entries, 1);
        t.eq(
            "entry type is Assistant",
            state.entries[0].entry_type,
            JournalEntryType::Assistant,
        );
        t.eq(
            "text matches",
            state.entries[0].text.as_deref(),
            Some("Hello!"),
        );
    }

    #[test]
    fn should_extract_model_from_assistant_message() {
        let mut t = TestCase::new("should_extract_model_from_assistant_message");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(&mut state, &assistant_text("Hi"));
        t.phase("Assert");
        t.eq(
            "model extracted",
            state.model.as_deref(),
            Some("claude-sonnet-4-6"),
        );
    }

    #[test]
    fn should_extract_token_counts_from_assistant_message() {
        let mut t = TestCase::new("should_extract_token_counts_from_assistant_message");
        t.phase("Act");
        let mut state = JournalState::default();
        // input=100, output=50, cache_write=20, cache_read=30 -> total input = 150
        process_line(&mut state, &assistant_with_tokens("hi", 100, 50, 20, 30));
        t.phase("Assert");
        t.eq(
            "input_tokens (input + cache_write + cache_read)",
            state.input_tokens,
            150u64,
        );
        t.eq("output_tokens", state.output_tokens, 50u64);
        t.eq("cache_write", state.cache_write, 20u64);
        t.eq("cache_read", state.cache_read, 30u64);
    }

    #[test]
    fn should_set_idle_status_on_end_turn() {
        let mut t = TestCase::new("should_set_idle_status_on_end_turn");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(&mut state, &assistant_end_turn("Done."));
        t.phase("Assert");
        t.eq("status is Idle", state.status, AgentStatus::Idle);
    }

    // -- Thinking

    #[test]
    fn should_create_thinking_entry_for_thinking_block() {
        let mut t = TestCase::new("should_create_thinking_entry_for_thinking_block");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(&mut state, &assistant_thinking("Let me reason..."));
        t.phase("Assert");
        t.len("one entry", &state.entries, 1);
        t.eq(
            "entry type is Thinking",
            state.entries[0].entry_type,
            JournalEntryType::Thinking,
        );
        t.eq(
            "thinking text",
            state.entries[0].thinking.as_deref(),
            Some("Let me reason..."),
        );
    }

    // -- Tool use

    #[test]
    fn should_create_tool_call_entry_for_bash() {
        let mut t = TestCase::new("should_create_tool_call_entry_for_bash");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(
            &mut state,
            &assistant_tool_use("Bash", serde_json::json!({"command": "ls"})),
        );
        t.phase("Assert");
        t.len("one entry", &state.entries, 1);
        t.eq(
            "entry type is ToolCall",
            state.entries[0].entry_type,
            JournalEntryType::ToolCall,
        );
        t.eq(
            "tool name is Bash",
            state.entries[0].tool.as_deref(),
            Some("Bash"),
        );
    }

    #[test]
    fn should_keep_working_status_after_bash_tool_use() {
        let mut t = TestCase::new("should_keep_working_status_after_bash_tool_use");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(
            &mut state,
            &assistant_tool_use("Bash", serde_json::json!({"command": "ls"})),
        );
        t.phase("Assert");
        // Bash auto-runs -- no approval needed, stays Working
        t.eq("status stays Working", state.status, AgentStatus::Working);
    }

    #[test]
    fn should_not_set_pending_approval_for_bash() {
        let mut t = TestCase::new("should_not_set_pending_approval_for_bash");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(
            &mut state,
            &assistant_tool_use("Bash", serde_json::json!({"command": "rm -rf /"})),
        );
        t.phase("Assert");
        t.none("no pending approval for Bash", &state.pending_approval);
    }

    #[test]
    fn should_not_set_pending_approval_for_non_bash_tool_with_bypass() {
        let mut t = TestCase::new("should_not_set_pending_approval_for_non_bash_tool_with_bypass");
        t.phase("Seed");
        let mut state = JournalState::default();
        process_line(
            &mut state,
            &assistant_tool_use(
                "CustomTool",
                serde_json::json!({"file_path": "/etc/passwd"}),
            ),
        );
        t.phase("Assert");
        t.none("pending_approval bypassed", &state.pending_approval);
    }

    #[test]
    fn should_remain_working_after_tool_result_with_bypass() {
        let mut t = TestCase::new("should_remain_working_after_tool_result_with_bypass");
        t.phase("Seed");
        let mut state = JournalState::default();
        process_line(
            &mut state,
            &assistant_tool_use(
                "CustomTool",
                serde_json::json!({"file_path": "/etc/passwd"}),
            ),
        );
        t.phase("Act -- tool_result processes normally");
        process_line(&mut state, &tool_result("done"));
        t.phase("Assert");
        t.none("pending_approval bypassed", &state.pending_approval);
    }

    // -- mini_log

    #[test]
    fn should_cap_mini_log_at_4_entries() {
        let mut t = TestCase::new("should_cap_mini_log_at_4_entries");
        t.phase("Seed -- push 5 tool uses");
        let mut state = JournalState::default();
        for name in ["Bash", "Read", "Write", "Grep", "Edit"] {
            process_line(
                &mut state,
                &assistant_tool_use(name, serde_json::json!({"command": "x"})),
            );
        }
        t.phase("Assert");
        t.len("mini_log capped at 4", &state.mini_log, 4);
        t.ne(
            "oldest entry evicted",
            state.mini_log[0].tool.as_str(),
            "Bash",
        );
    }

    #[test]
    fn should_mark_mini_log_entry_success_on_tool_result() {
        let mut t = TestCase::new("should_mark_mini_log_entry_success_on_tool_result");
        t.phase("Seed");
        let mut state = JournalState::default();
        process_line(
            &mut state,
            &assistant_tool_use("Bash", serde_json::json!({"command": "ls"})),
        );
        t.phase("Act");
        process_line(&mut state, &tool_result("file1.rs"));
        t.phase("Assert");
        t.eq(
            "mini_log entry marked success",
            state.mini_log[0].success,
            Some(true),
        );
    }

    // -- User

    #[test]
    fn should_create_user_entry_for_plain_text() {
        let mut t = TestCase::new("should_create_user_entry_for_plain_text");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(&mut state, &user_text("Fix the bug"));
        t.phase("Assert");
        t.len("one entry", &state.entries, 1);
        t.eq(
            "entry type is User",
            state.entries[0].entry_type,
            JournalEntryType::User,
        );
        t.eq(
            "text matches",
            state.entries[0].text.as_deref(),
            Some("Fix the bug"),
        );
    }

    #[test]
    fn should_set_working_status_after_user_message() {
        let mut t = TestCase::new("should_set_working_status_after_user_message");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(&mut state, &user_text("do something"));
        t.phase("Assert");
        t.eq("status is Working", state.status, AgentStatus::Working);
    }

    #[test]
    fn should_create_tool_result_entry_from_user_block() {
        let mut t = TestCase::new("should_create_tool_result_entry_from_user_block");
        t.phase("Seed");
        let mut state = JournalState::default();
        process_line(
            &mut state,
            &assistant_tool_use("Bash", serde_json::json!({"command": "ls"})),
        );
        t.phase("Act");
        process_line(&mut state, &tool_result("file1.rs\nfile2.rs"));
        t.phase("Assert");
        let tr = state
            .entries
            .iter()
            .find(|e| e.entry_type == JournalEntryType::ToolResult)
            .expect("no ToolResult entry found");
        t.ok(
            "output contains file1.rs",
            tr.output.as_deref().unwrap_or("").contains("file1.rs"),
        );
    }

    // -- System

    #[test]
    fn should_set_idle_on_stop_hook_summary() {
        let mut t = TestCase::new("should_set_idle_on_stop_hook_summary");
        t.phase("Seed");
        let mut state = JournalState::default();
        state.status = AgentStatus::Working;
        t.phase("Act");
        process_line(&mut state, system_stop_hook());
        t.phase("Assert");
        t.eq("status is Idle", state.status, AgentStatus::Idle);
    }

    // -- Progress

    #[test]
    fn should_create_progress_entry_for_non_empty_content() {
        let mut t = TestCase::new("should_create_progress_entry_for_non_empty_content");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(&mut state, &progress_line("stdout output line"));
        t.phase("Assert");
        t.len("one entry", &state.entries, 1);
        t.eq(
            "entry type is Progress",
            state.entries[0].entry_type,
            JournalEntryType::Progress,
        );
    }

    #[test]
    fn should_be_noop_for_empty_progress_content() {
        let mut t = TestCase::new("should_be_noop_for_empty_progress_content");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(&mut state, &progress_line("   "));
        t.phase("Assert");
        t.empty("no entries for whitespace content", &state.entries);
    }
}

#[cfg(test)]
mod helper_tests {
    use super::*;
    use crate::journal::state::detect_pending_approval;
    use crate::test_utils::TestCase;

    #[test]
    fn should_extract_bash_command_as_tool_target() {
        let mut t = TestCase::new("should_extract_bash_command_as_tool_target");
        t.phase("Act");
        let input = serde_json::json!({"command": "cargo test"});
        let result = extract_tool_target("Bash", &Some(input));
        t.phase("Assert");
        t.eq("target is the command", result.as_str(), "cargo test");
    }

    #[test]
    fn should_extract_filename_as_target_for_read_tool() {
        let mut t = TestCase::new("should_extract_filename_as_target_for_read_tool");
        t.phase("Act");
        let input = serde_json::json!({"file_path": "/src/lib.rs"});
        let result = extract_tool_target("Read", &Some(input));
        t.phase("Assert");
        t.eq("target is the filename only", result.as_str(), "lib.rs");
    }

    #[test]
    fn should_truncate_bash_command_at_60_chars() {
        let mut t = TestCase::new("should_truncate_bash_command_at_60_chars");
        t.phase("Seed");
        let long_cmd = "a".repeat(80);
        let input = serde_json::json!({"command": long_cmd});
        t.phase("Act");
        let result = extract_tool_target("Bash", &Some(input));
        t.phase("Assert");
        t.ok(
            "truncated to at most 63 chars (60 + ...)",
            result.len() <= 63,
        );
        t.ok("ends with ...", result.ends_with("..."));
    }

    #[test]
    fn should_truncate_output_at_max_chars() {
        let mut t = TestCase::new("should_truncate_output_at_max_chars");
        t.phase("Seed");
        let long_text = "x".repeat(3000);
        t.phase("Act");
        let result = truncate_output(&long_text, 2000);
        t.phase("Assert");
        t.ok("result length <= 2003 (2000 + ...)", result.len() <= 2003);
        t.ok("ends with ...", result.ends_with("..."));
    }

    #[test]
    fn should_not_truncate_output_within_limit() {
        let mut t = TestCase::new("should_not_truncate_output_within_limit");
        t.phase("Act");
        let result = truncate_output("short text", 2000);
        t.phase("Assert");
        t.eq("unchanged", result.as_str(), "short text");
    }

    #[test]
    fn should_create_edit_entry_from_codex_file_change() {
        let mut t = TestCase::new("should_create_edit_entry_from_codex_file_change");
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("note.txt");
        std::fs::write(&path, "hello\n").expect("seed file");
        let path_str = path.to_string_lossy().to_string();

        let started = serde_json::json!({
            "type": "item.started",
            "item": {
                "id": "item_1",
                "type": "file_change",
                "status": "in_progress",
                "changes": [{ "path": path_str, "kind": "update" }]
            }
        });

        let mut state = JournalState::default();
        t.phase("Act -- snapshot old file");
        process_line_codex(&mut state, &started.to_string());

        std::fs::write(&path, "hi\n").expect("update file");
        let completed = serde_json::json!({
            "type": "item.completed",
            "item": {
                "id": "item_1",
                "type": "file_change",
                "status": "completed",
                "changes": [{ "path": path.to_string_lossy(), "kind": "update" }]
            }
        });
        process_line_codex(&mut state, &completed.to_string());

        t.phase("Assert");
        let edit = state
            .entries
            .iter()
            .find(|entry| entry.entry_type == JournalEntryType::ToolCall)
            .expect("missing tool call");
        t.eq("tool type", edit.tool.as_deref(), Some("Edit"));
        t.eq(
            "old string captured",
            edit.tool_input
                .as_ref()
                .and_then(|input| input.get("old_string"))
                .and_then(|value| value.as_str()),
            Some("hello\n"),
        );
        t.eq(
            "new string captured",
            edit.tool_input
                .as_ref()
                .and_then(|input| input.get("new_string"))
                .and_then(|value| value.as_str()),
            Some("hi\n"),
        );
        t.eq("mini log created", state.mini_log.len(), 1usize);
    }

    #[test]
    fn should_detect_pending_approval_for_last_unanswered_tool_call() {
        let mut t = TestCase::new("should_detect_pending_approval_for_last_unanswered_tool_call");
        t.phase("Seed");
        let entries = vec![crate::models::JournalEntry {
            entry_type: crate::models::JournalEntryType::ToolCall,
            tool: Some("CustomTool".to_string()),
            tool_input: Some(serde_json::json!({"file_path": "/secret"})),
            ..crate::models::JournalEntry::default()
        }];
        t.phase("Act");
        let result = detect_pending_approval(&entries);
        t.phase("Assert");
        t.some("pending approval detected", &result);
    }

    #[test]
    fn should_not_detect_pending_when_tool_result_follows() {
        let mut t = TestCase::new("should_not_detect_pending_when_tool_result_follows");
        t.phase("Seed");
        let entries = vec![
            crate::models::JournalEntry {
                entry_type: crate::models::JournalEntryType::ToolCall,
                tool: Some("CustomTool".to_string()),
                ..crate::models::JournalEntry::default()
            },
            crate::models::JournalEntry {
                entry_type: crate::models::JournalEntryType::ToolResult,
                output: Some("result".to_string()),
                ..crate::models::JournalEntry::default()
            },
        ];
        t.phase("Act");
        let result = detect_pending_approval(&entries);
        t.phase("Assert");
        t.none("no pending approval when tool_result exists", &result);
    }
}
