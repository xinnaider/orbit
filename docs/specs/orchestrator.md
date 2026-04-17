# Agent Dashboard — Orchestrator Redesign Spec

## Goal

Transform agent-dashboard-v2 from a passive observer (reads external Claude processes) into an active orchestrator (spawns and controls Claude processes). The app gains full lifecycle control over sessions: create, run, monitor, stop.

## Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Backend stack | Tauri + Rust (keep) | Performance; avoids migration cost |
| External session support | None — only orchestrated sessions | Enables deprecating send_keys entirely |
| Persistence | SQLite via `rusqlite` | Survives restarts; enables history |
| Git worktrees | Optional per session | Isolation without forcing it |
| Permission mode | `ignore` default; `approve` via MCP optional per session | Flexibility without overhead |

## Architecture

### Service Layer (new)

```
src-tauri/src/
  services/
    session_manager.rs    -- lifecycle, state, event emission
    spawn_manager.rs      -- portable-pty: spawn, write, kill
    worktree_manager.rs   -- git worktree add/remove
    permission_manager.rs -- MCP server per session (optional)
    database.rs           -- SQLite connection + queries
  ipc/
    session.rs            -- create_session, stop_session, send_message, list_sessions
    project.rs            -- project CRUD
```

### Code Reuse Map

**Keep unchanged:**
- `models.rs` — all types reused; add `Session` and `Project` structs
- `journal_reader.rs` — parser logic unchanged; data source changes from file to DB rows
- `diff_builder.rs` — reads `~/.claude/file-history`, unaffected by spawn change
- `agent_tree.rs` — reads `~/.claude/projects/` subagent dirs, unaffected
- `commands.rs` (partial) — keep: `get_journal`, `get_diff`, `get_file_versions`, `get_subagent_journal`, `get_slash_commands`, `get_tasks`

**Delete:**
- `session_watcher.rs`
- `polling.rs`
- `keystroke_sender.rs`
- `src-tauri/src/bin/send_keys.rs`

**New:**
- All files under `services/` and `ipc/` listed above

## Database Schema

```sql
CREATE TABLE projects (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    path       TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE sessions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id      INTEGER REFERENCES projects(id),
    name            TEXT,
    status          TEXT NOT NULL DEFAULT 'initializing',
    -- 'initializing' | 'running' | 'waiting' | 'completed' | 'stopped' | 'error'
    worktree_path   TEXT,
    branch_name     TEXT,
    permission_mode TEXT NOT NULL DEFAULT 'ignore',
    model           TEXT,
    pid             INTEGER,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE session_outputs (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES sessions(id),
    data       TEXT NOT NULL,  -- raw JSONL line from PTY stdout
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_session_outputs_session_id ON session_outputs(session_id);
```

`session_outputs` stores raw JSONL lines. `journal_reader::parse_journal` will be refactored to accept an iterator of `&str` lines instead of a file path. The DB adapter fetches rows for a `session_id` and yields each `data` field as a line. The parsing logic itself is unchanged.

## Service Interfaces

### SpawnManager

```rust
// SessionId = i64 (SQLite AUTOINCREMENT rowid)
pub type SessionId = i64;

pub struct SpawnConfig {
    pub session_id: SessionId,
    pub cwd: PathBuf,
    pub prompt: String,
    pub model: Option<String>,
    pub permission_mode: PermissionMode,
    pub worktree_path: Option<PathBuf>,
    pub mcp_socket_path: Option<PathBuf>,
}

impl SpawnManager {
    pub fn spawn(&mut self, config: SpawnConfig) -> Result<Pid>
    pub fn write(&self, session_id: &SessionId, input: &str) -> Result<()>
    pub fn kill(&mut self, session_id: &SessionId) -> Result<()>
    pub fn is_running(&self, session_id: &SessionId) -> bool
}
```

Spawns `claude --output-format stream-json --verbose [--dangerously-skip-permissions | --mcp-config <path>] [--model <model>]` via `portable-pty`. Each stdout line is sent to `SessionManager` via an async channel.

> Note: exact MCP flag (`--mcp-config` vs `--mcp-server`) to be confirmed against Claude Code CLI version during Phase 4 implementation. Crystal uses a JSON config file written to a temp path.

### SessionManager

```rust
impl SessionManager {
    pub async fn create_session(&mut self, req: CreateSessionRequest) -> Result<Session>
    pub async fn stop_session(&mut self, session_id: &SessionId) -> Result<()>
    pub fn send_message(&self, session_id: &SessionId, text: &str) -> Result<()>
    pub fn get_sessions(&self) -> Vec<Session>
    pub fn get_journal_state(&self, session_id: &SessionId) -> Option<&JournalState>
    fn on_pty_output(&mut self, session_id: &SessionId, line: &str)  // internal
}
```

`on_pty_output` is called for every PTY stdout line:
1. Persists line to `session_outputs` in DB
2. Feeds line to `journal_reader` to update in-memory `JournalState`
3. Emits `session:output` Tauri event with parsed `JournalEntry`
4. Emits `session:state` Tauri event with updated status/tokens

### WorktreeManager

```rust
impl WorktreeManager {
    pub fn create(&self, project_path: &Path, branch: &str) -> Result<PathBuf>
    pub fn remove(&self, worktree_path: &Path) -> Result<()>
}
```

Wraps `git worktree add <path> -b <branch>` and `git worktree remove <path>`. Called by `SessionManager` before spawn when `use_worktree: true`.

### PermissionManager

```rust
impl PermissionManager {
    pub fn start_mcp_server(&self, session_id: &SessionId) -> Result<McpServerHandle>
    pub fn stop_mcp_server(&self, session_id: &SessionId)
    pub fn respond(&self, request_id: &str, approved: bool) -> Result<()>
}
```

When active, spawns a stdio MCP server process per session. Claude is launched with `--mcp-server <socket_path>`. When Claude requests a tool permission, the MCP server emits `permission:request` event to the frontend. The user responds via the existing approval banner UI, which calls `permission:respond` IPC.

## Data Flow

### Session Creation

```
Frontend → create_session(project_id, prompt, model, use_worktree, permission_mode)
  → SessionManager.create_session()
      → DB: INSERT session (status = "initializing")
      → [if use_worktree]  WorktreeManager.create()
      → [if approve mode]  PermissionManager.start_mcp_server()
      → SpawnManager.spawn(config)
      → DB: UPDATE session (status = "running", pid)
      → emit "session:created"
```

### Real-time Output

```
PTY stdout line
  → SpawnManager channel → SessionManager.on_pty_output()
      → DB: INSERT session_outputs
      → journal_reader::parse_line() → update JournalState (in-memory)
      → emit "session:output" { session_id, entry: JournalEntry }
      → emit "session:state"  { session_id, status, tokens, context_percent }
```

### Send Message

```
Frontend → send_message(session_id, text)
  → SessionManager.send_message()
      → SpawnManager.write(session_id, text + "\n")
      → DB: INSERT session_outputs (user input line)
      → emit "session:state" { status: "working" }
```

## Tauri Events

| Event | Payload | When |
|---|---|---|
| `session:created` | `Session` | Session created successfully |
| `session:output` | `{ session_id, entry: JournalEntry }` | Each JSONL line processed |
| `session:state` | `{ session_id, status, tokens, context_percent }` | Status changes |
| `session:stopped` | `{ session_id }` | Process exits |
| `permission:request` | `{ request_id, session_id, tool, input }` | Claude requests tool approval |

## Frontend Changes

**Remove:**
- `agents-update` polling listener
- `send_keystroke` / `send_message` via keystroke injection
- `session_watcher`-dependent logic

**Add:**
- Event listeners for all events listed above
- `sessions` store (replaces `agents` store) — session metadata from DB
- `journal` store — `Map<session_id, JournalEntry[]>` accumulated in memory
- `CreateSessionDialog` — fields: project path, prompt, model, worktree toggle, permission mode toggle

**Unchanged:**
- `JournalEntry` rendering components (`JournalEntry.svelte`, `ToolCallEntry.svelte`, `ThinkingBlock.svelte`)
- `DiffView.svelte`
- `TasksProgress.svelte`
- `StatsPanel.svelte`
- `SubagentsPanel.svelte`
- Approval banner (already works event-driven)

## Migration Phases

### Phase 1 — Core Orchestration
1. Add `rusqlite` dependency + `DatabaseService` + schema
2. `SpawnManager` with `portable-pty`
3. `SessionManager` (create, stop, on_pty_output)
4. IPC handlers: `create_session`, `stop_session`, `send_message`, `list_sessions`
5. Frontend: replace polling with event listeners, update stores

*Milestone: app creates and controls Claude sessions.*

### Phase 2 — Full Persistence
6. `session_outputs` DB writes in `on_pty_output`
7. `journal_reader` adapter to read from DB rows
8. App startup: reconnect to sessions with live PIDs

*Milestone: history survives app restarts.*

### Phase 3 — Worktrees
9. `WorktreeManager`
10. `CreateSessionDialog` worktree toggle

*Milestone: optional per-session git isolation.*

### Phase 4 — Permission MCP
11. `PermissionManager` + MCP server process
12. `CreateSessionDialog` permission mode toggle

*Milestone: granular tool approval per session.*

### Phase 5 — Cleanup
13. Delete `session_watcher.rs`, `polling.rs`, `keystroke_sender.rs`, `bin/send_keys.rs`
14. Remove unused Tauri commands (`send_keystroke`)

*Milestone: no legacy observer code remains.*

## Dependencies to Add (Cargo.toml)

```toml
rusqlite = { version = "0.31", features = ["bundled"] }
portable-pty = "0.8"
tokio = { version = "1", features = ["full"] }  # if not already present
```
