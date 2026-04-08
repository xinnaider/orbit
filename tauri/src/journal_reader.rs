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
    /// Real cost from the Claude `result` message (more accurate than token estimate).
    pub cost_usd: Option<f64>,
}

impl Default for JournalState {
    fn default() -> Self {
        JournalState {
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
            cost_usd: None,
        }
    }
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
            cost_usd: prev.cost_usd,
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
            cost_usd: None,
        },
    };

    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return state,
    };

    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
    if file_size == prev_file_size && prev_file_size > 0 {
        state.status = derive_status_from_tail(path, state.input_tokens, state.output_tokens);
        state.file_size = file_size;
        return state;
    }

    let mut reader = BufReader::new(file);
    if prev_file_size > 0 {
        let _ = reader.seek(SeekFrom::Start(prev_file_size));
    } else {
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

                    // Extract content blocks (thinking, text, tool_use)
                    if let Some(content) = msg.get("content").and_then(|v| v.as_array()) {
                        for block in content {
                            let block_type =
                                block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            match block_type {
                                "thinking" => {
                                    let thinking_text = block
                                        .get("thinking")
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
                                    let text = block
                                        .get("text")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    if !text.is_empty() {
                                        let duration =
                                            last_thinking_ts.take().and_then(|think_ts| {
                                                let t1 = think_ts
                                                    .parse::<chrono::DateTime<chrono::Utc>>()
                                                    .ok()?;
                                                let t2 = ts
                                                    .parse::<chrono::DateTime<chrono::Utc>>()
                                                    .ok()?;
                                                Some((t2 - t1).num_milliseconds() as f64 / 1000.0)
                                            });

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
                                    let tool_name = block
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                        .to_string();
                                    let input = block.get("input").cloned();

                                    let target = extract_tool_target(&tool_name, &input);
                                    state.mini_log.push(MiniLogEntry {
                                        tool: tool_name.clone(),
                                        target: target.clone(),
                                        result: None,
                                        success: None,
                                    });
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
                                if block.get("type").and_then(|v| v.as_str()) == Some("tool_result")
                                {
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
                        } else if let Some(text) = content.as_str() {
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

    state.status = derive_status_from_tail(path, state.input_tokens, state.output_tokens);

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
fn char_boundary(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

fn truncate_output(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        format!("{}...", char_boundary(text, max))
    }
}

fn detect_pending_approval(entries: &[JournalEntry]) -> Option<String> {
    // Walk backwards: if the last tool_call has no tool_result after it, it's pending
    for entry in entries.iter().rev() {
        match entry.entry_type {
            JournalEntryType::ToolResult => return None, // tool was answered
            JournalEntryType::ToolCall => {
                let tool = entry.tool.as_deref().unwrap_or("tool");
                // Bash always auto-runs with --dangerously-skip-permissions
                if tool == "Bash" {
                    return None;
                }
                let target = entry
                    .tool_input
                    .as_ref()
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

/// Flexible contains check that handles optional spaces around colons in JSON.
/// e.g. matches both `"key":"value"` and `"key": "value"`.
fn json_contains(haystack: &str, key: &str, value: &str) -> bool {
    // Try without space and with space
    haystack.contains(&format!("\"{}\":\"{}\"", key, value))
        || haystack.contains(&format!("\"{}\": \"{}\"", key, value))
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

    // Track the last entry's characteristics
    let mut last_type = String::new();
    let mut last_timestamp = String::new();
    let mut awaiting_tool_result = false;
    let mut finished = false; // definitive completion

    let mut line = String::new();
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

        if json_contains(trimmed, "type", "assistant") {
            last_type = "assistant".to_string();
            finished = false;

            if trimmed.contains("\"tool_use\"") {
                awaiting_tool_result = true;
            }
            if json_contains(trimmed, "stop_reason", "end_turn") {
                finished = true;
                awaiting_tool_result = false;
            }
        } else if json_contains(trimmed, "type", "user") {
            last_type = "user".to_string();
            finished = false;

            if trimmed.contains("\"tool_result\"") {
                awaiting_tool_result = false;
            }
        } else if json_contains(trimmed, "type", "progress") {
            last_type = "progress".to_string();
        } else if json_contains(trimmed, "type", "system") {
            if json_contains(trimmed, "subtype", "stop_hook_summary") {
                finished = true;
            }
        } else if json_contains(trimmed, "type", "last-prompt") {
            finished = true;
        }

        if let Some(ts_start) = trimmed.find("\"timestamp\"") {
            // Find the value after "timestamp": " or "timestamp":"
            let after_key = &trimmed[ts_start + 11..];
            if let Some(quote_start) = after_key.find('"') {
                let after_quote = &after_key[quote_start + 1..];
                if let Some(quote_end) = after_quote.find('"') {
                    last_timestamp = after_quote[..quote_end].to_string();
                }
            }
        }
    }

    // Definitive completion signals → idle
    if finished {
        return AgentStatus::Idle;
    }

    // Waiting for user to approve a tool call
    if awaiting_tool_result {
        return AgentStatus::Input;
    }

    // Determine working state based on what the last entry was:
    // - Last entry is "user" (non-tool_result) → agent is processing user input → Working
    // - Last entry is "user" with tool_result → agent got result, processing → Working
    // - Last entry is "assistant" without end_turn → still generating → Working
    // - Last entry is "progress" → actively working
    // Use a generous 120s timeout as safety net (not primary detection)
    let seconds_since_last = if !last_timestamp.is_empty() {
        if let Ok(dt) = last_timestamp.parse::<chrono::DateTime<chrono::Utc>>() {
            (chrono::Utc::now() - dt).num_seconds()
        } else {
            999
        }
    } else {
        999
    };

    // Safety net: if nothing happened for 2 minutes, assume idle
    if seconds_since_last > 120 {
        return AgentStatus::Idle;
    }

    match last_type.as_str() {
        // User sent a message or tool result came back → agent should be working
        "user" => AgentStatus::Working,
        // Assistant is still generating (no end_turn)
        "assistant" => AgentStatus::Working,
        // Progress event → actively working
        "progress" => AgentStatus::Working,
        _ => AgentStatus::Idle,
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
                        session_id: String::new(),
                        timestamp: ts.clone(),
                        entry_type: JournalEntryType::Progress,
                        text: Some(content.to_string()),
                        thinking: None,
                        thinking_duration: None,
                        tool: None,
                        tool_input: None,
                        output: None,
                        exit_code: None,
                        lines_changed: None,
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
                                        session_id: String::new(),
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
                                let text = block
                                    .get("text")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if !text.is_empty() {
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

            if end_turn {
                state.status = AgentStatus::Idle;
                state.pending_approval = None;
            } else if has_non_bash_tool {
                // Non-Bash tools may need approval
                state.status = AgentStatus::Input;
                state.pending_approval = detect_pending_approval(&state.entries);
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
                    } else if let Some(text) = content.as_str() {
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

        "result" => {
            // Final message from Claude with actual cost_usd
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let Some(cost) = val.get("cost_usd").and_then(|v| v.as_f64()) {
                    if cost > 0.0 {
                        state.cost_usd =
                            Some(state.cost_usd.map(|prev| prev + cost).unwrap_or(cost));
                    }
                }
            }
            state.status = AgentStatus::Idle;
        }

        _ => {}
    }
}

#[cfg(test)]
mod process_line_tests {
    use super::*;
    use crate::test_utils::{
        assistant_end_turn, assistant_text, assistant_thinking, assistant_tool_use,
        assistant_with_tokens, progress_line, system_stop_hook, tool_result, user_text, TestCase,
    };

    // ── Noop / guard cases ───────────────────────────────────────────────

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

    // ── Assistant text ───────────────────────────────────────────────────

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
            state.entries[0].entry_type.clone(),
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
        // input=100, output=50, cache_write=20, cache_read=30 → total input = 150
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

    // ── Thinking ─────────────────────────────────────────────────────────

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
            state.entries[0].entry_type.clone(),
            JournalEntryType::Thinking,
        );
        t.eq(
            "thinking text",
            state.entries[0].thinking.as_deref(),
            Some("Let me reason..."),
        );
    }

    // ── Tool use ─────────────────────────────────────────────────────────

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
            state.entries[0].entry_type.clone(),
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
        // Bash auto-runs — no approval needed, stays Working
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
    fn should_set_pending_approval_for_non_bash_tool() {
        let mut t = TestCase::new("should_set_pending_approval_for_non_bash_tool");
        t.phase("Act");
        let mut state = JournalState::default();
        process_line(
            &mut state,
            &assistant_tool_use(
                "CustomTool",
                serde_json::json!({"file_path": "/etc/passwd"}),
            ),
        );
        t.phase("Assert");
        t.some("pending_approval set", &state.pending_approval);
    }

    #[test]
    fn should_clear_pending_approval_after_tool_result() {
        let mut t = TestCase::new("should_clear_pending_approval_after_tool_result");
        t.phase("Seed — tool_use sets pending");
        let mut state = JournalState::default();
        process_line(
            &mut state,
            &assistant_tool_use(
                "CustomTool",
                serde_json::json!({"file_path": "/etc/passwd"}),
            ),
        );
        t.phase("Act — tool_result clears pending");
        process_line(&mut state, &tool_result("done"));
        t.phase("Assert");
        t.none("pending_approval cleared", &state.pending_approval);
    }

    // ── mini_log ─────────────────────────────────────────────────────────

    #[test]
    fn should_cap_mini_log_at_4_entries() {
        let mut t = TestCase::new("should_cap_mini_log_at_4_entries");
        t.phase("Seed — push 5 tool uses");
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

    // ── User ─────────────────────────────────────────────────────────────

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
            state.entries[0].entry_type.clone(),
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

    // ── System ───────────────────────────────────────────────────────────

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

    // ── Progress ─────────────────────────────────────────────────────────

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
            state.entries[0].entry_type.clone(),
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

    // ── Legacy tests kept for regression coverage ────────────────────────

    #[test]
    fn test_process_empty_line_is_noop() {
        let mut state = JournalState::default();
        process_line(&mut state, "");
        assert!(state.entries.is_empty());
    }

    #[test]
    fn test_process_invalid_json_is_noop() {
        let mut state = JournalState::default();
        process_line(&mut state, "not json");
        assert!(state.entries.is_empty());
    }

    #[test]
    fn test_process_assistant_text_creates_entry() {
        let mut state = JournalState::default();
        let line = r#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","content":[{"type":"text","text":"Hello!"}],"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        process_line(&mut state, line);
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].entry_type, JournalEntryType::Assistant);
        assert_eq!(state.entries[0].text.as_deref(), Some("Hello!"));
        assert_eq!(state.model.as_deref(), Some("claude-sonnet-4-6"));
        assert_eq!(state.output_tokens, 5);
    }

    #[test]
    fn test_process_bash_tool_use_stays_working() {
        // Bash auto-runs — status stays Working, not Input
        let mut state = JournalState::default();
        let line = r#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","content":[{"type":"tool_use","name":"Bash","input":{"command":"ls"}}],"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        process_line(&mut state, line);
        assert_eq!(state.status, AgentStatus::Working);
        assert_eq!(state.entries[0].entry_type, JournalEntryType::ToolCall);
    }

    #[test]
    fn test_process_end_turn_sets_idle_status() {
        let mut state = JournalState::default();
        let line = r#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","stop_reason":"end_turn","content":[{"type":"text","text":"Done."}],"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        process_line(&mut state, line);
        assert_eq!(state.status, AgentStatus::Idle);
    }

    #[test]
    fn test_process_thinking_block_creates_thinking_entry() {
        let mut state = JournalState::default();
        let line = r#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","content":[{"type":"thinking","thinking":"Let me reason..."}],"usage":{"input_tokens":5,"output_tokens":2,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        process_line(&mut state, line);
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].entry_type, JournalEntryType::Thinking);
        assert_eq!(
            state.entries[0].thinking.as_deref(),
            Some("Let me reason...")
        );
    }

    #[test]
    fn test_process_user_plain_text_creates_user_entry() {
        let mut state = JournalState::default();
        let line = r#"{"type":"user","message":{"content":"Fix the bug"}}"#;
        process_line(&mut state, line);
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].entry_type, JournalEntryType::User);
        assert_eq!(state.entries[0].text.as_deref(), Some("Fix the bug"));
        assert_eq!(state.status, AgentStatus::Working);
    }

    #[test]
    fn test_process_tool_result_creates_tool_result_entry() {
        let mut state = JournalState::default();
        // First send a tool_use so mini_log has an entry to update
        let tool_line = r#"{"type":"assistant","message":{"model":"m","content":[{"type":"tool_use","name":"Bash","input":{"command":"ls"}}],"usage":{"input_tokens":1,"output_tokens":1,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        process_line(&mut state, tool_line);

        let result_line = r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"file1.rs\nfile2.rs"}]}}"#;
        process_line(&mut state, result_line);

        let tool_result = state
            .entries
            .iter()
            .find(|e| e.entry_type == JournalEntryType::ToolResult);
        assert!(tool_result.is_some());
        assert!(tool_result
            .unwrap()
            .output
            .as_ref()
            .unwrap()
            .contains("file1.rs"));
        assert_eq!(state.status, AgentStatus::Working);
    }

    #[test]
    fn test_process_system_stop_hook_sets_idle() {
        let mut state = JournalState::default();
        state.status = AgentStatus::Working;
        let line = r#"{"type":"system","message":{"subtype":"stop_hook_summary"}}"#;
        process_line(&mut state, line);
        assert_eq!(state.status, AgentStatus::Idle);
    }

    #[test]
    fn test_process_mini_log_capped_at_4() {
        let mut state = JournalState::default();
        let tool_line = |name: &str| {
            format!(
                r#"{{"type":"assistant","message":{{"model":"m","content":[{{"type":"tool_use","name":"{}","input":{{"command":"x"}}}}],"usage":{{"input_tokens":1,"output_tokens":1,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}}}}"#,
                name
            )
        };

        for name in ["Bash", "Read", "Write", "Grep", "Edit"] {
            process_line(&mut state, &tool_line(name));
        }

        assert_eq!(state.mini_log.len(), 4);
        // First entry (Bash) was evicted
        assert_ne!(state.mini_log[0].tool, "Bash");
    }

    #[test]
    fn test_process_token_accumulation() {
        let mut state = JournalState::default();
        let line = r#"{"type":"assistant","message":{"model":"m","content":[{"type":"text","text":"hi"}],"usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":20,"cache_read_input_tokens":30}}}"#;
        process_line(&mut state, line);
        // input_tokens + cache_creation + cache_read = 100 + 20 + 30 = 150
        assert_eq!(state.input_tokens, 150);
        assert_eq!(state.output_tokens, 50);
        assert_eq!(state.cache_write, 20);
        assert_eq!(state.cache_read, 30);
    }

    #[test]
    fn test_process_bash_does_not_set_pending_approval() {
        let mut state = JournalState::default();
        // Bash auto-runs with --dangerously-skip-permissions — no approval needed
        let tool_line = r#"{"type":"assistant","message":{"model":"m","content":[{"type":"tool_use","name":"Bash","input":{"command":"rm -rf /"}}],"usage":{"input_tokens":1,"output_tokens":1,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        process_line(&mut state, tool_line);
        assert!(state.pending_approval.is_none());
        assert_eq!(state.status, AgentStatus::Working);
    }

    #[test]
    fn test_process_non_bash_tool_sets_pending_approval() {
        let mut state = JournalState::default();
        // Non-Bash tools (e.g. a custom tool) may need approval
        let tool_line = r#"{"type":"assistant","message":{"model":"m","content":[{"type":"tool_use","name":"CustomTool","input":{"file_path":"/etc/passwd"}}],"usage":{"input_tokens":1,"output_tokens":1,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        process_line(&mut state, tool_line);
        assert!(state.pending_approval.is_some());

        // Tool result → pending_approval cleared
        let result_line =
            r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"done"}]}}"#;
        process_line(&mut state, result_line);
        assert!(state.pending_approval.is_none());
    }

    #[test]
    fn test_process_synthetic_message_is_skipped() {
        let mut state = JournalState::default();
        let line = r#"{"type":"assistant","message":{"content":"<synthetic>"}}"#;
        process_line(&mut state, line);
        assert!(state.entries.is_empty());
    }
}

#[cfg(test)]
mod parse_journal_tests {
    use super::*;
    use crate::test_utils::{
        append_jsonl, assistant_end_turn, assistant_text, assistant_thinking, assistant_tool_use,
        tool_result, write_jsonl, TestCase,
    };

    #[test]
    fn should_return_default_state_when_file_does_not_exist() {
        let mut t = TestCase::new("should_return_default_state_when_file_does_not_exist");
        t.phase("Act");
        let state = parse_journal(std::path::Path::new("/nonexistent/path.jsonl"), 0, None);
        t.phase("Assert");
        t.empty("no entries", &state.entries);
        t.eq("input_tokens is 0", state.input_tokens, 0u64);
    }

    #[test]
    fn should_parse_assistant_text_entry_from_file() {
        let mut t = TestCase::new("should_parse_assistant_text_entry_from_file");
        t.phase("Seed");
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = write_jsonl(&dir, "s.jsonl", &[&assistant_text("Hello from file!")]);
        t.phase("Act");
        let state = parse_journal(&path, 0, None);
        t.phase("Assert");
        t.len("one entry", &state.entries, 1);
        t.eq(
            "entry type is Assistant",
            state.entries[0].entry_type.clone(),
            JournalEntryType::Assistant,
        );
        t.eq(
            "text matches",
            state.entries[0].text.as_deref(),
            Some("Hello from file!"),
        );
    }

    #[test]
    fn should_parse_thinking_entry_from_file() {
        let mut t = TestCase::new("should_parse_thinking_entry_from_file");
        t.phase("Seed");
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = write_jsonl(&dir, "s.jsonl", &[&assistant_thinking("deep thoughts")]);
        t.phase("Act");
        let state = parse_journal(&path, 0, None);
        t.phase("Assert");
        t.len("one entry", &state.entries, 1);
        t.eq(
            "entry type is Thinking",
            state.entries[0].entry_type.clone(),
            JournalEntryType::Thinking,
        );
        t.eq(
            "thinking text",
            state.entries[0].thinking.as_deref(),
            Some("deep thoughts"),
        );
    }

    #[test]
    fn should_parse_tool_use_and_result_sequence() {
        let mut t = TestCase::new("should_parse_tool_use_and_result_sequence");
        t.phase("Seed");
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = write_jsonl(
            &dir,
            "s.jsonl",
            &[
                &assistant_tool_use("Read", serde_json::json!({"file_path": "/src/main.rs"})),
                &tool_result("pub fn main() {}"),
            ],
        );
        t.phase("Act");
        let state = parse_journal(&path, 0, None);
        t.phase("Assert");
        t.len("two entries", &state.entries, 2);
        t.eq(
            "first is ToolCall",
            state.entries[0].entry_type.clone(),
            JournalEntryType::ToolCall,
        );
        t.eq(
            "second is ToolResult",
            state.entries[1].entry_type.clone(),
            JournalEntryType::ToolResult,
        );
    }

    #[test]
    fn should_resume_from_file_offset_without_reprocessing_old_lines() {
        let mut t = TestCase::new("should_resume_from_file_offset_without_reprocessing_old_lines");
        t.phase("Seed — write first line");
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = write_jsonl(&dir, "s.jsonl", &[&assistant_text("First")]);
        let first_state = parse_journal(&path, 0, None);
        let first_size = first_state.file_size;

        t.phase("Seed — append second line");
        append_jsonl(&path, &[&assistant_text("Second")]);

        t.phase("Act — resume from offset");
        let resumed = parse_journal(&path, first_size, Some(&first_state));

        t.phase("Assert");
        t.len("two total entries (no duplication)", &resumed.entries, 2);
        t.eq(
            "first entry unchanged",
            resumed.entries[0].text.as_deref(),
            Some("First"),
        );
        t.eq(
            "second entry added",
            resumed.entries[1].text.as_deref(),
            Some("Second"),
        );
    }

    #[test]
    fn should_return_same_entry_count_as_process_line_for_identical_input() {
        let mut t =
            TestCase::new("should_return_same_entry_count_as_process_line_for_identical_input");
        t.phase("Seed");
        let lines = [
            assistant_text("Hello"),
            assistant_tool_use("Read", serde_json::json!({"file_path": "/x"})),
            tool_result("contents"),
        ];
        let dir = tempfile::TempDir::new().expect("tempdir");
        let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        let path = write_jsonl(&dir, "s.jsonl", &line_refs);

        t.phase("Act — parse_journal path");
        let file_state = parse_journal(&path, 0, None);

        t.phase("Act — process_line path");
        let mut live_state = JournalState::default();
        for line in &lines {
            process_line(&mut live_state, line);
        }

        t.phase("Assert — both paths produce same count");
        t.eq(
            "parse_journal and process_line agree on entry count",
            file_state.entries.len(),
            live_state.entries.len(),
        );
    }

    #[test]
    fn should_set_idle_status_after_end_turn_in_file() {
        let mut t = TestCase::new("should_set_idle_status_after_end_turn_in_file");
        t.phase("Seed");
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = write_jsonl(&dir, "s.jsonl", &[&assistant_end_turn("Done.")]);
        t.phase("Act");
        let state = parse_journal(&path, 0, None);
        t.phase("Assert");
        t.eq(
            "status is Idle after end_turn",
            state.status,
            AgentStatus::Idle,
        );
    }
}

#[cfg(test)]
mod helper_tests {
    use super::*;
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
    fn should_detect_pending_approval_for_last_unanswered_tool_call() {
        let mut t = TestCase::new("should_detect_pending_approval_for_last_unanswered_tool_call");
        t.phase("Seed");
        let entries = vec![crate::models::JournalEntry {
            session_id: String::new(),
            timestamp: String::new(),
            entry_type: crate::models::JournalEntryType::ToolCall,
            text: None,
            thinking: None,
            thinking_duration: None,
            tool: Some("CustomTool".to_string()),
            tool_input: Some(serde_json::json!({"file_path": "/secret"})),
            output: None,
            exit_code: None,
            lines_changed: None,
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
                session_id: String::new(),
                timestamp: String::new(),
                entry_type: crate::models::JournalEntryType::ToolCall,
                text: None,
                thinking: None,
                thinking_duration: None,
                tool: Some("CustomTool".to_string()),
                tool_input: None,
                output: None,
                exit_code: None,
                lines_changed: None,
            },
            crate::models::JournalEntry {
                session_id: String::new(),
                timestamp: String::new(),
                entry_type: crate::models::JournalEntryType::ToolResult,
                text: None,
                thinking: None,
                thinking_duration: None,
                tool: None,
                tool_input: None,
                output: Some("result".to_string()),
                exit_code: None,
                lines_changed: None,
            },
        ];
        t.phase("Act");
        let result = detect_pending_approval(&entries);
        t.phase("Assert");
        t.none("no pending approval when tool_result exists", &result);
    }
}
