# Merge SSH Sessions into Multi-Provider Architecture

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Merge SSH remote session support (from `feat/ssh-sessions`) into the multi-provider architecture (from `feat/openrouter-provider`), enabling SSH spawning for all providers (Claude Code, Codex, OpenCode).

**Architecture:** Extract SSH utilities into a dedicated `services/ssh.rs` module. Add `SpawnMode` enum and SSH fields to `ProviderSpawnConfig` so all providers share the same SSH plumbing. Each provider builds its CLI command string; a shared `spawn_via_ssh()` function wraps it in an SSH tunnel. DB gets `ssh_host`/`ssh_user` columns. Frontend gets SSH toggle in `NewSessionModal` alongside the existing provider selector.

**Tech Stack:** Rust 1.85, Tauri 2.x, SQLite (rusqlite 0.31), SvelteKit 2 + Svelte 5 + TypeScript 5.6

**Base branch:** `feat/openrouter-provider` (the current branch — more complex, more recent architecture)
**Source of SSH code:** `feat/ssh-sessions`

---

## Branch Analysis

### What exists on feat/openrouter-provider (base)
- **Provider trait system**: `providers/mod.rs`, `claude.rs`, `codex.rs`, `opencode.rs`
- **ProviderRegistry**: maps provider IDs → trait impls, fallback to opencode for unknown IDs
- **Three spawn functions**: `spawn_claude()`, `spawn_codex()`, `spawn_opencode()` in `spawn_manager.rs`
- **Three JSONL parsers**: `process_line()`, `process_line_codex()`, `process_line_opencode()` in `journal/processor.rs`
- **Provider-aware session manager**: `do_spawn()` resolves provider from registry, `send_message()` passes `Arc<ProviderRegistry>`
- **Database**: `provider` column exists, no SSH columns
- **Models**: `provider: String` in Session, no SSH fields, no `InitSessionParams`
- **Frontend**: Provider selector in NewSessionModal, `provider` in Session store, `getProviders()`, `diagnoseProvider()`, `setSessionApiKey()` in tauri.ts
- **IPC**: `create_session` accepts `provider`, `ProviderRegistryState` in session.rs

### What needs to come from feat/ssh-sessions
- **SSH utilities**: `AskpassGuard`, `create_askpass()`, `apply_askpass()`, `validate_ssh_host()`, `validate_ssh_user()`, `posix_escape()`, `test_ssh_connection()`
- **SpawnMode enum**: `Local` vs `Ssh { host, user }`
- **SSH in SpawnHandle**: `_askpass: Option<AskpassGuard>` field
- **SSH spawn logic**: Build CLI command string → wrap in `ssh user@host bash -lc '...'`
- **SSH columns in DB**: `ssh_host`, `ssh_user`
- **SSH fields in Session**: `ssh_host: Option<String>`, `ssh_user: Option<String>`
- **SSH password in memory**: `ActiveSession.ssh_password`
- **test_ssh IPC command**: test SSH connectivity without creating a session
- **Frontend SSH UI**: toggle (local/ssh), host/user/password fields, test connection button

### Key Design Decisions
1. **SSH module extraction**: All SSH utilities go to `services/ssh.rs` for reuse across providers
2. **Shared `spawn_via_ssh()`**: A single function handles SSH wrapping for any CLI command — providers only build their command string
3. **All providers support SSH**: `SpawnMode` + `ssh_password` added to `ProviderSpawnConfig`, not just Claude
4. **Password never persisted**: SSH password held in `ActiveSession.ssh_password` (memory only)
5. **Worktrees disabled for SSH**: `use_worktree && ssh_host.is_none()` guard

---

## File Map

| Action | File | Change |
|--------|------|--------|
| **Create** | `tauri/src/services/ssh.rs` | SSH askpass, validation, test connection, `spawn_via_ssh()` |
| Modify | `tauri/src/services/mod.rs` | Add `pub mod ssh;` |
| Modify | `tauri/src/models.rs` | Add `ssh_host`, `ssh_user` to Session |
| Modify | `tauri/src/services/database.rs` | Migration + CRUD for SSH columns |
| Modify | `tauri/src/services/spawn_manager.rs` | Add `SpawnMode`, `_askpass` to SpawnHandle |
| Modify | `tauri/src/providers/mod.rs` | Add `spawn_mode`, `ssh_password` to `ProviderSpawnConfig` |
| Modify | `tauri/src/providers/claude.rs` | SSH-aware `spawn()` using `spawn_via_ssh()` |
| Modify | `tauri/src/providers/codex.rs` | SSH-aware `spawn()` using `spawn_via_ssh()` |
| Modify | `tauri/src/providers/opencode.rs` | SSH-aware `spawn()` using `spawn_via_ssh()` |
| Modify | `tauri/src/services/session_manager.rs` | SSH params in `init_session`, `SpawnMode` in `do_spawn`, `ssh_password` in `ActiveSession` |
| Modify | `tauri/src/ipc/session.rs` | SSH params in `create_session`, `test_ssh` command |
| Modify | `tauri/src/lib.rs` | Register `test_ssh` command |
| Modify | `ui/lib/stores/sessions.ts` | Add `sshHost`, `sshUser` to Session interface |
| Modify | `ui/lib/tauri.ts` | Add SSH fields to `CreateSessionOptions`, add `testSsh()`, `SshTestResult` |
| Modify | `ui/lib/mock/tauri-mock.ts` | Add SSH fields to mock sessions |
| Modify | `ui/components/NewSessionModal.svelte` | SSH toggle + host/user/password fields + test connection button |

---

## Task 1: Create SSH service module

**Files:**
- Create: `tauri/src/services/ssh.rs`
- Modify: `tauri/src/services/mod.rs`

- [ ] **Step 1: Create `tauri/src/services/ssh.rs`**

This module extracts all SSH utilities from the `feat/ssh-sessions` branch's `spawn_manager.rs` into a dedicated, reusable module. It includes `AskpassGuard`, askpass creation, validation, test connection, POSIX escaping, and a new `spawn_via_ssh()` function that wraps any CLI command string in an SSH tunnel.

```rust
//! SSH utilities for remote session spawning.
//!
//! Provides:
//! - Password authentication via temporary askpass scripts
//! - Host/user validation to prevent injection
//! - Test connection utility
//! - `spawn_via_ssh()` — wraps any CLI command string in an SSH tunnel

use std::path::PathBuf;

/// RAII guard that deletes the temporary askpass directory on drop.
pub struct AskpassGuard {
    dir: PathBuf,
}

impl Drop for AskpassGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// How to spawn a CLI process: locally or via SSH tunnel.
#[derive(Debug, Clone)]
pub enum SpawnMode {
    Local,
    Ssh { host: String, user: String },
}

/// Create a temporary SSH_ASKPASS helper that echoes `password`.
/// Returns `(guard, script_path)`. The guard deletes the temp dir on drop.
pub fn create_askpass(password: &str) -> Result<(AskpassGuard, String), String> {
    let tmp = std::env::temp_dir().join(format!("orbit-ssh-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("askpass dir: {e}"))?;

    let pw_file = tmp.join("pw");
    std::fs::write(&pw_file, format!("{password}\n")).map_err(|e| format!("askpass pw: {e}"))?;

    let script_path: PathBuf;

    #[cfg(windows)]
    {
        script_path = tmp.join("ask.cmd");
        std::fs::write(&script_path, "@type \"%~dp0pw\"\r\n")
            .map_err(|e| format!("askpass cmd: {e}"))?;
    }

    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        script_path = tmp.join("ask.sh");
        let pw_str = pw_file.display().to_string().replace('\'', "'\\''");
        std::fs::write(&script_path, format!("#!/bin/sh\ncat '{pw_str}'\n"))
            .map_err(|e| format!("askpass script: {e}"))?;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("askpass chmod script: {e}"))?;
        std::fs::set_permissions(&pw_file, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("askpass chmod pw: {e}"))?;
    }

    let script_str = script_path.display().to_string();
    Ok((AskpassGuard { dir: tmp }, script_str))
}

/// Apply SSH_ASKPASS env vars to `cmd` for password authentication.
/// Returns an `AskpassGuard` that must be kept alive until the process exits.
pub fn apply_askpass(
    cmd: &mut std::process::Command,
    password: &str,
) -> Result<AskpassGuard, String> {
    let (guard, script_path) = create_askpass(password)?;
    cmd.env("SSH_ASKPASS", &script_path);
    cmd.env("SSH_ASKPASS_REQUIRE", "force");
    #[cfg(not(windows))]
    {
        if std::env::var("DISPLAY").is_err() {
            cmd.env("DISPLAY", ":0");
        }
    }
    Ok(guard)
}

/// Validate that an SSH host string contains only safe characters.
pub fn validate_ssh_host(host: &str) -> bool {
    !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'))
}

/// Validate that an SSH user string contains only safe characters.
pub fn validate_ssh_user(user: &str) -> bool {
    !user.is_empty()
        && user
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Wrap a string in single quotes, escaping embedded single quotes.
/// Safe for embedding in a POSIX shell command string.
pub fn posix_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Result of a test SSH connection attempt.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTestResult {
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// Test SSH connectivity without spawning a full session.
/// Runs `ssh ... "echo __orbit_ok__"` with a 15-second wall-clock timeout.
pub fn test_ssh_connection(host: &str, user: &str, password: Option<&str>) -> SshTestResult {
    if !validate_ssh_host(host) {
        return SshTestResult {
            ok: false,
            latency_ms: None,
            error: Some(format!("invalid host: {host:?}")),
        };
    }
    if !validate_ssh_user(user) {
        return SshTestResult {
            ok: false,
            latency_ms: None,
            error: Some(format!("invalid user: {user:?}")),
        };
    }

    let mut cmd = std::process::Command::new("ssh");
    cmd.args([
        "-o", "BatchMode=no",
        "-o", "ConnectTimeout=10",
        "-o", "StrictHostKeyChecking=accept-new",
        "-o", "LogLevel=ERROR",
        "-o", "ControlMaster=no",
        "-o", "ControlPath=none",
    ]);
    if password.is_some() {
        cmd.args([
            "-o", "PreferredAuthentications=keyboard-interactive,password",
            "-o", "NumberOfPasswordPrompts=1",
        ]);
    }
    cmd.args([&format!("{user}@{host}"), "echo __orbit_ok__"]);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.stdin(std::process::Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let _guard = if let Some(pw) = password {
        match apply_askpass(&mut cmd, pw) {
            Ok(g) => Some(g),
            Err(e) => {
                return SshTestResult {
                    ok: false,
                    latency_ms: None,
                    error: Some(e),
                }
            }
        }
    } else {
        None
    };

    let start = std::time::Instant::now();
    let hard_timeout = std::time::Duration::from_secs(15);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return SshTestResult {
                ok: false,
                latency_ms: None,
                error: Some(format!("failed to start ssh: {e}")),
            }
        }
    };

    let stdout_handle = {
        let mut stdout = child.stdout.take().expect("stdout piped");
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = std::io::Read::read_to_end(&mut stdout, &mut buf);
            buf
        })
    };
    let stderr_handle = {
        let mut stderr = child.stderr.take().expect("stderr piped");
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = std::io::Read::read_to_end(&mut stderr, &mut buf);
            buf
        })
    };

    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Ok(s),
            Ok(None) => {
                if start.elapsed() >= hard_timeout {
                    let _ = child.kill();
                    return SshTestResult {
                        ok: false,
                        latency_ms: None,
                        error: Some("connection timed out (15s)".to_string()),
                    };
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => break Err(e),
        }
    };

    let latency_ms = start.elapsed().as_millis() as u64;
    let stdout_bytes = stdout_handle.join().unwrap_or_default();
    let stderr_bytes = stderr_handle.join().unwrap_or_default();

    match status {
        Ok(s) => {
            let stdout = String::from_utf8_lossy(&stdout_bytes);
            if s.success() && stdout.contains("__orbit_ok__") {
                SshTestResult {
                    ok: true,
                    latency_ms: Some(latency_ms),
                    error: None,
                }
            } else {
                let stderr = String::from_utf8_lossy(&stderr_bytes).trim().to_string();
                SshTestResult {
                    ok: false,
                    latency_ms: None,
                    error: Some(if stderr.is_empty() {
                        format!("exit code {}", s.code().unwrap_or(-1))
                    } else {
                        stderr
                    }),
                }
            }
        }
        Err(e) => SshTestResult {
            ok: false,
            latency_ms: None,
            error: Some(e.to_string()),
        },
    }
}

/// Spawn any CLI command string on a remote server via SSH.
///
/// Builds: `ssh [opts] user@host "bash -lc '<remote_script>'"`.
/// The `remote_script` should already be a properly escaped shell command string
/// (use `posix_escape()` for individual arguments).
///
/// Returns a `SpawnHandle` — same interface as local spawns.
pub fn spawn_via_ssh(
    host: &str,
    user: &str,
    password: Option<&str>,
    remote_script: &str,
) -> Result<(std::process::Child, Option<AskpassGuard>), String> {
    if !validate_ssh_host(host) {
        return Err(format!("invalid ssh host: {host:?}"));
    }
    if !validate_ssh_user(user) {
        return Err(format!("invalid ssh user: {user:?}"));
    }

    let batch_mode = if password.is_some() { "no" } else { "yes" };

    let mut cmd = std::process::Command::new("ssh");
    cmd.args([
        "-o", &format!("BatchMode={batch_mode}"),
        "-o", "ConnectTimeout=10",
        "-o", "StrictHostKeyChecking=accept-new",
        "-o", "ControlMaster=no",
        "-o", "ControlPath=none",
        &format!("{user}@{host}"),
    ]);
    cmd.arg(format!("bash -lc {}", posix_escape(remote_script)));
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.stdin(std::process::Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let askpass = if let Some(pw) = password {
        Some(apply_askpass(&mut cmd, pw)?)
    } else {
        None
    };

    let child = cmd.spawn().map_err(|e| format!("ssh spawn failed: {e}"))?;
    Ok((child, askpass))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ssh_host_accepts_valid_values() {
        assert!(validate_ssh_host("vps.example.com"));
        assert!(validate_ssh_host("192.168.1.1"));
        assert!(validate_ssh_host("[::1]"));
        assert!(validate_ssh_host("my-server"));
    }

    #[test]
    fn test_validate_ssh_host_rejects_injection() {
        assert!(!validate_ssh_host(""));
        assert!(!validate_ssh_host("-oProxyCommand=evil"));
        assert!(!validate_ssh_host("host;rm -rf /"));
        assert!(!validate_ssh_host("host$(whoami)"));
        assert!(!validate_ssh_host("host`cmd`"));
    }

    #[test]
    fn test_validate_ssh_user_accepts_valid_values() {
        assert!(validate_ssh_user("ubuntu"));
        assert!(validate_ssh_user("deploy_user"));
        assert!(validate_ssh_user("user.name"));
        assert!(validate_ssh_user("user-1"));
    }

    #[test]
    fn test_validate_ssh_user_rejects_injection() {
        assert!(!validate_ssh_user(""));
        assert!(!validate_ssh_user("user name"));
        assert!(!validate_ssh_user("user;id"));
        assert!(!validate_ssh_user("user$(id)"));
    }

    #[test]
    fn test_posix_escape_simple() {
        assert_eq!(posix_escape("hello world"), "'hello world'");
    }

    #[test]
    fn test_posix_escape_with_single_quote() {
        assert_eq!(posix_escape("it's a test"), "'it'\\''s a test'");
    }

    #[test]
    fn test_posix_escape_empty() {
        assert_eq!(posix_escape(""), "''");
    }
}
```

- [ ] **Step 2: Export SSH module in `services/mod.rs`**

Add `pub mod ssh;` to `tauri/src/services/mod.rs`:

```rust
pub mod database;
pub mod session_manager;
pub mod spawn_manager;
pub mod ssh;
pub mod worktree;
```

- [ ] **Step 3: Run SSH unit tests**

```bash
cd tauri && cargo test ssh:: -- --nocapture
```

Expected: All 7 tests PASS (validate_host ×2, validate_user ×2, posix_escape ×3).

- [ ] **Step 4: Commit**

```bash
git add tauri/src/services/ssh.rs tauri/src/services/mod.rs
git commit -m "feat: create SSH service module with askpass, validation, and spawn_via_ssh"
```

---

## Task 2: Add SSH fields to Session model and database

**Files:**
- Modify: `tauri/src/models.rs`
- Modify: `tauri/src/services/database.rs`

- [ ] **Step 1: Add `ssh_host` and `ssh_user` to Session struct in `models.rs`**

In `tauri/src/models.rs`, add two fields after `mini_log` in the `Session` struct (around line 312):

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mini_log: Option<Vec<MiniLogEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_user: Option<String>,
}
```

- [ ] **Step 2: Add DB migration for SSH columns in `database.rs`**

In the `migrate()` method (around line 96, after the `provider` column migration), add:

```rust
let _ = conn.execute_batch("ALTER TABLE sessions ADD COLUMN ssh_host TEXT");
let _ = conn.execute_batch("ALTER TABLE sessions ADD COLUMN ssh_user TEXT");
```

- [ ] **Step 3: Update `create_session` signature to accept SSH fields**

Change `create_session` in `database.rs` from:
```rust
pub fn create_session(
    &self,
    project_id: Option<i64>,
    name: Option<&str>,
    cwd: &str,
    permission_mode: &str,
    model: Option<&str>,
    provider: Option<&str>,
) -> SqlResult<SessionId>
```

To:
```rust
#[allow(clippy::too_many_arguments)]
pub fn create_session(
    &self,
    project_id: Option<i64>,
    name: Option<&str>,
    cwd: &str,
    permission_mode: &str,
    model: Option<&str>,
    provider: Option<&str>,
    ssh_host: Option<&str>,
    ssh_user: Option<&str>,
) -> SqlResult<SessionId>
```

Update the INSERT statement:
```rust
conn.execute(
    "INSERT INTO sessions (project_id, name, cwd, status, permission_mode, model, provider, ssh_host, ssh_user)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    params![
        project_id,
        name,
        cwd,
        crate::models::SessionStatus::Initializing,
        permission_mode,
        model,
        provider.unwrap_or("claude-code"),
        ssh_host,
        ssh_user,
    ],
)?;
```

- [ ] **Step 4: Update `get_sessions` to include SSH columns**

In `get_sessions()`, update the SELECT query and Session mapping:

```rust
let mut stmt = conn.prepare(
    "SELECT id, project_id, name, status, worktree_path, branch_name,
            permission_mode, model, pid, cwd, created_at, updated_at, provider,
            ssh_host, ssh_user
     FROM sessions ORDER BY created_at DESC",
)?;
```

Add to the Session struct construction:
```rust
    provider: row
        .get::<_, Option<String>>(12)?
        .unwrap_or_else(|| "claude-code".to_string()),
    ssh_host: row.get(13)?,
    ssh_user: row.get(14)?,
```

- [ ] **Step 5: Update `get_session` with the same SSH columns**

Same changes as Step 4, applied to the `get_session()` method.

- [ ] **Step 6: Fix all `create_session` callers**

In `session_manager.rs`, the `init_session` call to `db.create_session(...)` needs the two new `None` args for now:

```rust
let session_id = self
    .db
    .create_session(
        Some(project.id),
        session_name,
        project_path,
        permission_mode,
        model,
        provider,
        None, // ssh_host — will be wired in Task 5
        None, // ssh_user — will be wired in Task 5
    )
    .map_err(|e| e.to_string())?;
```

Also update the `Session` construction in `init_session` to include:
```rust
    ssh_host: None,
    ssh_user: None,
```

- [ ] **Step 7: Fix all other `Session` construction sites**

Search the codebase for every place that constructs a `Session { ... }` literal and add `ssh_host: None, ssh_user: None`. This includes:
- `database.rs` → `get_sessions()` and `get_session()` (already done in steps 4-5)
- `session_manager.rs` → `init_session()` (done in step 6)
- Any test helpers that construct Session instances

- [ ] **Step 8: Add database test for SSH columns**

Add to `database.rs` tests:

```rust
#[test]
fn should_store_and_retrieve_ssh_host_and_user() {
    let mut t = crate::test_utils::TestCase::new("should_store_and_retrieve_ssh_host_and_user");

    t.phase("Seed");
    let db = DatabaseService::open_in_memory().unwrap();
    let pid = db.create_session(
        None,
        Some("ssh-test"),
        "/tmp",
        "ignore",
        Some("claude-sonnet"),
        Some("claude-code"),
        Some("vps.example.com"),
        Some("ubuntu"),
    ).unwrap();

    t.phase("Assert SSH session");
    let s = db.get_session(pid).unwrap().unwrap();
    t.eq("ssh_host stored", s.ssh_host.as_deref(), Some("vps.example.com"));
    t.eq("ssh_user stored", s.ssh_user.as_deref(), Some("ubuntu"));

    t.phase("Assert local session has null SSH");
    let pid2 = db.create_session(
        None,
        Some("local-test"),
        "/tmp",
        "ignore",
        Some("claude-sonnet"),
        Some("claude-code"),
        None,
        None,
    ).unwrap();
    let s2 = db.get_session(pid2).unwrap().unwrap();
    t.none("ssh_host is None for local session", &s2.ssh_host);
    t.none("ssh_user is None for local session", &s2.ssh_user);
}
```

- [ ] **Step 9: Run tests, verify compilation**

```bash
cd tauri && cargo test -- --nocapture
```

Expected: All existing tests PASS, new SSH DB test PASSES.

- [ ] **Step 10: Commit**

```bash
git add tauri/src/models.rs tauri/src/services/database.rs tauri/src/services/session_manager.rs
git commit -m "feat: add ssh_host and ssh_user to Session model and database"
```

---

## Task 3: Add SpawnMode to spawn_manager and ProviderSpawnConfig

**Files:**
- Modify: `tauri/src/services/spawn_manager.rs`
- Modify: `tauri/src/providers/mod.rs`

- [ ] **Step 1: Add `_askpass` field to `SpawnHandle` in `spawn_manager.rs`**

Update the `SpawnHandle` struct to keep the askpass guard alive:

```rust
pub struct SpawnHandle {
    pub pid: u32,
    pub reader: Box<dyn std::io::Read + Send>,
    pub stderr: Box<dyn std::io::Read + Send>,
    pub child: std::process::Child,
    /// Keeps the askpass temp dir alive for the duration of the SSH session.
    pub _askpass: Option<crate::services::ssh::AskpassGuard>,
}
```

- [ ] **Step 2: Update all `SpawnHandle` construction sites to include `_askpass: None`**

In `spawn_claude()`, `spawn_opencode()`, and `spawn_codex()` in `spawn_manager.rs`, add `_askpass: None` to each `SpawnHandle { ... }` construction.

- [ ] **Step 3: Add SSH fields to `ProviderSpawnConfig` in `providers/mod.rs`**

```rust
pub struct ProviderSpawnConfig {
    pub session_id: crate::models::SessionId,
    pub cwd: std::path::PathBuf,
    pub model: String,
    pub prompt: String,
    pub resume_id: Option<String>,
    pub extra_env: Vec<(String, String)>,
    pub effort: Option<String>,
    /// How to spawn: locally or via SSH tunnel.
    pub spawn_mode: crate::services::ssh::SpawnMode,
    /// SSH password (memory only, never persisted).
    pub ssh_password: Option<String>,
}
```

- [ ] **Step 4: Update `do_spawn` in `session_manager.rs` to pass SpawnMode**

In `do_spawn()`, when constructing `ProviderSpawnConfig`, add:

```rust
spawn_mode: crate::services::ssh::SpawnMode::Local,
ssh_password: None,
```

These will be wired to real values in Task 5.

- [ ] **Step 5: Run tests, verify compilation**

```bash
cd tauri && cargo test -- --nocapture
```

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add tauri/src/services/spawn_manager.rs tauri/src/providers/mod.rs tauri/src/services/session_manager.rs
git commit -m "feat: add SpawnMode to ProviderSpawnConfig and _askpass to SpawnHandle"
```

---

## Task 4: Make providers SSH-aware

**Files:**
- Modify: `tauri/src/providers/claude.rs`
- Modify: `tauri/src/providers/codex.rs`
- Modify: `tauri/src/providers/opencode.rs`

Each provider's `spawn()` method needs to handle `SpawnMode::Ssh` by building a CLI command string and calling `ssh::spawn_via_ssh()`.

- [ ] **Step 1: Update `ClaudeProvider::spawn()` for SSH**

```rust
use crate::services::ssh::{self, SpawnMode};

fn spawn(&self, config: ProviderSpawnConfig) -> Result<SpawnHandle, String> {
    match config.spawn_mode {
        SpawnMode::Local => {
            // Existing local spawn logic (unchanged)
            spawn_claude(SpawnConfig {
                session_id: config.session_id,
                cwd: config.cwd,
                permission_mode: "ignore".to_string(),
                model: if config.model == "auto" {
                    None
                } else {
                    Some(config.model)
                },
                effort: config.effort,
                prompt: config.prompt,
                claude_session_id: config.resume_id,
            })
        }
        SpawnMode::Ssh { ref host, ref user } => {
            let mut parts = vec![
                "claude".to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--verbose".to_string(),
                "--dangerously-skip-permissions".to_string(),
            ];
            if config.model != "auto" && !config.model.is_empty() {
                parts.push("--model".to_string());
                parts.push(ssh::posix_escape(&config.model));
            }
            if let Some(ref effort) = config.effort {
                parts.push("--effort".to_string());
                parts.push(ssh::posix_escape(effort));
            }
            if let Some(ref resume_id) = config.resume_id {
                parts.push("--resume".to_string());
                parts.push(ssh::posix_escape(resume_id));
            }
            parts.push("-p".to_string());
            parts.push(ssh::posix_escape(&config.prompt));

            let cwd_str = config.cwd.to_string_lossy();
            let remote_script =
                format!("cd {} && {}", ssh::posix_escape(&cwd_str), parts.join(" "));

            let (mut child, askpass) = ssh::spawn_via_ssh(
                host,
                user,
                config.ssh_password.as_deref(),
                &remote_script,
            )?;

            let pid = child.id();
            let stdout = child.stdout.take().ok_or("no stdout")?;
            let stderr = child.stderr.take().ok_or("no stderr")?;

            Ok(SpawnHandle {
                pid,
                reader: Box::new(stdout),
                stderr: Box::new(stderr),
                child,
                _askpass: askpass,
            })
        }
    }
}
```

- [ ] **Step 2: Update `CodexProvider::spawn()` for SSH**

Same pattern. Build the codex CLI args as a string, call `spawn_via_ssh()`:

```rust
SpawnMode::Ssh { ref host, ref user } => {
    let mut parts = vec!["codex".to_string()];
    if let Some(ref sid) = config.resume_id {
        parts.extend([
            "exec".to_string(), "resume".to_string(),
            "--json".to_string(),
            "--dangerously-bypass-approvals-and-sandbox".to_string(),
            "-m".to_string(), ssh::posix_escape(&config.model),
            ssh::posix_escape(sid),
            ssh::posix_escape(&config.prompt),
        ]);
    } else {
        parts.extend([
            "exec".to_string(), "--json".to_string(),
            "--dangerously-bypass-approvals-and-sandbox".to_string(),
            "-m".to_string(), ssh::posix_escape(&config.model),
            ssh::posix_escape(&config.prompt),
        ]);
    }

    let cwd_str = config.cwd.to_string_lossy();
    let remote_script =
        format!("cd {} && {}", ssh::posix_escape(&cwd_str), parts.join(" "));

    let (mut child, askpass) = ssh::spawn_via_ssh(
        host, user, config.ssh_password.as_deref(), &remote_script,
    )?;

    let pid = child.id();
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;

    Ok(SpawnHandle {
        pid,
        reader: Box::new(stdout),
        stderr: Box::new(stderr),
        child,
        _askpass: askpass,
    })
}
```

- [ ] **Step 3: Update `OpenCodeProvider::spawn()` for SSH**

Same pattern. Build the opencode CLI args, call `spawn_via_ssh()`:

```rust
SpawnMode::Ssh { ref host, ref user } => {
    let mut parts = vec![
        "opencode".to_string(),
        "run".to_string(),
        "--format".to_string(), "json".to_string(),
        "-m".to_string(), ssh::posix_escape(&config.model),
    ];

    let cwd_str = config.cwd.to_string_lossy();
    parts.extend(["--dir".to_string(), ssh::posix_escape(&cwd_str)]);

    if let Some(ref sid) = config.resume_id {
        parts.extend([
            "--continue".to_string(),
            "-s".to_string(),
            ssh::posix_escape(sid),
        ]);
    }

    parts.push(ssh::posix_escape(&config.prompt));

    // For SSH, env vars must be set remotely — prepend exports
    let mut env_prefix = String::new();
    for (k, v) in &config.extra_env {
        env_prefix.push_str(&format!("export {}={} && ", k, ssh::posix_escape(v)));
    }

    let remote_script = format!("{env_prefix}{}", parts.join(" "));

    let (mut child, askpass) = ssh::spawn_via_ssh(
        host, user, config.ssh_password.as_deref(), &remote_script,
    )?;

    let pid = child.id();
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;

    Ok(SpawnHandle {
        pid,
        reader: Box::new(stdout),
        stderr: Box::new(stderr),
        child,
        _askpass: askpass,
    })
}
```

- [ ] **Step 4: Run tests, verify compilation**

```bash
cd tauri && cargo test -- --nocapture
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add tauri/src/providers/claude.rs tauri/src/providers/codex.rs tauri/src/providers/opencode.rs
git commit -m "feat: make all providers SSH-aware via spawn_via_ssh"
```

---

## Task 5: Wire SSH through session_manager and IPC

**Files:**
- Modify: `tauri/src/services/session_manager.rs`
- Modify: `tauri/src/ipc/session.rs`
- Modify: `tauri/src/lib.rs`

- [ ] **Step 1: Add `ssh_password` to `ActiveSession`**

In `session_manager.rs`, update the `ActiveSession` struct:

```rust
struct ActiveSession {
    session: Session,
    pub claude_session_id: Option<String>,
    pub effort: Option<String>,
    pub api_key: Option<String>,
    /// SSH password held in memory (never in DB). Reused for follow-up messages.
    pub ssh_password: Option<String>,
}
```

- [ ] **Step 2: Update `init_session` to accept SSH params**

Change the `init_session` signature to accept SSH fields:

```rust
pub fn init_session(
    &mut self,
    project_path: &str,
    session_name: Option<&str>,
    permission_mode: &str,
    model: Option<&str>,
    use_worktree: bool,
    provider: Option<&str>,
    ssh_host: Option<&str>,
    ssh_user: Option<&str>,
    ssh_password: Option<String>,
) -> Result<Session, String> {
```

Update the `db.create_session` call to pass `ssh_host` and `ssh_user`:

```rust
let session_id = self
    .db
    .create_session(
        Some(project.id),
        session_name,
        project_path,
        permission_mode,
        model,
        provider,
        ssh_host,
        ssh_user,
    )
    .map_err(|e| e.to_string())?;
```

Add worktree guard — worktrees don't apply to SSH sessions:

```rust
let use_worktree = use_worktree && ssh_host.is_none();
```

Update the Session construction:

```rust
ssh_host: ssh_host.map(|s| s.to_string()),
ssh_user: ssh_user.map(|s| s.to_string()),
```

Update the ActiveSession insertion to include `ssh_password`:

```rust
self.active.insert(
    session_id,
    ActiveSession {
        session: session.clone(),
        claude_session_id: None,
        effort: None,
        api_key: None,
        ssh_password,
    },
);
```

- [ ] **Step 3: Update `do_spawn` to build SpawnMode from session SSH fields**

In the data extraction block at the start of `do_spawn()`, after extracting `extra_env`, also extract SSH info:

```rust
let spawn_mode = match (a.session.ssh_host.clone(), a.session.ssh_user.clone()) {
    (Some(host), Some(user)) => crate::services::ssh::SpawnMode::Ssh { host, user },
    (Some(host), None) => {
        eprintln!(
            "[orbit] session {session_id}: ssh_host={host:?} set but ssh_user is missing — \
             falling back to local spawn."
        );
        crate::services::ssh::SpawnMode::Local
    }
    _ => crate::services::ssh::SpawnMode::Local,
};
let ssh_password = a.ssh_password.clone();
```

Then pass these to `ProviderSpawnConfig`:

```rust
let spawn_config = ProviderSpawnConfig {
    session_id,
    cwd: std::path::PathBuf::from(&cwd),
    model,
    prompt,
    resume_id,
    extra_env,
    effort,
    spawn_mode,
    ssh_password,
};
```

- [ ] **Step 4: Update `send_message` to restore ssh_password**

In `send_message()`, when re-creating an `ActiveSession` from DB (the branch that handles app restart), set `ssh_password: None` since we can't recover the password after restart:

```rust
m.active.insert(
    session_id,
    ActiveSession {
        session,
        claude_session_id,
        effort: None,
        api_key: None,
        ssh_password: None,
    },
);
```

- [ ] **Step 5: Update `create_session` in `ipc/session.rs` to accept SSH params**

```rust
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn create_session(
    project_path: String,
    prompt: String,
    model: Option<String>,
    permission_mode: Option<String>,
    session_name: Option<String>,
    use_worktree: Option<bool>,
    provider: Option<String>,
    ssh_host: Option<String>,
    ssh_user: Option<String>,
    ssh_password: Option<String>,
    state: State<SessionState>,
    registry: State<ProviderRegistryState>,
    app: AppHandle,
) -> Result<Session, IpcError> {
    let mode = permission_mode.unwrap_or_else(|| "ignore".to_string());

    let session = {
        let mut m = state.write();
        m.init_session(
            &project_path,
            session_name.as_deref(),
            &mode,
            model.as_deref(),
            use_worktree.unwrap_or(false),
            provider.as_deref(),
            ssh_host.as_deref(),
            ssh_user.as_deref(),
            ssh_password,
        )?
    };

    // ... rest unchanged
```

- [ ] **Step 6: Add `test_ssh` command in `ipc/session.rs`**

```rust
use crate::services::ssh::{self, SshTestResult};

/// Test SSH connectivity to a remote host without creating a session.
#[tauri::command]
pub fn test_ssh(host: String, user: String, password: Option<String>) -> SshTestResult {
    ssh::test_ssh_connection(&host, &user, password.as_deref())
}
```

- [ ] **Step 7: Register `test_ssh` in `lib.rs`**

Add to the `invoke_handler` in `tauri/src/lib.rs`:

```rust
ipc::session::test_ssh,
```

- [ ] **Step 8: Run tests, verify compilation**

```bash
cd tauri && cargo test -- --nocapture
```

Expected: All tests PASS.

- [ ] **Step 9: Commit**

```bash
git add tauri/src/services/session_manager.rs tauri/src/ipc/session.rs tauri/src/lib.rs
git commit -m "feat: wire SSH params through session manager, IPC, and lib registration"
```

---

## Task 6: Frontend — add SSH fields to types, tauri.ts, and stores

**Files:**
- Modify: `ui/lib/stores/sessions.ts`
- Modify: `ui/lib/tauri.ts`
- Modify: `ui/lib/mock/tauri-mock.ts`
- Modify: `ui/lib/stores/sessions.test.ts`

- [ ] **Step 1: Add SSH fields to Session interface in `sessions.ts`**

Add after `miniLog`:

```typescript
export interface Session {
  // ... existing fields ...
  miniLog: MiniLogEntry[] | null;
  sshHost: string | null;
  sshUser: string | null;
  subagents?: SubagentInfo[];
  createdAt: string;
  updatedAt: string;
}
```

- [ ] **Step 2: Add SSH fields to `CreateSessionOptions` and `createSession()` in `tauri.ts`**

```typescript
export interface CreateSessionOptions {
  projectPath: string;
  prompt: string;
  model?: string;
  permissionMode?: 'ignore' | 'approve';
  sessionName?: string;
  useWorktree?: boolean;
  provider?: string;
  sshHost?: string;
  sshUser?: string;
  sshPassword?: string;
}

export interface SshTestResult {
  ok: boolean;
  latencyMs?: number;
  error?: string;
}

export async function createSession(opts: CreateSessionOptions): Promise<Session> {
  return await invoke('create_session', {
    projectPath: opts.projectPath,
    prompt: opts.prompt,
    model: opts.model ?? null,
    permissionMode: opts.permissionMode ?? 'ignore',
    sessionName: opts.sessionName ?? null,
    useWorktree: opts.useWorktree ?? false,
    provider: opts.provider ?? 'claude-code',
    sshHost: opts.sshHost ?? null,
    sshUser: opts.sshUser ?? null,
    sshPassword: opts.sshPassword ?? null,
  });
}

export async function testSsh(
  host: string,
  user: string,
  password?: string
): Promise<SshTestResult> {
  return await invoke('test_ssh', {
    host,
    user,
    password: password ?? null,
  });
}
```

- [ ] **Step 3: Update mock to include SSH fields**

In `ui/lib/mock/tauri-mock.ts`, add `sshHost: null, sshUser: null` to every mock session object.

Also add `test_ssh` handler:

```typescript
case 'test_ssh':
  return { ok: true, latencyMs: 42, error: null };
```

- [ ] **Step 4: Update `sessions.test.ts`**

If there's a `makeSession()` factory, add `sshHost: null, sshUser: null` to it.

- [ ] **Step 5: Run TypeScript checks**

```bash
npx svelte-check --fail-on-warnings
```

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add ui/lib/stores/sessions.ts ui/lib/tauri.ts ui/lib/mock/tauri-mock.ts ui/lib/stores/sessions.test.ts
git commit -m "feat: add SSH types and testSsh to frontend"
```

---

## Task 7: Frontend — SSH UI in NewSessionModal

**Files:**
- Modify: `ui/components/NewSessionModal.svelte`

- [ ] **Step 1: Add SSH state variables**

Add to the `<script>` block after the existing state declarations:

```typescript
let sshMode = false;
let sshHost = '';
let sshUser = 'ubuntu';
let sshPassword = '';

let sshTesting = false;
let sshConnectedMs: number | null = null;
```

- [ ] **Step 2: Add SSH test connection handler**

```typescript
import { testSsh, type SshTestResult } from '../lib/tauri';

async function handleTestSsh() {
  if (!sshHost.trim() || !sshUser.trim()) return;
  sshTesting = true;
  sshConnectedMs = null;
  try {
    const result: SshTestResult = await testSsh(
      sshHost.trim(),
      sshUser.trim(),
      sshPassword.trim() || undefined
    );
    if (result.ok) {
      sshConnectedMs = result.latencyMs ?? 0;
    } else {
      error = result.error ?? 'connection failed';
    }
  } catch (e: unknown) {
    error = e instanceof Error ? e.message : String(e);
  } finally {
    sshTesting = false;
  }
}
```

- [ ] **Step 3: Reset SSH test result when fields change**

```typescript
$: if (sshHost || sshUser || sshPassword) sshConnectedMs = null;
```

- [ ] **Step 4: Add SSH validation to submit handler**

In the submit handler, before `createSession()`:

```typescript
if (sshMode && !sshHost.trim()) {
  error = 'ssh host required';
  return;
}
if (sshMode && !sshUser.trim()) {
  error = 'ssh user required';
  return;
}
```

Update the path label for SSH mode:
```typescript
error = sshMode ? 'remote path required' : 'project path required';
```

- [ ] **Step 5: Pass SSH fields to `createSession()`**

```typescript
await createSession({
  // ... existing fields ...
  sshHost: sshMode ? sshHost.trim() : undefined,
  sshUser: sshMode ? sshUser.trim() : undefined,
  sshPassword: sshMode && sshPassword.trim() ? sshPassword.trim() : undefined,
});
```

- [ ] **Step 6: Add local/SSH mode toggle buttons in the template**

Add toggle buttons near the top of the modal form, below the header:

```svelte
<div class="mode-toggle">
  <button
    type="button"
    class:active={!sshMode}
    on:click={() => (sshMode = false)}
  >local</button>
  <button
    type="button"
    class:active={sshMode}
    on:click={() => (sshMode = true)}
    disabled={loading}
  >ssh remote</button>
</div>
```

- [ ] **Step 7: Add SSH connection fields (shown when sshMode is true)**

```svelte
{#if sshMode}
  <div class="field">
    <label for="ssh-host">host</label>
    <div class="ssh-host-row">
      <input
        id="ssh-host"
        type="text"
        bind:value={sshHost}
        placeholder="vps.example.com"
        disabled={loading}
      />
      <button
        type="button"
        class="test-btn"
        on:click={handleTestSsh}
        disabled={loading || sshTesting || !sshHost.trim() || !sshUser.trim()}
      >
        {#if sshTesting}
          testing…
        {:else if sshConnectedMs !== null}
          ✓ {sshConnectedMs}ms
        {:else}
          test
        {/if}
      </button>
    </div>
  </div>

  <div class="field">
    <label for="ssh-user">user</label>
    <input
      id="ssh-user"
      type="text"
      bind:value={sshUser}
      placeholder="ubuntu"
      disabled={loading}
    />
  </div>

  <div class="field">
    <label for="ssh-password">password <span class="optional">(optional — uses SSH key if empty)</span></label>
    <input
      id="ssh-password"
      type="password"
      bind:value={sshPassword}
      placeholder="leave empty for key auth"
      disabled={loading}
    />
  </div>
{/if}
```

- [ ] **Step 8: Add CSS for mode toggle and SSH fields**

```css
.mode-toggle {
  display: flex;
  gap: 0;
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
  margin-bottom: 12px;
}
.mode-toggle button {
  flex: 1;
  padding: 6px 12px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 0.85rem;
  transition: background 0.15s, color 0.15s;
}
.mode-toggle button.active {
  background: var(--accent);
  color: var(--text-primary);
}
.ssh-host-row {
  display: flex;
  gap: 8px;
}
.ssh-host-row input {
  flex: 1;
}
.test-btn {
  white-space: nowrap;
  padding: 6px 12px;
  min-height: 0;
  font-size: 0.85rem;
}
.optional {
  font-weight: normal;
  color: var(--text-secondary);
  font-size: 0.8rem;
}
```

- [ ] **Step 9: Test manually with `npm run dev:mock`**

```bash
npm run dev:mock
```

Verify:
- Mode toggle appears and switches between local/SSH
- SSH fields appear/disappear on toggle
- Test button calls mock and shows ✓
- Form validation shows errors for missing SSH fields
- Provider selector still works alongside SSH toggle

- [ ] **Step 10: Commit**

```bash
git add ui/components/NewSessionModal.svelte
git commit -m "feat: add SSH toggle and connection fields to NewSessionModal"
```

---

## Task 8: Integration test and final verification

**Files:**
- All modified files

- [ ] **Step 1: Run full Rust test suite**

```bash
cd tauri && cargo test -- --nocapture
```

Expected: ALL tests PASS.

- [ ] **Step 2: Run clippy**

```bash
cd tauri && cargo clippy -- -D warnings
```

Expected: Zero warnings.

- [ ] **Step 3: Run frontend checks**

```bash
npx prettier --check "ui/**/*.{ts,svelte,css}" && npx eslint . --max-warnings 0 && npx svelte-check --fail-on-warnings
```

Expected: All pass.

- [ ] **Step 4: Run Vitest**

```bash
npm test
```

Expected: All tests pass.

- [ ] **Step 5: Dev build smoke test**

```bash
npm run tauri:dev
```

Verify:
- App launches without errors
- Can create a local session (existing flow works)
- SSH toggle appears in NewSessionModal
- SSH fields validate correctly
- Provider selector and SSH mode work independently and together

- [ ] **Step 6: Update CHANGELOG.md**

Add entry under the current month:

```markdown
### 04/14 · New — SSH remote sessions for all providers
You can now run sessions on remote servers via SSH. When creating a session, switch
to "ssh remote" mode, enter the host and user (with optional password), and test the
connection before starting. Works with Claude Code, Codex, and OpenCode providers.
```

- [ ] **Step 7: Final commit**

```bash
git add CHANGELOG.md
git commit -m "docs: add SSH remote sessions changelog entry"
```
