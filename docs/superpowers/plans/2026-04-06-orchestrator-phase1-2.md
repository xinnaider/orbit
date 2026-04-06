# Agent Dashboard Orchestrator — Phase 1 & 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform agent-dashboard-v2 into an active orchestrator that spawns and controls Claude Code processes via PTY, backed by SQLite persistence, replacing all legacy observer code.

**Architecture:** Tauri v2 + Rust backend with four services: `DatabaseService` (SQLite), `SpawnManager` (portable-pty), `SessionManager` (lifecycle + in-memory state), and refactored `journal_reader` (single-line processor). IPC commands replace polling with Tauri events pushed per output line.

**Tech Stack:** Rust, Tauri v2, `portable-pty 0.8`, `rusqlite 0.31` (bundled), `tokio` (already present), Svelte frontend (mostly unchanged).

---

## File Map

```
MODIFY:
  src-tauri/Cargo.toml
  src-tauri/src/models.rs          — add Session, Project, SessionId, SpawnConfig, CreateSessionRequest
  src-tauri/src/journal_reader.rs  — add process_line(), update JournalState to track status inline
  src-tauri/src/commands.rs        — remove send_keystroke/send_message; update get_journal signature
  src-tauri/src/lib.rs             — replace AppState, wire new services, register new IPC

CREATE:
  src-tauri/src/services/mod.rs
  src-tauri/src/services/database.rs
  src-tauri/src/services/spawn_manager.rs
  src-tauri/src/services/session_manager.rs
  src-tauri/src/ipc/mod.rs
  src-tauri/src/ipc/session.rs
  src-tauri/src/ipc/project.rs
  src/lib/stores/sessions.ts       — replaces agents.ts
  src/components/CreateSessionDialog.svelte

MODIFY (frontend):
  src/lib/tauri.ts
  src/App.svelte
  src/components/Sidebar.svelte    — show sessions list instead of agents
```

---

## Task 1: Add Dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add portable-pty and rusqlite to Cargo.toml**

Replace the `[dependencies]` block with:

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
dirs = "6"
chrono = "0.4"
tokio = { version = "1", features = ["full"] }
ignore = "0.4"
rusqlite = { version = "0.31", features = ["bundled"] }
portable-pty = "0.8"

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.59", features = ["Win32_System_Console", "Win32_Foundation"] }
```

- [ ] **Step 2: Verify compilation**

```bash
cd src-tauri && cargo check 2>&1 | head -30
```

Expected: compiles (new deps download). Ignore warnings about unused code.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: add portable-pty and rusqlite dependencies"
```

---

## Task 2: Extend Models

**Files:**
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: Add new types to the bottom of models.rs**

Append after the existing `context_window` function:

```rust
// Session ID type — SQLite AUTOINCREMENT rowid
pub type SessionId = i64;

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
    pub status: String,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub permission_mode: String,
    pub model: Option<String>,
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
```

- [ ] **Step 2: Verify compilation**

```bash
cd src-tauri && cargo check 2>&1 | head -20
```

Expected: no errors on models.rs.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/models.rs
git commit -m "feat: add Session, Project, SessionStatus types to models"
```

---

## Task 3: DatabaseService

**Files:**
- Create: `src-tauri/src/services/mod.rs`
- Create: `src-tauri/src/services/database.rs`

- [ ] **Step 1: Create services/mod.rs**

```rust
pub mod database;
pub mod spawn_manager;
pub mod session_manager;
```

- [ ] **Step 2: Write failing test for DatabaseService**

Create `src-tauri/src/services/database.rs` with the test first:

```rust
use rusqlite::{Connection, Result as SqlResult, params};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::models::{Project, Session, SessionId};

pub struct DatabaseService {
    conn: Mutex<Connection>,
}

impl DatabaseService {
    pub fn open(path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = DatabaseService { conn: Mutex::new(conn) };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = DatabaseService { conn: Mutex::new(conn) };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS projects (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                name       TEXT NOT NULL,
                path       TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id      INTEGER REFERENCES projects(id),
                name            TEXT,
                status          TEXT NOT NULL DEFAULT 'initializing',
                worktree_path   TEXT,
                branch_name     TEXT,
                permission_mode TEXT NOT NULL DEFAULT 'ignore',
                model           TEXT,
                pid             INTEGER,
                cwd             TEXT,
                created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS session_outputs (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL REFERENCES sessions(id),
                data       TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_session_outputs_session_id
                ON session_outputs(session_id);
        ")?;
        Ok(())
    }

    pub fn create_project(&self, name: &str, path: &str) -> SqlResult<Project> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO projects (name, path) VALUES (?1, ?2)",
            params![name, path],
        )?;
        let id = conn.query_row(
            "SELECT id, name, path, created_at FROM projects WHERE path = ?1",
            params![path],
            |row| Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                created_at: row.get(3)?,
            }),
        )?;
        Ok(id)
    }

    pub fn create_session(
        &self,
        project_id: Option<i64>,
        name: Option<&str>,
        cwd: &str,
        permission_mode: &str,
        model: Option<&str>,
    ) -> SqlResult<SessionId> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (project_id, name, cwd, status, permission_mode, model)
             VALUES (?1, ?2, ?3, 'initializing', ?4, ?5)",
            params![project_id, name, cwd, permission_mode, model],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_session_status(&self, id: SessionId, status: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }

    pub fn update_session_pid(&self, id: SessionId, pid: i32) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET pid = ?1, status = 'running', updated_at = datetime('now') WHERE id = ?2",
            params![pid, id],
        )?;
        Ok(())
    }

    pub fn get_sessions(&self) -> SqlResult<Vec<Session>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, status, worktree_path, branch_name,
                    permission_mode, model, pid, cwd, created_at, updated_at
             FROM sessions ORDER BY created_at DESC"
        )?;
        let sessions = stmt.query_map([], |row| {
            Ok(Session {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                status: row.get(3)?,
                worktree_path: row.get(4)?,
                branch_name: row.get(5)?,
                permission_mode: row.get(6)?,
                model: row.get(7)?,
                pid: row.get(8)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
                cwd: row.get(9)?,
                project_name: None,
                git_branch: None,
                tokens: None,
                context_percent: None,
                pending_approval: None,
                mini_log: None,
            })
        })?.collect::<SqlResult<Vec<_>>>()?;
        Ok(sessions)
    }

    pub fn insert_output(&self, session_id: SessionId, data: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO session_outputs (session_id, data) VALUES (?1, ?2)",
            params![session_id, data],
        )?;
        Ok(())
    }

    pub fn get_outputs(&self, session_id: SessionId) -> SqlResult<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT data FROM session_outputs WHERE session_id = ?1 ORDER BY id ASC"
        )?;
        let rows = stmt.query_map(params![session_id], |row| row.get(0))?
            .collect::<SqlResult<Vec<String>>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrate_creates_tables() {
        let db = DatabaseService::open_in_memory().unwrap();
        // If migration ran without error, tables exist
        let sessions = db.get_sessions().unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_create_project() {
        let db = DatabaseService::open_in_memory().unwrap();
        let p = db.create_project("my-app", "/home/user/my-app").unwrap();
        assert_eq!(p.name, "my-app");
        assert_eq!(p.path, "/home/user/my-app");
        assert!(p.id > 0);
    }

    #[test]
    fn test_create_project_idempotent() {
        let db = DatabaseService::open_in_memory().unwrap();
        let p1 = db.create_project("my-app", "/home/user/my-app").unwrap();
        let p2 = db.create_project("my-app", "/home/user/my-app").unwrap();
        assert_eq!(p1.id, p2.id);
    }

    #[test]
    fn test_create_session() {
        let db = DatabaseService::open_in_memory().unwrap();
        let id = db.create_session(None, Some("task 1"), "/tmp/proj", "ignore", None).unwrap();
        assert!(id > 0);
        let sessions = db.get_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].status, "initializing");
        assert_eq!(sessions[0].cwd, Some("/tmp/proj".to_string()));
    }

    #[test]
    fn test_update_session_status() {
        let db = DatabaseService::open_in_memory().unwrap();
        let id = db.create_session(None, None, "/tmp/proj", "ignore", None).unwrap();
        db.update_session_status(id, "running").unwrap();
        let sessions = db.get_sessions().unwrap();
        assert_eq!(sessions[0].status, "running");
    }

    #[test]
    fn test_insert_and_get_outputs() {
        let db = DatabaseService::open_in_memory().unwrap();
        let id = db.create_session(None, None, "/tmp/proj", "ignore", None).unwrap();
        db.insert_output(id, r#"{"type":"assistant"}"#).unwrap();
        db.insert_output(id, r#"{"type":"user"}"#).unwrap();
        let rows = db.get_outputs(id).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], r#"{"type":"assistant"}"#);
    }
}
```

- [ ] **Step 3: Run the tests**

```bash
cd src-tauri && cargo test services::database 2>&1
```

Expected: all 5 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/services/
git commit -m "feat: add DatabaseService with SQLite schema and tests"
```

---

## Task 4: Refactor journal_reader — add process_line

**Files:**
- Modify: `src-tauri/src/journal_reader.rs`

The existing `parse_journal` stays for `get_subagent_journal` (reads from files in `~/.claude`). We add `process_line` for real-time PTY output processing.

- [ ] **Step 1: Add process_line function and tests at the bottom of journal_reader.rs**

Append to `src-tauri/src/journal_reader.rs`:

```rust
/// Process a single raw JSONL line from PTY stdout and update state.
/// This is the real-time counterpart to parse_journal (which reads files).
pub fn process_line(state: &mut JournalState, line: &str) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return;
    }

    // Skip synthetic messages
    if trimmed.contains("\"<synthetic>\"") {
        return;
    }

    let raw: RawEntry = match serde_json::from_str(trimmed) {
        Ok(r) => r,
        Err(_) => return,
    };

    let ts = raw.timestamp.clone().unwrap_or_default();

    match raw.r#type.as_str() {
        "assistant" => {
            state.last_activity = raw.timestamp.clone();

            let mut has_tool_use = false;
            let mut end_turn = false;

            if let Some(msg) = &raw.message {
                // Extract model
                if let Some(m) = msg.get("model").and_then(|v| v.as_str()) {
                    state.model = Some(m.to_string());
                }

                // Extract token usage
                if let Some(usage) = msg.get("usage") {
                    state.input_tokens = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0)
                        + usage.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0)
                        + usage.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    state.output_tokens = usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    state.cache_read = usage.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    state.cache_write = usage.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                }

                if let Some(stop) = msg.get("stop_reason").and_then(|v| v.as_str()) {
                    if stop == "end_turn" {
                        end_turn = true;
                    }
                }

                // Extract content blocks
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
                                let text = block.get("text")
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
                                let tool_name = block.get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();
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

            // Update status
            if end_turn {
                state.status = AgentStatus::Idle;
                state.pending_approval = None;
            } else if has_tool_use {
                state.status = AgentStatus::Input;
                state.pending_approval = detect_pending_approval(&state.entries);
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
                                let output_text = block.get("content")
                                    .and_then(|v| v.as_str())
                                    .or_else(|| block.get("text").and_then(|v| v.as_str()))
                                    .unwrap_or("")
                                    .to_string();

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
            if let Some(subtype) = raw.message.as_ref()
                .and_then(|m| m.get("subtype"))
                .and_then(|v| v.as_str())
            {
                if subtype == "stop_hook_summary" {
                    state.status = AgentStatus::Idle;
                }
            }
        }

        _ => {}
    }
}

#[cfg(test)]
mod process_line_tests {
    use super::*;

    fn empty_state() -> JournalState {
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

    #[test]
    fn test_process_empty_line_is_noop() {
        let mut state = empty_state();
        process_line(&mut state, "");
        assert!(state.entries.is_empty());
    }

    #[test]
    fn test_process_invalid_json_is_noop() {
        let mut state = empty_state();
        process_line(&mut state, "not json");
        assert!(state.entries.is_empty());
    }

    #[test]
    fn test_process_assistant_text_creates_entry() {
        let mut state = empty_state();
        let line = r#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","content":[{"type":"text","text":"Hello!"}],"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        process_line(&mut state, line);
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].entry_type, JournalEntryType::Assistant);
        assert_eq!(state.entries[0].text.as_deref(), Some("Hello!"));
        assert_eq!(state.model.as_deref(), Some("claude-sonnet-4-6"));
        assert_eq!(state.output_tokens, 5);
    }

    #[test]
    fn test_process_tool_use_sets_input_status() {
        let mut state = empty_state();
        let line = r#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","content":[{"type":"tool_use","name":"Bash","input":{"command":"ls"}}],"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        process_line(&mut state, line);
        assert_eq!(state.status, AgentStatus::Input);
        assert_eq!(state.entries[0].entry_type, JournalEntryType::ToolCall);
    }

    #[test]
    fn test_process_end_turn_sets_idle_status() {
        let mut state = empty_state();
        let line = r#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","stop_reason":"end_turn","content":[{"type":"text","text":"Done."}],"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        process_line(&mut state, line);
        assert_eq!(state.status, AgentStatus::Idle);
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
cd src-tauri && cargo test process_line_tests 2>&1
```

Expected: 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/journal_reader.rs
git commit -m "feat: add process_line to journal_reader for real-time PTY output processing"
```

---

## Task 5: SpawnManager

**Files:**
- Create: `src-tauri/src/services/spawn_manager.rs`

- [ ] **Step 1: Create spawn_manager.rs**

```rust
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

pub struct SpawnConfig {
    pub session_id: crate::models::SessionId,
    pub cwd: PathBuf,
    pub permission_mode: String,   // "ignore" | "approve"
    pub model: Option<String>,
}

pub struct PtyHandle {
    pub pid: u32,
    pub writer: Box<dyn Write + Send>,
    pub reader: Box<dyn std::io::Read + Send>,
}

/// Spawn a Claude Code process via PTY.
/// Returns a PtyHandle with the process PID, a writer (for stdin), and a reader (for stdout).
///
/// The initial prompt is NOT sent here — caller writes it to PtyHandle.writer after spawn.
pub fn spawn_claude(config: SpawnConfig) -> Result<PtyHandle, String> {
    let pty_system = native_pty_system();

    let pair = pty_system.openpty(PtySize {
        rows: 50,
        cols: 220,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e| format!("openpty failed: {e}"))?;

    let mut cmd = CommandBuilder::new("claude");
    cmd.args(["--output-format", "stream-json", "--verbose"]);

    if config.permission_mode == "ignore" {
        cmd.args(["--dangerously-skip-permissions"]);
    }

    if let Some(ref model) = config.model {
        if model != "auto" {
            cmd.args(["--model", model]);
        }
    }

    cmd.cwd(&config.cwd);

    let child = pair.slave.spawn_command(cmd)
        .map_err(|e| format!("spawn failed: {e}"))?;

    // IMPORTANT: drop slave after spawn so reader gets EOF when process exits
    drop(pair.slave);

    let pid = child.process_id().unwrap_or(0);

    let writer = pair.master.take_writer()
        .map_err(|e| format!("take_writer failed: {e}"))?;

    let reader = pair.master.try_clone_reader()
        .map_err(|e| format!("clone_reader failed: {e}"))?;

    // Keep child alive by leaking it — it will be reaped when the process exits.
    // We track lifecycle via PTY EOF instead of explicit child management.
    std::mem::forget(child);

    Ok(PtyHandle { pid, writer, reader })
}
```

- [ ] **Step 2: Verify compilation**

```bash
cd src-tauri && cargo check 2>&1 | grep -E "^error" | head -20
```

Expected: no errors on spawn_manager.rs.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/services/spawn_manager.rs
git commit -m "feat: add SpawnManager — spawn Claude via portable-pty"
```

---

## Task 6: SessionManager

**Files:**
- Create: `src-tauri/src/services/session_manager.rs`

- [ ] **Step 1: Create session_manager.rs**

```rust
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter};

use crate::journal_reader::{JournalState, process_line};
use crate::models::{Session, SessionId, AgentStatus, TokenUsage};
use crate::services::database::DatabaseService;
use crate::services::spawn_manager::{SpawnConfig, PtyHandle, spawn_claude};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionOutputEvent {
    pub session_id: SessionId,
    pub entry: crate::models::JournalEntry,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStateEvent {
    pub session_id: SessionId,
    pub status: String,
    pub tokens: TokenUsage,
    pub context_percent: f64,
    pub pending_approval: Option<String>,
    pub mini_log: Vec<crate::models::MiniLogEntry>,
}

struct ActiveSession {
    session: Session,
    writer: Box<dyn Write + Send>,
}

pub struct SessionManager {
    db: Arc<DatabaseService>,
    active: HashMap<SessionId, ActiveSession>,
    journal_states: HashMap<SessionId, JournalState>,
}

impl SessionManager {
    pub fn new(db: Arc<DatabaseService>) -> Self {
        SessionManager {
            db,
            active: HashMap::new(),
            journal_states: HashMap::new(),
        }
    }

    /// Create a new session: persist to DB, spawn PTY, start reader thread.
    pub fn create_session(
        manager: Arc<Mutex<SessionManager>>,
        app: AppHandle,
        project_path: String,
        prompt: String,
        model: Option<String>,
        permission_mode: String,
        session_name: Option<String>,
    ) -> Result<Session, String> {
        // 1. Resolve project
        let db = {
            let m = manager.lock().unwrap();
            m.db.clone()
        };

        let project_name = std::path::Path::new(&project_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project_path.clone());

        let project = db.create_project(&project_name, &project_path)
            .map_err(|e| e.to_string())?;

        // 2. Create session record
        let session_id = db.create_session(
            Some(project.id),
            session_name.as_deref(),
            &project_path,
            &permission_mode,
            model.as_deref(),
        ).map_err(|e| e.to_string())?;

        // 3. Spawn PTY
        let handle = spawn_claude(SpawnConfig {
            session_id,
            cwd: std::path::PathBuf::from(&project_path),
            permission_mode: permission_mode.clone(),
            model: model.clone(),
        })?;

        let pid = handle.pid as i32;

        // 4. Update DB with PID
        db.update_session_pid(session_id, pid)
            .map_err(|e| e.to_string())?;

        // 5. Build session struct
        let now = chrono::Utc::now().to_rfc3339();
        let session = Session {
            id: session_id,
            project_id: Some(project.id),
            name: session_name,
            status: "running".to_string(),
            worktree_path: None,
            branch_name: None,
            permission_mode: permission_mode.clone(),
            model,
            pid: Some(pid),
            created_at: now.clone(),
            updated_at: now,
            cwd: Some(project_path.clone()),
            project_name: Some(project_name),
            git_branch: None,
            tokens: None,
            context_percent: None,
            pending_approval: None,
            mini_log: None,
        };

        // 6. Register session + writer
        {
            let mut m = manager.lock().unwrap();
            m.active.insert(session_id, ActiveSession {
                session: session.clone(),
                writer: handle.writer,
            });
            m.journal_states.insert(session_id, JournalState {
                entries: Vec::new(),
                input_tokens: 0,
                output_tokens: 0,
                cache_read: 0,
                cache_write: 0,
                model: None,
                last_activity: None,
                status: AgentStatus::Working,
                pending_approval: None,
                mini_log: Vec::new(),
                file_size: 0,
            });
        }

        // 7. Send initial prompt via PTY stdin
        {
            let mut m = manager.lock().unwrap();
            if let Some(active) = m.active.get_mut(&session_id) {
                let _ = write!(active.writer, "{}\n", prompt);
            }
        }

        // 8. Spawn PTY reader thread
        let manager_clone = Arc::clone(&manager);
        let app_clone = app.clone();
        std::thread::spawn(move || {
            Self::pty_reader_loop(manager_clone, app_clone, session_id, handle.reader);
        });

        // 9. Emit session:created
        let _ = app.emit("session:created", &session);

        Ok(session)
    }

    fn pty_reader_loop(
        manager: Arc<Mutex<SessionManager>>,
        app: AppHandle,
        session_id: SessionId,
        reader: Box<dyn std::io::Read + Send>,
    ) {
        let mut reader = std::io::BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF — process exited
                Ok(_) => {
                    let trimmed = line.trim().to_string();
                    if trimmed.is_empty() {
                        continue;
                    }

                    // Update in-memory journal state
                    let (new_entries, state_event) = {
                        let mut m = manager.lock().unwrap();
                        let state = m.journal_states.entry(session_id).or_insert_with(|| JournalState {
                            entries: Vec::new(),
                            input_tokens: 0,
                            output_tokens: 0,
                            cache_read: 0,
                            cache_write: 0,
                            model: None,
                            last_activity: None,
                            status: AgentStatus::Working,
                            pending_approval: None,
                            mini_log: Vec::new(),
                            file_size: 0,
                        });

                        let prev_len = state.entries.len();
                        process_line(state, &trimmed);
                        let new_entries: Vec<_> = state.entries[prev_len..].to_vec();

                        let window = state.model.as_deref()
                            .map(crate::models::context_window)
                            .unwrap_or(200_000);
                        let total = state.input_tokens + state.output_tokens;
                        let context_percent = if window > 0 {
                            (total as f64 / window as f64) * 100.0
                        } else {
                            0.0
                        };

                        let event = SessionStateEvent {
                            session_id,
                            status: format!("{:?}", state.status).to_lowercase(),
                            tokens: TokenUsage {
                                input: state.input_tokens,
                                output: state.output_tokens,
                                cache_read: state.cache_read,
                                cache_write: state.cache_write,
                            },
                            context_percent,
                            pending_approval: state.pending_approval.clone(),
                            mini_log: state.mini_log.clone(),
                        };

                        (new_entries, event)
                    };

                    // Emit new journal entries
                    for entry in new_entries {
                        let mut e = entry.clone();
                        e.session_id = session_id.to_string();
                        let _ = app.emit("session:output", SessionOutputEvent {
                            session_id,
                            entry: e,
                        });
                    }

                    // Emit state update
                    let _ = app.emit("session:state", &state_event);
                }
                Err(_) => break,
            }
        }

        // PTY exited — update status
        {
            let mut m = manager.lock().unwrap();
            if let Some(active) = m.active.get_mut(&session_id) {
                active.session.status = "completed".to_string();
            }
            if let Some(state) = m.journal_states.get_mut(&session_id) {
                state.status = AgentStatus::Idle;
            }
            let db = m.db.clone();
            let _ = db.update_session_status(session_id, "completed");
        }

        let _ = app.emit("session:stopped", serde_json::json!({ "sessionId": session_id }));
    }

    /// Write a message to the session's PTY stdin.
    pub fn send_message(&mut self, session_id: SessionId, text: &str) -> Result<(), String> {
        let active = self.active.get_mut(&session_id)
            .ok_or_else(|| format!("Session {session_id} not active"))?;
        write!(active.writer, "{}\n", text)
            .map_err(|e| e.to_string())
    }

    /// Kill a running session process.
    pub fn stop_session(&mut self, session_id: SessionId) -> Result<(), String> {
        self.active.remove(&session_id);
        let _ = self.db.update_session_status(session_id, "stopped");
        Ok(())
    }

    /// Get all sessions from DB, enriched with in-memory runtime state.
    pub fn get_sessions(&self) -> Vec<Session> {
        let mut sessions = self.db.get_sessions().unwrap_or_default();
        for session in &mut sessions {
            if let Some(state) = self.journal_states.get(&session.id) {
                let window = state.model.as_deref()
                    .map(crate::models::context_window)
                    .unwrap_or(200_000);
                let total = state.input_tokens + state.output_tokens;
                session.tokens = Some(TokenUsage {
                    input: state.input_tokens,
                    output: state.output_tokens,
                    cache_read: state.cache_read,
                    cache_write: state.cache_write,
                });
                session.context_percent = Some(if window > 0 {
                    (total as f64 / window as f64) * 100.0
                } else {
                    0.0
                });
                session.pending_approval = state.pending_approval.clone();
                session.mini_log = Some(state.mini_log.clone());
            }
            if let Some(active) = self.active.get(&session.id) {
                session.status = active.session.status.clone();
            }
        }
        sessions
    }

    pub fn get_journal(&self, session_id: SessionId) -> Vec<crate::models::JournalEntry> {
        match self.journal_states.get(&session_id) {
            Some(state) => state.entries.iter().map(|e| {
                let mut entry = e.clone();
                entry.session_id = session_id.to_string();
                entry
            }).collect(),
            None => vec![],
        }
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
cd src-tauri && cargo check 2>&1 | grep -E "^error" | head -30
```

Expected: no errors. Fix any type mismatches before proceeding.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/services/session_manager.rs
git commit -m "feat: add SessionManager — session lifecycle, PTY reader thread, event emission"
```

---

## Task 7: IPC Handlers

**Files:**
- Create: `src-tauri/src/ipc/mod.rs`
- Create: `src-tauri/src/ipc/session.rs`
- Create: `src-tauri/src/ipc/project.rs`

- [ ] **Step 1: Create ipc/mod.rs**

```rust
pub mod session;
pub mod project;
```

- [ ] **Step 2: Create ipc/session.rs**

```rust
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};

use crate::models::{Session, SessionId, JournalEntry};
use crate::services::session_manager::SessionManager;

pub struct SessionState(pub Arc<Mutex<SessionManager>>);

#[tauri::command]
pub fn create_session(
    project_path: String,
    prompt: String,
    model: Option<String>,
    permission_mode: Option<String>,
    session_name: Option<String>,
    state: State<SessionState>,
    app: AppHandle,
) -> Result<Session, String> {
    let mode = permission_mode.unwrap_or_else(|| "ignore".to_string());
    SessionManager::create_session(
        Arc::clone(&state.0),
        app,
        project_path,
        prompt,
        model,
        mode,
        session_name,
    )
}

#[tauri::command]
pub fn list_sessions(state: State<SessionState>) -> Vec<Session> {
    state.0.lock().unwrap().get_sessions()
}

#[tauri::command]
pub fn stop_session(session_id: SessionId, state: State<SessionState>) -> Result<(), String> {
    state.0.lock().unwrap().stop_session(session_id)
}

#[tauri::command]
pub fn send_session_message(
    session_id: SessionId,
    message: String,
    state: State<SessionState>,
) -> Result<(), String> {
    state.0.lock().unwrap().send_message(session_id, &message)
}

#[tauri::command]
pub fn get_session_journal(session_id: SessionId, state: State<SessionState>) -> Vec<JournalEntry> {
    state.0.lock().unwrap().get_journal(session_id)
}
```

- [ ] **Step 3: Create ipc/project.rs**

```rust
use std::sync::{Arc, Mutex};
use tauri::State;

use crate::models::Project;
use crate::services::session_manager::SessionManager;
use crate::ipc::session::SessionState;

#[tauri::command]
pub fn create_project(
    name: String,
    path: String,
    state: State<SessionState>,
) -> Result<Project, String> {
    state.0.lock().unwrap()
        .db_ref()
        .create_project(&name, &path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_projects(state: State<SessionState>) -> Vec<Project> {
    state.0.lock().unwrap()
        .db_ref()
        .get_projects()
        .unwrap_or_default()
}
```

- [ ] **Step 4: Add `db_ref` and `get_projects` to SessionManager and DatabaseService**

In `session_manager.rs`, add at the bottom of `impl SessionManager`:

```rust
    pub fn db_ref(&self) -> &DatabaseService {
        &self.db
    }
```

In `database.rs`, add the `get_projects` method:

```rust
    pub fn get_projects(&self) -> SqlResult<Vec<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, path, created_at FROM projects ORDER BY name ASC"
        )?;
        let projects = stmt.query_map([], |row| Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            created_at: row.get(3)?,
        }))?.collect::<SqlResult<Vec<_>>>()?;
        Ok(projects)
    }
```

- [ ] **Step 5: Verify compilation**

```bash
cd src-tauri && cargo check 2>&1 | grep -E "^error" | head -30
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/ipc/
git commit -m "feat: add IPC handlers for session and project management"
```

---

## Task 8: Wire lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Replace lib.rs entirely**

```rust
pub mod models;
pub mod journal_reader;
pub mod diff_builder;
pub mod agent_tree;
pub mod commands;
pub mod services;
pub mod ipc;

// Legacy observer modules — KEPT temporarily until frontend migration is complete
pub mod session_watcher;
pub mod keystroke_sender;
pub mod polling;

use std::sync::{Arc, Mutex};
use services::database::DatabaseService;
use services::session_manager::SessionManager;
use ipc::session::SessionState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Resolve app data directory for SQLite DB
            let data_dir = app.path().app_data_dir()
                .expect("Could not resolve app data dir");
            std::fs::create_dir_all(&data_dir)
                .expect("Could not create app data dir");

            let db_path = data_dir.join("agent-dashboard.db");
            let db = Arc::new(
                DatabaseService::open(&db_path)
                    .expect("Could not open database")
            );

            let session_manager = Arc::new(Mutex::new(SessionManager::new(db)));
            app.manage(SessionState(session_manager));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // New orchestrator commands
            ipc::session::create_session,
            ipc::session::list_sessions,
            ipc::session::stop_session,
            ipc::session::send_session_message,
            ipc::session::get_session_journal,
            ipc::project::create_project,
            ipc::project::list_projects,
            // Keep existing read-only commands
            commands::get_diff,
            commands::get_file_versions,
            commands::get_subagent_journal,
            commands::get_slash_commands,
            commands::list_project_files,
            commands::get_tasks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Note: `polling::start_polling` is removed. `send_keystroke`, `send_message`, `get_journal` (old) are removed from the handler.

- [ ] **Step 2: Update commands.rs — remove old journal command**

In `commands.rs`, remove the `use crate::polling::AppState;` import and the `send_keystroke`, `send_message`, and `get_journal` functions. Also remove `use crate::session_watcher;` and `use crate::keystroke_sender;` from the imports.

The file should start with:

```rust
use std::path::Path;

use crate::journal_reader;
use crate::models::*;
use crate::diff_builder;
// ... rest of file unchanged
```

And remove the three functions: `send_keystroke`, `send_message`, `get_journal`.

- [ ] **Step 3: Build the project**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error" | head -40
```

Expected: clean build. If there are errors about unused imports in `polling.rs` or `session_watcher.rs`, add `#[allow(unused)]` at the top of those files temporarily — they'll be deleted in Phase 5.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/commands.rs
git commit -m "feat: wire new services into Tauri app, remove old polling from handler"
```

---

## Task 9: Frontend — Sessions Store and Tauri Bindings

**Files:**
- Create: `src/lib/stores/sessions.ts`
- Modify: `src/lib/tauri.ts`

- [ ] **Step 1: Create src/lib/stores/sessions.ts**

```typescript
import { writable } from 'svelte/store';

export interface TokenUsage {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
}

export interface MiniLogEntry {
  tool: string;
  target: string;
  result: string | null;
  success: boolean | null;
}

export interface Session {
  id: number;
  projectId: number | null;
  name: string | null;
  status: 'initializing' | 'running' | 'waiting' | 'completed' | 'stopped' | 'error';
  permissionMode: string;
  model: string | null;
  pid: number | null;
  cwd: string | null;
  projectName: string | null;
  gitBranch: string | null;
  tokens: TokenUsage | null;
  contextPercent: number | null;
  pendingApproval: string | null;
  miniLog: MiniLogEntry[] | null;
  createdAt: string;
  updatedAt: string;
}

export const sessions = writable<Session[]>([]);
export const selectedSessionId = writable<number | null>(null);

export function getSelectedSession(list: Session[], id: number | null): Session | null {
  if (id === null) return null;
  return list.find(s => s.id === id) ?? null;
}

export function upsertSession(list: Session[], updated: Session): Session[] {
  const idx = list.findIndex(s => s.id === updated.id);
  if (idx === -1) return [updated, ...list];
  const next = [...list];
  next[idx] = { ...next[idx], ...updated };
  return next;
}

export function updateSessionState(
  list: Session[],
  sessionId: number,
  patch: Partial<Session>
): Session[] {
  return list.map(s => s.id === sessionId ? { ...s, ...patch } : s);
}
```

- [ ] **Step 2: Replace src/lib/tauri.ts**

```typescript
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Session, TokenUsage, MiniLogEntry } from './stores/sessions';
import type { JournalEntry, SlashCommand, TaskItem } from './types';

// ── Session IPC ────────────────────────────────────────────────

export interface CreateSessionOptions {
  projectPath: string;
  prompt: string;
  model?: string;
  permissionMode?: 'ignore' | 'approve';
  sessionName?: string;
}

export async function createSession(opts: CreateSessionOptions): Promise<Session> {
  return await invoke('create_session', {
    projectPath: opts.projectPath,
    prompt: opts.prompt,
    model: opts.model ?? null,
    permissionMode: opts.permissionMode ?? 'ignore',
    sessionName: opts.sessionName ?? null,
  });
}

export async function listSessions(): Promise<Session[]> {
  return await invoke('list_sessions');
}

export async function stopSession(sessionId: number): Promise<void> {
  await invoke('stop_session', { sessionId });
}

export async function sendSessionMessage(sessionId: number, message: string): Promise<void> {
  await invoke('send_session_message', { sessionId, message });
}

export async function getSessionJournal(sessionId: number): Promise<JournalEntry[]> {
  return await invoke('get_session_journal', { sessionId });
}

// ── Project IPC ────────────────────────────────────────────────

export async function createProject(name: string, path: string) {
  return await invoke('create_project', { name, path });
}

export async function listProjects() {
  return await invoke('list_projects');
}

// ── Read-only commands (unchanged) ────────────────────────────

export async function getSubagentJournal(sessionId: string, subagentId: string): Promise<JournalEntry[]> {
  return await invoke('get_subagent_journal', { sessionId, subagentId });
}

export async function getSlashCommands(): Promise<SlashCommand[]> {
  return await invoke('get_slash_commands');
}

export async function listProjectFiles(cwd: string): Promise<string[]> {
  return await invoke('list_project_files', { cwd });
}

export async function getSessionTasks(sessionId: string): Promise<TaskItem[]> {
  return await invoke('get_tasks', { sessionId });
}

// ── Event listeners ────────────────────────────────────────────

export interface SessionOutputPayload {
  sessionId: number;
  entry: JournalEntry;
}

export interface SessionStatePayload {
  sessionId: number;
  status: string;
  tokens: TokenUsage;
  contextPercent: number;
  pendingApproval: string | null;
  miniLog: MiniLogEntry[];
}

export function onSessionCreated(cb: (session: Session) => void) {
  return listen<Session>('session:created', e => cb(e.payload));
}

export function onSessionOutput(cb: (payload: SessionOutputPayload) => void) {
  return listen<SessionOutputPayload>('session:output', e => cb(e.payload));
}

export function onSessionState(cb: (payload: SessionStatePayload) => void) {
  return listen<SessionStatePayload>('session:state', e => cb(e.payload));
}

export function onSessionStopped(cb: (sessionId: number) => void) {
  return listen<{ sessionId: number }>('session:stopped', e => cb(e.payload.sessionId));
}
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/stores/sessions.ts src/lib/tauri.ts
git commit -m "feat: add sessions store and new tauri bindings for orchestrator"
```

---

## Task 10: Frontend — App.svelte Migration

**Files:**
- Modify: `src/App.svelte`

- [ ] **Step 1: Replace App.svelte**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import {
    sessions, selectedSessionId, upsertSession, updateSessionState
  } from './lib/stores/sessions';
  import { journal } from './lib/stores/journal';
  import {
    listSessions,
    onSessionCreated,
    onSessionOutput,
    onSessionState,
    onSessionStopped,
  } from './lib/tauri';
  import Sidebar from './components/Sidebar.svelte';
  import CentralPanel from './components/CentralPanel.svelte';
  import RightPanel from './components/RightPanel.svelte';

  let prevStatuses: Record<number, string> = {};
  let audioCtx: AudioContext | null = null;

  function playNotificationBeep() {
    try {
      if (!audioCtx || audioCtx.state === 'closed') {
        audioCtx = new AudioContext();
      }
      const osc = audioCtx.createOscillator();
      const gain = audioCtx.createGain();
      osc.connect(gain);
      gain.connect(audioCtx.destination);
      osc.frequency.value = 800;
      osc.type = 'sine';
      gain.gain.value = 0.3;
      gain.gain.exponentialRampToValueAtTime(0.001, audioCtx.currentTime + 0.2);
      osc.start(audioCtx.currentTime);
      osc.stop(audioCtx.currentTime + 0.2);
    } catch {
      // Audio not available
    }
  }

  onMount(async () => {
    // Load existing sessions on startup
    const existing = await listSessions();
    sessions.set(existing);
    if (existing.length > 0 && !$selectedSessionId) {
      selectedSessionId.set(existing[0].id);
    }

    // session:created — new session spawned
    const unCreated = onSessionCreated((session) => {
      sessions.update(list => upsertSession(list, session));
      if (!$selectedSessionId) selectedSessionId.set(session.id);
    });

    // session:output — new journal entry
    const unOutput = onSessionOutput(({ sessionId, entry }) => {
      journal.update(map => {
        const entries = map.get(sessionId) ?? [];
        return new Map(map).set(sessionId, [...entries, entry]);
      });
    });

    // session:state — status/token update
    const unState = onSessionState((payload) => {
      // Sound on transition to 'input' (needs approval)
      const prev = prevStatuses[payload.sessionId];
      if (payload.status === 'input' && prev && prev !== 'input') {
        playNotificationBeep();
      }
      prevStatuses[payload.sessionId] = payload.status;

      sessions.update(list => updateSessionState(list, payload.sessionId, {
        status: payload.status as any,
        tokens: payload.tokens,
        contextPercent: payload.contextPercent,
        pendingApproval: payload.pendingApproval,
        miniLog: payload.miniLog,
      }));
    });

    // session:stopped
    const unStopped = onSessionStopped((sessionId) => {
      sessions.update(list => updateSessionState(list, sessionId, { status: 'completed' }));
    });

    return () => {
      Promise.all([unCreated, unOutput, unState, unStopped]).then(fns => fns.forEach(fn => fn()));
    };
  });
</script>

<div class="app-layout">
  <Sidebar />
  <CentralPanel />
  <RightPanel />
</div>

<style>
  .app-layout {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }
</style>
```

- [ ] **Step 2: Update journal store to use number keys**

In `src/lib/stores/journal.ts`, change the type to use `number` as key:

```typescript
import { writable } from 'svelte/store';
import type { JournalEntry } from '../types';

export const journal = writable<Map<number, JournalEntry[]>>(new Map());
```

- [ ] **Step 3: Commit**

```bash
git add src/App.svelte src/lib/stores/journal.ts
git commit -m "feat: migrate App.svelte from polling to session events"
```

---

## Task 11: CreateSessionDialog

**Files:**
- Create: `src/components/CreateSessionDialog.svelte`

- [ ] **Step 1: Create CreateSessionDialog.svelte**

```svelte
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { createSession } from '../lib/tauri';
  import type { CreateSessionOptions } from '../lib/tauri';

  const dispatch = createEventDispatcher();

  let projectPath = '';
  let prompt = '';
  let model = 'auto';
  let permissionMode: 'ignore' | 'approve' = 'ignore';
  let sessionName = '';
  let loading = false;
  let error = '';

  const models = [
    { value: 'auto', label: 'Auto (Claude default)' },
    { value: 'claude-sonnet-4-6', label: 'Sonnet 4.6' },
    { value: 'claude-opus-4-6', label: 'Opus 4.6' },
    { value: 'claude-haiku-4-5-20251001', label: 'Haiku 4.5' },
  ];

  async function handleSubmit() {
    if (!projectPath.trim() || !prompt.trim()) {
      error = 'Project path and prompt are required.';
      return;
    }
    loading = true;
    error = '';
    try {
      const opts: CreateSessionOptions = {
        projectPath: projectPath.trim(),
        prompt: prompt.trim(),
        model: model === 'auto' ? undefined : model,
        permissionMode,
        sessionName: sessionName.trim() || undefined,
      };
      await createSession(opts);
      dispatch('created');
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      loading = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') dispatch('cancel');
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="overlay" on:click|self={() => dispatch('cancel')}>
  <div class="dialog">
    <h2>New Session</h2>

    <label>
      Project Path
      <input
        type="text"
        bind:value={projectPath}
        placeholder="/home/user/my-project"
        disabled={loading}
      />
    </label>

    <label>
      Prompt
      <textarea
        bind:value={prompt}
        placeholder="What should Claude work on?"
        rows="4"
        disabled={loading}
      />
    </label>

    <label>
      Session Name (optional)
      <input
        type="text"
        bind:value={sessionName}
        placeholder="e.g. Fix auth bug"
        disabled={loading}
      />
    </label>

    <label>
      Model
      <select bind:value={model} disabled={loading}>
        {#each models as m}
          <option value={m.value}>{m.label}</option>
        {/each}
      </select>
    </label>

    <label class="permission-row">
      <span>Approve tool calls</span>
      <input
        type="checkbox"
        checked={permissionMode === 'approve'}
        on:change={e => permissionMode = e.currentTarget.checked ? 'approve' : 'ignore'}
        disabled={loading}
      />
    </label>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <div class="actions">
      <button on:click={() => dispatch('cancel')} disabled={loading}>Cancel</button>
      <button class="primary" on:click={handleSubmit} disabled={loading}>
        {loading ? 'Starting…' : 'Start Session'}
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.6);
    display: flex; align-items: center; justify-content: center;
    z-index: 100;
  }
  .dialog {
    background: var(--bg-surface, #1e1e1e);
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    padding: 24px;
    width: 480px;
    max-width: 90vw;
    display: flex; flex-direction: column; gap: 14px;
  }
  h2 { margin: 0; font-size: 1rem; font-weight: 600; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 0.8rem; color: #aaa; }
  input, textarea, select {
    background: var(--bg-input, #2a2a2a);
    border: 1px solid var(--border, #333);
    border-radius: 4px;
    color: inherit;
    font-size: 0.85rem;
    padding: 6px 8px;
  }
  .permission-row { flex-direction: row; align-items: center; justify-content: space-between; }
  .error { color: #f87171; font-size: 0.8rem; margin: 0; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 4px; }
  button { padding: 6px 16px; border-radius: 4px; border: 1px solid #444; background: #2a2a2a; color: inherit; cursor: pointer; }
  button.primary { background: #3b82f6; border-color: #3b82f6; color: white; }
  button:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
```

- [ ] **Step 2: Add "New Session" button to Sidebar**

In `src/components/Sidebar.svelte`, import and wire the dialog. Find where the sidebar header or agent list is rendered and add:

```svelte
<script lang="ts">
  // Add to existing imports:
  import CreateSessionDialog from './CreateSessionDialog.svelte';
  let showCreateDialog = false;
</script>

<!-- Add a button at the top of the sidebar: -->
<button class="new-session-btn" on:click={() => showCreateDialog = true}>+ New Session</button>

{#if showCreateDialog}
  <CreateSessionDialog
    on:created={() => showCreateDialog = false}
    on:cancel={() => showCreateDialog = false}
  />
{/if}
```

- [ ] **Step 3: Test the full flow manually**

```bash
npm run tauri dev
```

1. Click "New Session"
2. Enter a real project path and a prompt
3. Verify the session appears in the sidebar
4. Verify journal entries stream in as Claude works
5. Verify status updates (working → idle)

- [ ] **Step 4: Commit**

```bash
git add src/components/CreateSessionDialog.svelte src/components/Sidebar.svelte
git commit -m "feat: add CreateSessionDialog and wire New Session button in sidebar"
```

---

## Task 12 (Phase 2): Persist session_outputs to DB

This ensures journal history survives app restarts.

**Files:**
- Modify: `src-tauri/src/services/session_manager.rs`

- [ ] **Step 1: Add DB persistence in pty_reader_loop**

In the `pty_reader_loop` function, after acquiring the lock and calling `process_line`, add a DB write. Find the block:

```rust
let prev_len = state.entries.len();
process_line(state, &trimmed);
```

And after it, add:

```rust
// Persist raw line to DB
let db = m.db.clone();
drop(m); // release lock before DB write
let _ = db.insert_output(session_id, &trimmed);
```

Then re-acquire the lock for the state event:

```rust
let m = manager.lock().unwrap();
// ... build state_event from m.journal_states ...
```

Refactor the block to:

```rust
// Update in-memory state
let (new_entries, state_event, db) = {
    let mut m = manager.lock().unwrap();
    let state = m.journal_states.entry(session_id)
        .or_insert_with(JournalState::default);

    let prev_len = state.entries.len();
    process_line(state, &trimmed);
    let new_entries: Vec<_> = state.entries[prev_len..].to_vec();

    let window = state.model.as_deref()
        .map(crate::models::context_window)
        .unwrap_or(200_000);
    let total = state.input_tokens + state.output_tokens;

    let event = SessionStateEvent {
        session_id,
        status: format!("{:?}", state.status).to_lowercase(),
        tokens: TokenUsage {
            input: state.input_tokens,
            output: state.output_tokens,
            cache_read: state.cache_read,
            cache_write: state.cache_write,
        },
        context_percent: if window > 0 { (total as f64 / window as f64) * 100.0 } else { 0.0 },
        pending_approval: state.pending_approval.clone(),
        mini_log: state.mini_log.clone(),
    };

    (new_entries, event, m.db.clone())
};

// Persist outside of lock
let _ = db.insert_output(session_id, &trimmed);
```

Also add `Default` to `JournalState` in journal_reader.rs:

```rust
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
```

- [ ] **Step 2: Build and verify**

```bash
cd src-tauri && cargo build 2>&1 | grep "^error" | head -20
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/services/session_manager.rs src-tauri/src/journal_reader.rs
git commit -m "feat: persist session output lines to SQLite in real-time"
```

---

## Task 13 (Phase 2): Restore journal from DB on app startup

**Files:**
- Modify: `src-tauri/src/services/session_manager.rs`

- [ ] **Step 1: Add restore_from_db method to SessionManager**

```rust
    /// Load journal states for all existing sessions from DB on startup.
    /// Called once from lib.rs setup after SessionManager is created.
    pub fn restore_from_db(&mut self) {
        let sessions = match self.db.get_sessions() {
            Ok(s) => s,
            Err(_) => return,
        };

        for session in sessions {
            if self.journal_states.contains_key(&session.id) {
                continue; // already loaded (active session)
            }

            let rows = match self.db.get_outputs(session.id) {
                Ok(r) => r,
                Err(_) => continue,
            };

            if rows.is_empty() {
                continue;
            }

            let mut state = JournalState::default();
            for line in &rows {
                process_line(&mut state, line);
            }
            state.file_size = rows.len() as u64;
            self.journal_states.insert(session.id, state);
        }
    }
```

- [ ] **Step 2: Call restore_from_db from lib.rs setup**

In `lib.rs`, after creating the `SessionManager`, add:

```rust
let session_manager = {
    let mut sm = SessionManager::new(db);
    sm.restore_from_db();
    Arc::new(Mutex::new(sm))
};
```

- [ ] **Step 3: Build and verify**

```bash
cd src-tauri && cargo build 2>&1 | grep "^error" | head -20
```

- [ ] **Step 4: Manual test — journal history after restart**

```bash
npm run tauri dev
```

1. Create a session, let Claude run a few messages
2. Close the app
3. Reopen — click on the session in sidebar
4. Verify journal entries are restored

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/session_manager.rs src-tauri/src/lib.rs
git commit -m "feat: restore session journal from DB on app startup"
```

---

## Phases 3–5 (Future Plans)

The following are **not included** in this plan. Each warrants its own plan when ready:

- **Phase 3 — Worktrees:** `WorktreeManager` (`git worktree add/remove`), toggle in `CreateSessionDialog`
- **Phase 4 — Permission MCP:** `PermissionManager`, MCP stdio server per session, `permission:request` event flow
- **Phase 5 — Cleanup:** Delete `session_watcher.rs`, `polling.rs`, `keystroke_sender.rs`, `src/bin/send_keys.rs`

---

## Self-Review Notes

- All Tauri commands use `State<SessionState>` — consistent throughout
- `SessionId = i64` used in Rust; frontend uses `number` — Tauri serializes i64 as JSON number ✓
- `get_subagent_journal` still takes `session_id: String` (Claude session UUID) — this is intentional, subagents still read from `~/.claude` ✓
- `JournalState.file_size` is repurposed in `restore_from_db` as row count — harmless, field is only used by legacy `parse_journal` ✓
- `portable-pty` child is forgotten after spawn — PTY EOF is the lifecycle signal ✓
- `polling.rs` and `session_watcher.rs` remain compiled but unused until Phase 5 — no functional impact ✓
