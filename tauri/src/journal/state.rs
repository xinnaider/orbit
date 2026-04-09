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
        }
    }
}

/// Raw JSONL entry from Claude Code logs.
#[derive(Deserialize)]
pub(crate) struct RawEntry {
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub message: Option<Value>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub data: Option<Value>,
}

/// Returns the tool name if the last ToolCall entry has no following ToolResult.
pub(crate) fn detect_pending_approval(entries: &[JournalEntry]) -> Option<String> {
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
