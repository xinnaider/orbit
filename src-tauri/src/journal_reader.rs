use std::fs;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::models::*;

/// Accumulated state from parsing a JSONL file.
#[derive(Debug, Clone)]
pub struct JournalState {
    pub entries: Vec<JournalEntry>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub model: Option<String>,
    pub last_activity: Option<String>,
    pub status: AgentStatus,
    pub pending_approval: Option<String>,
    pub mini_log: Vec<MiniLogEntry>,
    pub file_size: u64,
}

/// Raw JSONL entry from Claude Code logs.
#[derive(Deserialize)]
struct RawEntry {
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    message: Option<Value>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    data: Option<Value>,
}

/// Parse a JSONL file incrementally from `prev_file_size`, returning full journal state.
pub fn parse_journal(
    path: &Path,
    prev_file_size: u64,
    prev_state: Option<&JournalState>,
) -> JournalState {
    let mut state = match prev_state {
        Some(prev) => JournalState {
            entries: prev.entries.clone(),
            input_tokens: prev.input_tokens,
            output_tokens: prev.output_tokens,
            cache_read: prev.cache_read,
            cache_write: prev.cache_write,
            model: prev.model.clone(),
            last_activity: prev.last_activity.clone(),
            status: AgentStatus::Idle,
            pending_approval: None,
            mini_log: prev.mini_log.clone(),
            file_size: prev.file_size,
        },
        None => JournalState {
            entries: Vec::new(),
            input_tokens: 0,
            output_tokens: 0,
            cache_read: 0,
            cache_write: 0,
            model: None,
            last_activity: None,
            status: AgentStatus::New,
            pending_approval: None,
            mini_log: Vec::new(),
            file_size: 0,
        },
    };

    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return state,
    };

    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
    if file_size == prev_file_size && prev_file_size > 0 {
        // Re-derive status from tail even if no new data
        state.status = derive_status_from_tail(path, state.input_tokens, state.output_tokens);
        state.file_size = file_size;
        return state;
    }

    let mut reader = BufReader::new(file);
    if prev_file_size > 0 {
        let _ = reader.seek(SeekFrom::Start(prev_file_size));
    } else {
        // Fresh parse — reset accumulators
        state.entries.clear();
        state.input_tokens = 0;
        state.output_tokens = 0;
        state.cache_read = 0;
        state.cache_write = 0;
        state.model = None;
        state.mini_log.clear();
    }

    let mut line = String::new();
    let mut last_thinking_ts: Option<String> = None;

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let raw: RawEntry = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let ts = raw.timestamp.clone().unwrap_or_default();

        match raw.r#type.as_str() {
            "assistant" => {
                if trimmed.contains("\"<synthetic>\"") {
                    continue;
                }
                state.last_activity = raw.timestamp.clone();

                if let Some(msg) = &raw.message {
                    // Extract model
                    if let Some(m) = msg.get("model").and_then(|v| v.as_str()) {
                        state.model = Some(m.to_string());
                    }

                    // Extract token usage (cumulative)
                    if let Some(usage) = msg.get("usage") {
                        state.input_tokens = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0)
                            + usage.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0)
                            + usage.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                        state.output_tokens = usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                        state.cache_read = usage.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                        state.cache_write = usage.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    }

                    // Extract content blocks (thinking, text, tool_use)
                    if let Some(content) = msg.get("content").and_then(|v| v.as_array()) {
                        for block in content {
                            let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            match block_type {
                                "thinking" => {
                                    let thinking_text = block.get("thinking")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    if !thinking_text.is_empty() {
                                        last_thinking_ts = Some(ts.clone());
                                        state.entries.push(JournalEntry {
                                            session_id: String::new(), // filled by caller
                                            timestamp: ts.clone(),
                                            entry_type: JournalEntryType::Thinking,
                                            text: None,
                                            thinking: Some(thinking_text),
                                            thinking_duration: None,
                                            tool: None,
                                            tool_input: None,
                                            output: None,
                                            exit_code: None,
                                            lines_changed: None,
                                        });
                                    }
                                }
                                "text" => {
                                    let text = block.get("text")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    if !text.is_empty() {
                                        // Calculate thinking duration if we had a thinking block
                                        let duration = last_thinking_ts.take().and_then(|think_ts| {
                                            let t1 = think_ts.parse::<chrono::DateTime<chrono::Utc>>().ok()?;
                                            let t2 = ts.parse::<chrono::DateTime<chrono::Utc>>().ok()?;
                                            Some((t2 - t1).num_milliseconds() as f64 / 1000.0)
                                        });

                                        // Update thinking_duration on the last thinking entry
                                        if let Some(d) = duration {
                                            if let Some(last) = state.entries.last_mut() {
                                                if last.entry_type == JournalEntryType::Thinking {
                                                    last.thinking_duration = Some(d);
                                                }
                                            }
                                        }

                                        state.entries.push(JournalEntry {
                                            session_id: String::new(),
                                            timestamp: ts.clone(),
                                            entry_type: JournalEntryType::Assistant,
                                            text: Some(text),
                                            thinking: None,
                                            thinking_duration: None,
                                            tool: None,
                                            tool_input: None,
                                            output: None,
                                            exit_code: None,
                                            lines_changed: None,
                                        });
                                    }
                                }
                                "tool_use" => {
                                    let tool_name = block.get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                        .to_string();
                                    let input = block.get("input").cloned();

                                    // Build mini log entry
                                    let target = extract_tool_target(&tool_name, &input);
                                    state.mini_log.push(MiniLogEntry {
                                        tool: tool_name.clone(),
                                        target: target.clone(),
                                        result: None,
                                        success: None,
                                    });
                                    // Keep only last 4
                                    if state.mini_log.len() > 4 {
                                        state.mini_log.remove(0);
                                    }

                                    state.entries.push(JournalEntry {
                                        session_id: String::new(),
                                        timestamp: ts.clone(),
                                        entry_type: JournalEntryType::ToolCall,
                                        text: None,
                                        thinking: None,
                                        thinking_duration: None,
                                        tool: Some(tool_name),
                                        tool_input: input,
                                        output: None,
                                        exit_code: None,
                                        lines_changed: None,
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            "user" => {
                state.last_activity = raw.timestamp.clone();

                if let Some(msg) = &raw.message {
                    if let Some(content) = msg.get("content") {
                        // Tool result (array with tool_result type)
                        if let Some(arr) = content.as_array() {
                            for block in arr {
                                if block.get("type").and_then(|v| v.as_str()) == Some("tool_result") {
                                    let output_text = block.get("content")
                                        .and_then(|v| v.as_str())
                                        .or_else(|| block.get("text").and_then(|v| v.as_str()))
                                        .unwrap_or("")
                                        .to_string();

                                    // Update last mini log entry with result
                                    if let Some(last) = state.mini_log.last_mut() {
                                        last.success = Some(!output_text.contains("error") && !output_text.contains("Error"));
                                    }

                                    state.entries.push(JournalEntry {
                                        session_id: String::new(),
                                        timestamp: ts.clone(),
                                        entry_type: JournalEntryType::ToolResult,
                                        text: None,
                                        thinking: None,
                                        thinking_duration: None,
                                        tool: None,
                                        tool_input: None,
                                        output: Some(truncate_output(&output_text, 2000)),
                                        exit_code: None,
                                        lines_changed: None,
                                    });
                                }
                            }
                        }
                        // Plain user text
                        else if let Some(text) = content.as_str() {
                            if !text.is_empty() {
                                state.entries.push(JournalEntry {
                                    session_id: String::new(),
                                    timestamp: ts.clone(),
                                    entry_type: JournalEntryType::User,
                                    text: Some(text.to_string()),
                                    thinking: None,
                                    thinking_duration: None,
                                    tool: None,
                                    tool_input: None,
                                    output: None,
                                    exit_code: None,
                                    lines_changed: None,
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Derive current status from the tail of the file
    state.status = derive_status_from_tail(path, state.input_tokens, state.output_tokens);

    // Check for pending approval (last entry is tool_use with no tool_result)
    state.pending_approval = detect_pending_approval(&state.entries);

    state.file_size = file_size;
    state
}

/// Extract a short target description from tool input.
fn extract_tool_target(tool: &str, input: &Option<Value>) -> String {
    let input = match input {
        Some(v) => v,
        None => return String::new(),
    };

    match tool {
        "Bash" => {
            input.get("command")
                .and_then(|v| v.as_str())
                .map(|cmd| {
                    let first = cmd.lines().next().unwrap_or(cmd);
                    if first.len() > 60 { format!("{}...", &first[..first.floor_char_boundary(60)]) } else { first.to_string() }
                })
                .unwrap_or_default()
        }
        "Read" | "Edit" | "Write" => {
            input.get("file_path")
                .and_then(|v| v.as_str())
                .map(|p| {
                    // Show just the basename
                    p.rsplit(&['/', '\\']).next().unwrap_or(p).to_string()
                })
                .unwrap_or_default()
        }
        "Grep" => {
            input.get("pattern")
                .and_then(|v| v.as_str())
                .map(|p| {
                    if p.len() > 30 { format!("{}...", &p[..p.floor_char_boundary(30)]) } else { p.to_string() }
                })
                .unwrap_or_default()
        }
        "Agent" => {
            input.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("subagent")
                .to_string()
        }
        _ => String::new(),
    }
}

fn truncate_output(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        let end = text.floor_char_boundary(max);
        format!("{}...", &text[..end])
    }
}

fn detect_pending_approval(entries: &[JournalEntry]) -> Option<String> {
    // Walk backwards: if the last tool_call has no tool_result after it, it's pending
    for entry in entries.iter().rev() {
        match entry.entry_type {
            JournalEntryType::ToolResult => return None, // tool was answered
            JournalEntryType::ToolCall => {
                let tool = entry.tool.as_deref().unwrap_or("tool");
                let target = entry.tool_input.as_ref()
                    .and_then(|v| v.get("file_path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                return Some(format!("Allow {} to {}?", tool, target));
            }
            _ => {}
        }
    }
    None
}

/// Derive agent status from the tail of the JSONL file.
fn derive_status_from_tail(path: &Path, input_tokens: u64, output_tokens: u64) -> AgentStatus {
    if input_tokens == 0 && output_tokens == 0 {
        return AgentStatus::New;
    }

    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return AgentStatus::Idle,
    };

    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut reader = BufReader::new(file);
    let seek_pos = file_size.saturating_sub(32768);
    let _ = reader.seek(SeekFrom::Start(seek_pos));

    let mut last_type = String::new();
    let mut last_timestamp = String::new();
    let mut awaiting_tool_result = false;
    let mut saw_end_turn = false;
    let mut saw_stop_hook = false;
    let mut saw_last_prompt = false;

    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        if trimmed.contains("\"type\":\"assistant\"") {
            last_type = "assistant".to_string();
            if trimmed.contains("\"type\":\"tool_use\"") {
                awaiting_tool_result = true;
                saw_end_turn = false;
            }
            // Detect stop_reason: "end_turn" — agent finished responding
            if trimmed.contains("\"stop_reason\":\"end_turn\"") {
                saw_end_turn = true;
                awaiting_tool_result = false;
            }
        } else if trimmed.contains("\"type\":\"user\"") {
            last_type = "user".to_string();
            if trimmed.contains("\"tool_result\"") {
                awaiting_tool_result = false;
            }
            saw_end_turn = false;
        } else if trimmed.contains("\"type\":\"progress\"") {
            last_type = "progress".to_string();
        } else if trimmed.contains("\"type\":\"system\"") {
            // Detect stop_hook_summary — post-completion hook
            if trimmed.contains("\"subtype\":\"stop_hook_summary\"") {
                saw_stop_hook = true;
            }
        } else if trimmed.contains("\"type\":\"last-prompt\"") {
            // Final entry in a completed session
            saw_last_prompt = true;
        }

        if let Some(ts_start) = trimmed.find("\"timestamp\":\"") {
            let after = &trimmed[ts_start + 13..];
            if let Some(ts_end) = after.find('"') {
                last_timestamp = after[..ts_end].to_string();
            }
        }
    }

    // If we saw definitive completion signals, agent is idle
    if saw_last_prompt || saw_stop_hook {
        return AgentStatus::Idle;
    }

    if awaiting_tool_result {
        return AgentStatus::Input;
    }

    // end_turn without tool_use means the assistant finished its turn
    if saw_end_turn {
        return AgentStatus::Idle;
    }

    if !last_timestamp.is_empty() {
        if let Ok(dt) = last_timestamp.parse::<chrono::DateTime<chrono::Utc>>() {
            if (chrono::Utc::now() - dt).num_seconds() < 10 {
                return AgentStatus::Working;
            }
        }
    }

    if last_type == "progress" {
        return AgentStatus::Working;
    }

    AgentStatus::Idle
}
