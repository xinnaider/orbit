# SSH Remote Sessions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow Orbit to spawn Claude Code sessions on remote Linux/Ubuntu servers via SSH, with live journal streaming and follow-up messages — exactly like local sessions.

**Architecture:** Add an optional `ssh_host` + `ssh_user` to Session. When both are set, `spawn_manager` runs `ssh user@host bash -lc 'claude ...'` instead of a local process. The stdout/stderr pipe and entire `reader_loop` are unchanged — stream-json flows through SSH transparently. Phase 1 covers journal + chat only; file diff, subagent viewer, and directory browser remain local-only.

**Tech Stack:** Rust 1.85, rusqlite 0.31, Tauri 2, SvelteKit 2 + Svelte 5 + TypeScript 5.6, Vitest 2, cargo test

**Branch:** `feat/ssh-sessions`

---

## File Map

| Action | File | Change |
|--------|------|--------|
| Modify | `front/src/models.rs` | Add `ssh_host`, `ssh_user` to `Session` |
| Modify | `front/src/services/database.rs` | DB migration + CRUD for SSH fields |
| Modify | `front/src/services/spawn_manager.rs` | `SpawnMode` enum + SSH spawn logic |
| Modify | `front/src/services/session_manager.rs` | Pass SSH config through `do_spawn` |
| Modify | `front/src/ipc/session.rs` | Accept `ssh_host`, `ssh_user` in `create_session` |
| Modify | `api/lib/stores/sessions.ts` | Add `sshHost`, `sshUser` to `Session` interface |
| Modify | `api/lib/tauri.ts` | Add SSH fields to `CreateSessionOptions` |
| Modify | `api/components/NewSessionModal.svelte` | SSH toggle + host/user fields |
| Modify | `api/lib/mock/tauri-mock.ts` | Add `sshHost: null`, `sshUser: null` to mock sessions |
| Modify | `api/lib/stores/sessions.test.ts` | Add SSH fields to `makeSession()` factory |

---

## Task 1: Add SSH fields to Session model and DB

**Files:**
- Modify: `front/src/models.rs`
- Modify: `front/src/services/database.rs`

- [ ] **Step 1: Add `ssh_host` and `ssh_user` to the `Session` struct in `models.rs`**

In `front/src/models.rs`, find the `Session` struct (around line 222). Add two fields after `mini_log`:

```rust
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
}
```

- [ ] **Step 2: Add DB migration for SSH columns in `database.rs`**

In `front/src/services/database.rs`, in the `migrate` method (around line 30), add two new `ALTER TABLE` statements alongside the existing ones:

```rust
fn migrate(&self) -> SqlResult<()> {
    let conn = self.conn.lock().unwrap();
    let _ = conn.execute_batch("ALTER TABLE sessions ADD COLUMN claude_session_id TEXT");
    let _ = conn.execute_batch("ALTER TABLE sessions ADD COLUMN cwd TEXT");
    let _ = conn.execute_batch("ALTER TABLE sessions ADD COLUMN ssh_host TEXT");
    let _ = conn.execute_batch("ALTER TABLE sessions ADD COLUMN ssh_user TEXT");
    // ... rest of migrate unchanged
```

- [ ] **Step 3: Update `create_session` in `database.rs` to accept SSH fields**

Replace the existing `create_session` method signature and body:

```rust
pub fn create_session(
    &self,
    project_id: Option<i64>,
    name: Option<&str>,
    cwd: &str,
    permission_mode: &str,
    model: Option<&str>,
    ssh_host: Option<&str>,
    ssh_user: Option<&str>,
) -> SqlResult<SessionId> {
    let conn = self.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO sessions (project_id, name, cwd, status, permission_mode, model, ssh_host, ssh_user)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            project_id,
            name,
            cwd,
            crate::models::SessionStatus::Initializing.as_str(),
            permission_mode,
            model,
            ssh_host,
            ssh_user,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}
```

- [ ] **Step 4: Update `get_sessions` and `get_session` in `database.rs` to read SSH fields**

Replace the SELECT query and row mapping in both `get_sessions` and `get_session`. Add `ssh_host, ssh_user` to the SELECT and map them at indices 12 and 13:

```rust
// get_sessions — replace prepare statement:
let mut stmt = conn.prepare(
    "SELECT id, project_id, name, status, worktree_path, branch_name,
            permission_mode, model, pid, cwd, created_at, updated_at,
            ssh_host, ssh_user
     FROM sessions ORDER BY created_at DESC",
)?;

// Row mapping (same for both get_sessions and get_session):
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
    ssh_host: row.get(12)?,
    ssh_user: row.get(13)?,
})
```

Apply the same mapping update to `get_session` — it uses the same column order.

- [ ] **Step 5: Fix all existing `db.create_session` call sites to pass the two new `None` args**

Run:

```bash
grep -rn "create_session(" front/src/
```

Every call in `session_manager.rs` and in `database.rs` tests needs two extra `None` args. Example — in `session_manager.rs` (around line 70):

```rust
let session_id = self
    .db
    .create_session(
        Some(project.id),
        session_name,
        project_path,
        permission_mode,
        model,
        None, // ssh_host — set by IPC layer for SSH sessions
        None, // ssh_user
    )
    .map_err(|e| e.to_string())?;
```

And in `database.rs` tests — every `create_session(` call:

```rust
// Before: db.create_session(None, None, "/tmp/proj", "ignore", None)
// After:
db.create_session(None, None, "/tmp/proj", "ignore", None, None, None)
```

There are ~10 call sites in the tests — update all of them.

- [ ] **Step 6: Write failing Rust tests for SSH fields**

Add to the `#[cfg(test)]` block in `database.rs`:

```rust
#[test]
fn test_create_session_with_ssh() {
    let db = DatabaseService::open_in_memory().unwrap();
    let id = db
        .create_session(
            None,
            Some("remote-task"),
            "/home/user/project",
            "ignore",
            None,
            Some("vps.example.com"),
            Some("ubuntu"),
        )
        .unwrap();
    let session = db.get_session(id).unwrap().unwrap();
    assert_eq!(session.ssh_host, Some("vps.example.com".to_string()));
    assert_eq!(session.ssh_user, Some("ubuntu".to_string()));
    assert_eq!(session.cwd, Some("/home/user/project".to_string()));
}

#[test]
fn test_create_session_local_has_no_ssh() {
    let db = DatabaseService::open_in_memory().unwrap();
    let id = db
        .create_session(None, None, "/tmp/proj", "ignore", None, None, None)
        .unwrap();
    let session = db.get_session(id).unwrap().unwrap();
    assert_eq!(session.ssh_host, None);
    assert_eq!(session.ssh_user, None);
}
```

- [ ] **Step 7: Run Rust tests — verify they pass**

```bash
cd front && cargo test 2>&1 | tail -20
```

Expected: all tests pass, including `test_create_session_with_ssh` and `test_create_session_local_has_no_ssh`.

- [ ] **Step 8: Commit**

```bash
cd /c/Users/fernandonepen/Documents/agent-dashboard-v2
git add front/src/models.rs front/src/services/database.rs front/src/services/session_manager.rs
git commit -m "feat(ssh): add ssh_host/ssh_user to Session model and DB"
```

---

## Task 2: SSH spawn mode in `spawn_manager.rs`

**Files:**
- Modify: `front/src/services/spawn_manager.rs`

- [ ] **Step 1: Write failing tests for the new types and `posix_escape`**

Add to the `#[cfg(test)]` block in `spawn_manager.rs`:

```rust
#[test]
fn test_posix_escape_simple() {
    assert_eq!(posix_escape("hello world"), "'hello world'");
}

#[test]
fn test_posix_escape_with_single_quote() {
    // "it's a test" → 'it'\''s a test'
    assert_eq!(posix_escape("it's a test"), "'it'\\''s a test'");
}

#[test]
fn test_posix_escape_empty() {
    assert_eq!(posix_escape(""), "''");
}

#[test]
fn test_spawn_local_config_no_panic() {
    let config = SpawnConfig {
        session_id: 0,
        cwd: std::env::temp_dir(),
        remote_cwd: None,
        permission_mode: "ignore".to_string(),
        model: None,
        prompt: "test".to_string(),
        claude_session_id: None,
        spawn_mode: SpawnMode::Local,
    };
    let _ = spawn_claude(config);
}
```

- [ ] **Step 2: Run tests to verify they fail (compile error expected)**

```bash
cd front && cargo test test_posix_escape 2>&1 | head -20
```

Expected: compile error — `posix_escape`, `SpawnMode`, `remote_cwd` not defined yet.

- [ ] **Step 3: Add `SpawnMode` enum, update `SpawnConfig`, add `posix_escape`**

At the top of `spawn_manager.rs`, before `SpawnConfig`, add:

```rust
#[derive(Debug, Clone)]
pub enum SpawnMode {
    Local,
    Ssh { host: String, user: String },
}

pub struct SpawnConfig {
    pub session_id: crate::models::SessionId,
    pub cwd: std::path::PathBuf,     // used only for Local
    pub remote_cwd: Option<String>,  // used only for Ssh
    pub permission_mode: String,
    pub model: Option<String>,
    pub prompt: String,
    pub claude_session_id: Option<String>,
    pub spawn_mode: SpawnMode,
}

/// Wrap a string in POSIX single quotes, escaping embedded single quotes as '\''.
fn posix_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
```

- [ ] **Step 4: Rename current `spawn_claude` to `spawn_claude_local` and add SSH variant**

Rename the existing function to `spawn_claude_local(config: SpawnConfig) -> Result<SpawnHandle, String>` (private, no `pub`).

Then add the SSH variant immediately after:

```rust
fn spawn_claude_ssh(config: SpawnConfig, host: &str, user: &str) -> Result<SpawnHandle, String> {
    let remote_cwd = config.remote_cwd.as_deref().unwrap_or("/tmp").to_string();

    let mut claude_args = String::from(
        "claude --output-format stream-json --verbose --dangerously-skip-permissions",
    );

    if let Some(ref model) = config.model {
        if model != "auto" {
            claude_args.push_str(&format!(" --model {}", posix_escape(model)));
        }
    }

    if let Some(ref resume_id) = config.claude_session_id {
        claude_args.push_str(&format!(" --resume {}", posix_escape(resume_id)));
    }

    claude_args.push_str(&format!(" -p {}", posix_escape(&config.prompt)));

    // cd to remote dir first; failure surfaces via stderr → session:error
    let script = format!("cd {} && {}", posix_escape(&remote_cwd), claude_args);

    let mut cmd = std::process::Command::new("ssh");
    cmd.args([
        "-o", "BatchMode=yes",
        "-o", "ConnectTimeout=15",
        "-o", "StrictHostKeyChecking=accept-new",
        &format!("{}@{}", user, host),
        "bash", "-lc", &script,
    ]);

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd.spawn().map_err(|e| format!("ssh spawn failed: {e}"))?;

    let pid = child.id();
    let stdout = child.stdout.take().ok_or_else(|| "no stdout".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "no stderr".to_string())?;

    std::mem::forget(child);

    Ok(SpawnHandle {
        pid,
        reader: Box::new(stdout),
        stderr: Box::new(stderr),
    })
}
```

Then replace the public `spawn_claude` with a dispatcher:

```rust
pub fn spawn_claude(config: SpawnConfig) -> Result<SpawnHandle, String> {
    match &config.spawn_mode {
        SpawnMode::Local => spawn_claude_local(config),
        SpawnMode::Ssh { host, user } => {
            let host = host.clone();
            let user = user.clone();
            spawn_claude_ssh(config, &host, &user)
        }
    }
}
```

- [ ] **Step 5: Fix the existing test `test_spawn_bad_path_returns_error` — add new fields**

```rust
#[test]
fn test_spawn_bad_path_returns_error() {
    let result = spawn_claude(SpawnConfig {
        session_id: 0,
        cwd: std::env::temp_dir(),
        remote_cwd: None,
        permission_mode: "ignore".to_string(),
        model: None,
        prompt: "test".to_string(),
        claude_session_id: None,
        spawn_mode: SpawnMode::Local,
    });
    if let Err(e) = result {
        assert!(!e.is_empty());
    }
}
```

- [ ] **Step 6: Run tests — verify all pass**

```bash
cd front && cargo test 2>&1 | tail -20
```

Expected: `test_posix_escape_simple`, `test_posix_escape_with_single_quote`, `test_posix_escape_empty`, `test_spawn_local_config_no_panic`, `test_spawn_bad_path_returns_error` all pass.

- [ ] **Step 7: Commit**

```bash
cd /c/Users/fernandonepen/Documents/agent-dashboard-v2
git add front/src/services/spawn_manager.rs
git commit -m "feat(ssh): add SpawnMode enum and SSH spawn via ssh + bash -lc"
```

---

## Task 3: Wire SSH config through `session_manager.rs` and `ipc/session.rs`

**Files:**
- Modify: `front/src/services/session_manager.rs`
- Modify: `front/src/ipc/session.rs`

- [ ] **Step 1: Update `init_session` signature to accept SSH params**

Change the method signature at line ~53 of `session_manager.rs`:

```rust
pub fn init_session(
    &mut self,
    project_path: &str,
    session_name: Option<&str>,
    permission_mode: &str,
    model: Option<&str>,
    ssh_host: Option<&str>,
    ssh_user: Option<&str>,
) -> Result<Session, String> {
```

Update the `db.create_session` call inside the method:

```rust
let session_id = self
    .db
    .create_session(
        Some(project.id),
        session_name,
        project_path,
        permission_mode,
        model,
        ssh_host,
        ssh_user,
    )
    .map_err(|e| e.to_string())?;
```

Update the `Session` construction to populate the new fields:

```rust
let session = Session {
    // ... all existing fields unchanged ...
    mini_log: None,
    ssh_host: ssh_host.map(|s| s.to_string()),
    ssh_user: ssh_user.map(|s| s.to_string()),
};
```

- [ ] **Step 2: Update `do_spawn` to derive `SpawnMode` from session**

In `do_spawn`, find the tuple destructure where `cwd`, `permission_mode` etc. are extracted from `m.active`. Extend it to also extract `ssh_host` and `ssh_user`:

```rust
let (db, cwd, permission_mode, model, claude_session_id, ssh_host, ssh_user) = {
    let m = manager.lock().unwrap();
    let a = match m.active.get(&session_id) {
        Some(a) => a,
        None => {
            let _ = app.emit(
                "session:error",
                serde_json::json!({
                    "sessionId": session_id,
                    "error": "Session not found in active map"
                }),
            );
            return;
        }
    };
    (
        m.db.clone(),
        a.session.cwd.clone().unwrap_or_default(),
        a.session.permission_mode.clone(),
        a.session.model.clone(),
        a.claude_session_id.clone(),
        a.session.ssh_host.clone(),
        a.session.ssh_user.clone(),
    )
};

let spawn_mode = match (&ssh_host, &ssh_user) {
    (Some(host), Some(user)) => crate::services::spawn_manager::SpawnMode::Ssh {
        host: host.clone(),
        user: user.clone(),
    },
    _ => crate::services::spawn_manager::SpawnMode::Local,
};

let config = SpawnConfig {
    session_id,
    cwd: std::path::PathBuf::from(&cwd),
    remote_cwd: ssh_host.as_ref().map(|_| cwd.clone()),
    permission_mode,
    model,
    prompt,
    claude_session_id,
    spawn_mode,
};
```

- [ ] **Step 3: Fix `init_session` calls in `session_manager.rs` tests**

Every call in `#[cfg(test)]` that calls `init_session` needs two extra `None` args:

```rust
// Before: m.init_session("/tmp/proj", None, "ignore", None)
// After:
m.init_session("/tmp/proj", None, "ignore", None, None, None)
```

There are 6 such calls — update all of them.

- [ ] **Step 4: Update `create_session` Tauri command in `ipc/session.rs`**

```rust
#[tauri::command]
pub fn create_session(
    project_path: String,
    prompt: String,
    model: Option<String>,
    permission_mode: Option<String>,
    session_name: Option<String>,
    ssh_host: Option<String>,
    ssh_user: Option<String>,
    state: State<SessionState>,
    app: AppHandle,
) -> Result<Session, String> {
    let mode = permission_mode.unwrap_or_else(|| "ignore".to_string());

    let session = {
        let mut m = state.0.lock().unwrap();
        m.init_session(
            &project_path,
            session_name.as_deref(),
            &mode,
            model.as_deref(),
            ssh_host.as_deref(),
            ssh_user.as_deref(),
        )?
    };

    use tauri::Emitter;
    let _ = app.emit("session:created", &session);

    let manager = Arc::clone(&state.0);
    let session_id = session.id;
    std::thread::spawn(move || {
        SessionManager::do_spawn(manager, app, session_id, prompt);
    });

    Ok(session)
}
```

- [ ] **Step 5: Run all Rust tests**

```bash
cd front && cargo test 2>&1 | tail -30
```

Expected: all tests pass.

- [ ] **Step 6: Run clippy**

```bash
cd front && cargo clippy -- -D warnings 2>&1 | tail -20
```

Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
cd /c/Users/fernandonepen/Documents/agent-dashboard-v2
git add front/src/services/session_manager.rs front/src/ipc/session.rs
git commit -m "feat(ssh): wire SpawnMode and SSH params through session manager and IPC"
```

---

## Task 4: TypeScript types and mock

**Files:**
- Modify: `api/lib/stores/sessions.ts`
- Modify: `api/lib/tauri.ts`
- Modify: `api/lib/mock/tauri-mock.ts`
- Modify: `api/lib/stores/sessions.test.ts`

- [ ] **Step 1: Add `sshHost` and `sshUser` to `Session` interface in `sessions.ts`**

Add the two fields after `costUsd` in the `Session` interface:

```typescript
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
  costUsd: number | null;
  sshHost: string | null;
  sshUser: string | null;
  createdAt: string;
  updatedAt: string;
}
```

- [ ] **Step 2: Add SSH fields to `CreateSessionOptions` and `createSession` in `tauri.ts`**

```typescript
export interface CreateSessionOptions {
  projectPath: string;
  prompt: string;
  model?: string;
  permissionMode?: 'ignore' | 'approve';
  sessionName?: string;
  sshHost?: string;
  sshUser?: string;
}

export async function createSession(opts: CreateSessionOptions): Promise<Session> {
  return await invoke('create_session', {
    projectPath: opts.projectPath,
    prompt: opts.prompt,
    model: opts.model ?? null,
    permissionMode: opts.permissionMode ?? 'ignore',
    sessionName: opts.sessionName ?? null,
    sshHost: opts.sshHost ?? null,
    sshUser: opts.sshUser ?? null,
  });
}
```

- [ ] **Step 3: Add `sshHost: null, sshUser: null` to all 4 mock sessions in `tauri-mock.ts`**

In every session object in `MOCK_SESSIONS`, add after `costUsd: null`:

```typescript
sshHost: null,
sshUser: null,
```

There are 4 mock session objects — add to all of them.

- [ ] **Step 4: Add `sshHost` and `sshUser` to `makeSession()` factory in `sessions.test.ts`**

```typescript
function makeSession(overrides: Partial<Session> = {}): Session {
  return {
    id: 1,
    projectId: null,
    name: null,
    status: 'running',
    permissionMode: 'ignore',
    model: null,
    pid: null,
    cwd: '/tmp/proj',
    projectName: 'proj',
    gitBranch: null,
    tokens: null,
    contextPercent: null,
    pendingApproval: null,
    miniLog: null,
    costUsd: null,
    sshHost: null,
    sshUser: null,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}
```

- [ ] **Step 5: Run TypeScript tests**

```bash
npm run test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 6: Run ESLint + svelte-check**

```bash
npm run lint 2>&1 | tail -20
```

Expected: 0 errors, 0 warnings.

- [ ] **Step 7: Commit**

```bash
cd /c/Users/fernandonepen/Documents/agent-dashboard-v2
git add api/lib/stores/sessions.ts api/lib/tauri.ts api/lib/mock/tauri-mock.ts api/lib/stores/sessions.test.ts
git commit -m "feat(ssh): add sshHost/sshUser to TypeScript Session type and mock"
```

---

## Task 5: New Session Modal — SSH mode UI

**Files:**
- Modify: `api/components/NewSessionModal.svelte`

- [ ] **Step 1: Replace the `<script>` block with the SSH-aware version**

```svelte
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { createSession, diagnoseSpawn } from '../lib/tauri';
  import type { SpawnDiagnostic } from '../lib/tauri';

  const dispatch = createEventDispatcher();

  let path = '';
  let prompt = '';
  let model = 'auto';
  let loading = false;
  let error = '';
  let diagRunning = false;
  let diag: SpawnDiagnostic | null = null;

  let sshMode = false;
  let sshHost = '';
  let sshUser = 'ubuntu';

  async function runDiag() {
    diagRunning = true;
    try {
      diag = await diagnoseSpawn();
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      diagRunning = false;
    }
  }

  const models = [
    { v: 'auto', l: 'auto' },
    { v: 'claude-sonnet-4-6', l: 'sonnet-4.6' },
    { v: 'claude-opus-4-6', l: 'opus-4.6' },
    { v: 'claude-haiku-4-5-20251001', l: 'haiku-4.5' },
  ];

  async function browse() {
    const sel = await open({ directory: true, multiple: false });
    if (sel && typeof sel === 'string') path = sel;
  }

  async function submit() {
    if (!path.trim()) {
      error = sshMode ? 'remote path required' : 'project path required';
      return;
    }
    if (sshMode && !sshHost.trim()) {
      error = 'ssh host required';
      return;
    }
    if (sshMode && !sshUser.trim()) {
      error = 'ssh user required';
      return;
    }
    loading = true;
    error = '';
    try {
      await createSession({
        projectPath: path.trim(),
        prompt: prompt.trim() || 'Hello',
        model: model === 'auto' ? undefined : model,
        permissionMode: 'ignore',
        sshHost: sshMode ? sshHost.trim() : undefined,
        sshUser: sshMode ? sshUser.trim() : undefined,
      });
      dispatch('done');
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      loading = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') dispatch('cancel');
  }
</script>
```

- [ ] **Step 2: Replace the modal template (everything inside `<div class="modal">`)**

```svelte
<svelte:window on:keydown={onKey} />

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  on:click|self={() => dispatch('cancel')}
  on:keydown={onKey}
>
  <div class="modal">
    <div class="modal-header">
      <span class="modal-title">new session</span>
      <button class="close" on:click={() => dispatch('cancel')}>✕</button>
    </div>

    <div class="mode-toggle">
      <button
        class="mode-btn"
        class:active={!sshMode}
        on:click={() => (sshMode = false)}
        disabled={loading}
      >local</button>
      <button
        class="mode-btn"
        class:active={sshMode}
        on:click={() => (sshMode = true)}
        disabled={loading}
      >ssh remote</button>
    </div>

    {#if sshMode}
      <div class="row">
        <div class="field" style="flex:2">
          <label class="label" for="ns-ssh-host">host</label>
          <input
            id="ns-ssh-host"
            class="input"
            bind:value={sshHost}
            placeholder="vps.example.com"
            disabled={loading}
          />
        </div>
        <div class="field" style="flex:1">
          <label class="label" for="ns-ssh-user">user</label>
          <input
            id="ns-ssh-user"
            class="input"
            bind:value={sshUser}
            placeholder="ubuntu"
            disabled={loading}
          />
        </div>
      </div>
    {/if}

    <div class="field">
      <label class="label" for="ns-path">{sshMode ? 'remote path' : 'path'}</label>
      <div class="path-row">
        <input
          id="ns-path"
          class="input"
          bind:value={path}
          placeholder={sshMode ? '/home/ubuntu/project' : '/home/user/project'}
          disabled={loading}
          on:keydown={(e) => e.key === 'Enter' && prompt && submit()}
        />
        {#if !sshMode}
          <button class="browse" on:click={browse} disabled={loading} title="browse">⌘</button>
        {/if}
      </div>
    </div>

    <div class="field">
      <label class="label" for="ns-prompt">prompt</label>
      <textarea
        id="ns-prompt"
        class="input textarea"
        bind:value={prompt}
        placeholder="what should claude work on? (optional)"
        rows="3"
        disabled={loading}
        on:keydown={(e) => {
          if (e.key === 'Enter' && e.metaKey) submit();
        }}
      ></textarea>
    </div>

    <div class="row">
      <div class="field half">
        <label class="label" for="ns-model">model</label>
        <select id="ns-model" class="input select" bind:value={model} disabled={loading}>
          {#each models as m}
            <option value={m.v}>{m.l}</option>
          {/each}
        </select>
      </div>
    </div>

    {#if sshMode}
      <p class="ssh-hint">
        requires key-based auth (no passphrase or ssh-agent) and <code>claude</code> installed on
        the remote host
      </p>
    {/if}

    {#if error}
      <p class="error">! {error}</p>
    {/if}

    {#if diag && !sshMode}
      <div class="diag">
        <div class="diag-row" class:ok={diag.claudeFound} class:fail={!diag.claudeFound}>
          claude: {diag.claudeFound ? `✓ ${diag.claudePath ?? diag.whereOutput}` : '✗ not found'}
        </div>
        {#if diag.versionOutput}
          <div class="diag-row ok">version: {diag.versionOutput.slice(0, 60)}</div>
        {/if}
        {#if !diag.claudeFound}
          <div class="diag-row fail">install: npm install -g @anthropic-ai/claude-code</div>
          <div class="diag-row" style="font-size:9px;color:var(--t3)">
            PATH: {diag.augmentedPath.slice(0, 120)}
          </div>
        {/if}
      </div>
    {/if}

    <div class="actions">
      {#if !sshMode}
        <button class="btn ghost" on:click={runDiag} disabled={diagRunning || loading}>
          {diagRunning ? 'testing...' : '⚙ diagnose'}
        </button>
      {/if}
      <button class="btn ghost" on:click={() => dispatch('cancel')} disabled={loading}
        >cancel</button
      >
      <button class="btn primary" on:click={submit} disabled={loading || !path}>
        {loading ? 'spawning...' : 'start session'}
      </button>
    </div>
  </div>
</div>
```

- [ ] **Step 3: Add CSS for the new SSH elements at the end of the `<style>` block**

```css
  .mode-toggle {
    display: flex;
    gap: 4px;
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: 3px;
    padding: 3px;
  }
  .mode-btn {
    flex: 1;
    background: none;
    border: none;
    color: var(--t2);
    font-size: var(--xs);
    padding: 4px 8px;
    border-radius: 2px;
    letter-spacing: 0.04em;
    transition: all 0.15s;
  }
  .mode-btn.active {
    background: var(--bg3);
    color: var(--t0);
  }
  .mode-btn:hover:not(.active) {
    color: var(--t1);
  }
  .ssh-hint {
    font-size: var(--xs);
    color: var(--t3);
    line-height: 1.5;
    margin: 0;
  }
  .ssh-hint code {
    color: var(--t2);
    background: var(--bg3);
    padding: 0 3px;
    border-radius: 2px;
  }
```

- [ ] **Step 4: Run `svelte-check`**

```bash
npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -20
```

Expected: 0 errors, 0 warnings.

- [ ] **Step 5: Run full lint**

```bash
npm run lint 2>&1 | tail -20
```

Expected: passes with 0 warnings.

- [ ] **Step 6: Commit**

```bash
cd /c/Users/fernandonepen/Documents/agent-dashboard-v2
git add api/components/NewSessionModal.svelte
git commit -m "feat(ssh): add SSH mode toggle and fields to NewSessionModal"
```

---

## Task 6: Final verification

- [ ] **Step 1: Run all Rust tests**

```bash
cd front && cargo test 2>&1 | tail -30
```

Expected: all tests pass, no failures.

- [ ] **Step 2: Run clippy**

```bash
cd front && cargo clippy -- -D warnings 2>&1 | tail -20
```

Expected: no warnings.

- [ ] **Step 3: Run all TypeScript tests**

```bash
cd /c/Users/fernandonepen/Documents/agent-dashboard-v2 && npm run test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 4: Run full lint suite**

```bash
npm run lint 2>&1 | tail -30
```

Expected: passes completely.

- [ ] **Step 5: Verify git log for this branch**

```bash
git log master..HEAD --oneline
```

Expected (5 commits in order):
```
feat(ssh): add SSH mode toggle and fields to NewSessionModal
feat(ssh): add sshHost/sshUser to TypeScript Session type and mock
feat(ssh): wire SpawnMode and SSH params through session manager and IPC
feat(ssh): add SpawnMode enum and SSH spawn via ssh + bash -lc
feat(ssh): add ssh_host/ssh_user to Session model and DB
```

---

## Known limitations (Phase 1 scope)

These features do **not** work for SSH sessions and are intentionally out of scope:

| Feature | Why it breaks | Future fix |
|---------|--------------|------------|
| File diff viewer | Reads local filesystem paths | Phase 2: SSH file fetch command |
| Directory browser (⌘ button) | Opens local OS dialog | Intentionally hidden in SSH mode |
| `diagnose_spawn` | Checks local `claude` binary | Phase 2: SSH connectivity check command |
| Subagent journal (`.meta.json`) | Reads local agent tree | Phase 2: remote file read |
| `list_project_files` for `@` picker | Reads local directory | Phase 2: remote `ls` command |

For SSH sessions, the `CentralPanel` journal feed, chat input, tokens, cost, and context % all work without any changes because stream-json flows through the SSH pipe identically to a local process.
