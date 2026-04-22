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
pub struct RateLimitInfo {
    pub status: String,
    pub rate_limit_type: String,
    pub utilization: f64,
    pub resets_at: Option<i64>,
    pub is_using_overage: bool,
    pub surpassed_threshold: f64,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum JournalEntryType {
    User,
    Thinking,
    Assistant,
    ToolCall,
    ToolResult,
    System,
    Progress,
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
    #[serde(default)]
    pub seq: u32,
    #[serde(default)]
    pub epoch: String,
}

impl Default for JournalEntry {
    /// Provides a zero-valued base for struct-update syntax (`..JournalEntry::default()`).
    /// Callers MUST override `entry_type`; `Assistant` here is a placeholder, not a semantic default.
    fn default() -> Self {
        JournalEntry {
            session_id: String::new(),
            timestamp: String::new(),
            entry_type: JournalEntryType::Assistant,
            text: None,
            thinking: None,
            thinking_duration: None,
            tool: None,
            tool_input: None,
            output: None,
            exit_code: None,
            lines_changed: None,
            seq: 0,
            epoch: String::new(),
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskItem {
    pub id: String,
    pub subject: String,
    pub description: String,
    pub active_form: Option<String>,
    pub status: String,
    pub blocks: Vec<String>,
    pub blocked_by: Vec<String>,
}

/// Map raw model IDs to human-friendly display names.
pub fn model_display_name(model_id: &str) -> &str {
    match model_id {
        "claude-opus-4-7" => "Opus 4.7",
        "claude-opus-4-7[1m]" => "Opus 4.7 (1M)",
        "claude-opus-4-6" => "Opus 4.6",
        "claude-opus-4-6[1m]" => "Opus 4.6 (1M)",
        "claude-sonnet-4-6" => "Sonnet 4.6",
        "claude-haiku-4-5-20251001" => "Haiku 4.5",
        _ => model_id,
    }
}

/// Context window size for a given model ID.
pub fn context_window(model_id: &str) -> u64 {
    match model_id {
        "claude-opus-4-7[1m]" | "claude-opus-4-6[1m]" => 1_000_000,
        _ => 200_000,
    }
}

// Session ID type — SQLite AUTOINCREMENT rowid
pub type SessionId = i64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AttentionReason {
    Permission,
    Completed,
    Error,
    RateLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionState {
    pub requires_attention: bool,
    pub reason: Option<AttentionReason>,
    pub since: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_construct_journal_entry_with_default() {
        let entry = JournalEntry {
            session_id: "123".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            entry_type: JournalEntryType::User,
            text: Some("hello".to_string()),
            ..JournalEntry::default()
        };
        assert_eq!(entry.thinking, None);
        assert_eq!(entry.tool, None);
        assert_eq!(entry.exit_code, None);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SessionStatus {
    Initializing,
    Running,
    Waiting,
    Completed,
    Stopped,
    Error,
}

impl SessionStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SessionStatus::Initializing => "initializing",
            SessionStatus::Running => "running",
            SessionStatus::Waiting => "waiting",
            SessionStatus::Completed => "completed",
            SessionStatus::Stopped => "stopped",
            SessionStatus::Error => "error",
        }
    }
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl rusqlite::types::FromSql for SessionStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = String::column_result(value)?;
        Ok(match s.as_str() {
            "initializing" => SessionStatus::Initializing,
            "running" => SessionStatus::Running,
            "waiting" => SessionStatus::Waiting,
            "completed" => SessionStatus::Completed,
            "stopped" => SessionStatus::Stopped,
            "error" => SessionStatus::Error,
            _ => {
                return Err(rusqlite::types::FromSqlError::Other(
                    format!("unknown SessionStatus: {s}").into(),
                ))
            }
        })
    }
}

impl rusqlite::types::ToSql for SessionStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.as_str()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: SessionId,
    pub project_id: Option<i64>,
    pub name: Option<String>,
    pub status: SessionStatus,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub permission_mode: String,
    pub model: Option<String>,
    pub provider: String,
    pub pid: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
    // Runtime fields (not in DB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<TokenUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_approval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mini_log: Option<Vec<MiniLogEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attention: Option<AttentionState>,
    #[serde(default = "default_true")]
    pub skip_permissions: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<SessionId>,
    #[serde(default)]
    pub depth: i32,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone)]
pub struct CreateSessionRequest {
    pub project_path: String,
    pub prompt: String,
    pub model: Option<String>,
    pub permission_mode: String, // "ignore" | "approve"
    pub use_worktree: bool,
    pub session_name: Option<String>,
}
