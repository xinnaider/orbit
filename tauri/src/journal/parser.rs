use std::fs;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

use super::state::{detect_pending_approval, JournalState, RawEntry};
use crate::models::*;

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

/// Extract a short target description from tool input (used by parse_journal).
fn extract_tool_target(tool: &str, input: &Option<serde_json::Value>) -> String {
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

#[cfg(test)]
mod parse_journal_tests {
    use super::*;
    use crate::journal::processor::process_line;
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
            state.entries[0].entry_type,
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
            state.entries[0].entry_type,
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
            state.entries[0].entry_type,
            JournalEntryType::ToolCall,
        );
        t.eq(
            "second is ToolResult",
            state.entries[1].entry_type,
            JournalEntryType::ToolResult,
        );
    }

    #[test]
    fn should_resume_from_file_offset_without_reprocessing_old_lines() {
        let mut t = TestCase::new("should_resume_from_file_offset_without_reprocessing_old_lines");
        t.phase("Seed -- write first line");
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = write_jsonl(&dir, "s.jsonl", &[&assistant_text("First")]);
        let first_state = parse_journal(&path, 0, None);
        let first_size = first_state.file_size;

        t.phase("Seed -- append second line");
        append_jsonl(&path, &[&assistant_text("Second")]);

        t.phase("Act -- resume from offset");
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

        t.phase("Act -- parse_journal path");
        let file_state = parse_journal(&path, 0, None);

        t.phase("Act -- process_line path");
        let mut live_state = JournalState::default();
        for line in &lines {
            process_line(&mut live_state, line);
        }

        t.phase("Assert -- both paths produce same count");
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
