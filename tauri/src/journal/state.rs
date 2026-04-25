use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

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
    pub context_window: Option<u64>,
    pub last_activity: Option<String>,
    pub status: AgentStatus,
    pub pending_approval: Option<String>,
    /// JSON-RPC request ID for the pending permission request (ACP providers).
    pub pending_approval_id: Option<serde_json::Value>,
    pub mini_log: Vec<MiniLogEntry>,
    pub attention: AttentionState,
    pub next_seq: u32,
    pub epoch: String,
    pub file_size: u64,
    pub rate_limit: Vec<crate::models::RateLimitInfo>,
    pub cost_usd: Option<f64>,
    pub pending_file_changes: HashMap<String, Option<String>>,
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
            context_window: None,
            last_activity: None,
            status: AgentStatus::New,
            pending_approval: None,
            pending_approval_id: None,
            mini_log: Vec::new(),
            attention: AttentionState {
                requires_attention: false,
                reason: None,
                since: None,
            },
            next_seq: 0,
            epoch: uuid_epoch(),
            file_size: 0,
            rate_limit: Vec::new(),
            cost_usd: None,
            pending_file_changes: HashMap::new(),
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

/// Generate a short epoch identifier for this session run.
fn uuid_epoch() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("e-{ts}")
}

/// Returns the tool name if the last ToolCall entry has no following ToolResult.
/// TODO: Permissions bypassed — this function is temporarily unused.
/// Re-enable when permission dialog is fixed (auto-deny error).
#[allow(dead_code)]
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
                if target.is_empty() {
                    return Some(format!("Allow {tool}?"));
                }
                return Some(format!("Allow {tool} to {target}?"));
            }
            _ => {}
        }
    }
    None
}
