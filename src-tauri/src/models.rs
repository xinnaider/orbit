use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AgentStatus {
    Working,
    Input,
    Idle,
    New,
}

impl AgentStatus {
    pub fn label(&self) -> &str {
        match self {
            AgentStatus::Working => "WORKING",
            AgentStatus::Input => "INPUT",
            AgentStatus::Idle => "IDLE",
            AgentStatus::New => "NEW",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniLogEntry {
    pub tool: String,
    pub target: String,
    pub result: Option<String>,
    pub success: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentInfo {
    pub id: String,
    pub agent_type: String,
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentState {
    pub session_id: String,
    pub project: String,
    pub cwd: String,
    pub git_branch: Option<String>,
    pub status: AgentStatus,
    pub model: Option<String>,
    pub model_display: String,
    pub tokens: TokenUsage,
    pub context_percent: f64,
    pub subagents: Vec<SubagentInfo>,
    pub mini_log: Vec<MiniLogEntry>,
    pub pending_approval: Option<String>,
    pub pid: Option<i32>,
    pub started_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum JournalEntryType {
    User,
    Thinking,
    Assistant,
    ToolCall,
    ToolResult,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntry {
    pub session_id: String,
    pub timestamp: String,
    pub entry_type: JournalEntryType,
    pub text: Option<String>,
    pub thinking: Option<String>,
    pub thinking_duration: Option<f64>,
    pub tool: Option<String>,
    pub tool_input: Option<serde_json::Value>,
    pub output: Option<String>,
    pub exit_code: Option<i32>,
    pub lines_changed: Option<LinesChanged>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinesChanged {
    pub added: u32,
    pub removed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    pub file_path: String,
    pub from_version: u32,
    pub to_version: u32,
    pub hunks: Vec<DiffHunk>,
    pub added: u32,
    pub removed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
    pub old_start: u32,
    pub new_start: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlashCommand {
    pub cmd: String,
    pub desc: String,
    pub category: String,
}

/// Map raw model IDs to human-friendly display names.
pub fn model_display_name(model_id: &str) -> &str {
    match model_id {
        "claude-opus-4-6" => "Opus 4.6",
        "claude-sonnet-4-6" => "Sonnet 4.6",
        "claude-sonnet-4-5-20250514" => "Sonnet 4.5",
        "claude-haiku-4-5-20251001" => "Haiku 4.5",
        "claude-opus-4-20250514" => "Opus 4",
        "claude-sonnet-4-20250514" => "Sonnet 4",
        _ => model_id,
    }
}

/// Context window size for a given model ID.
pub fn context_window(model_id: &str) -> u64 {
    match model_id {
        "claude-opus-4-6" => 1_000_000,
        _ => 200_000,
    }
}
