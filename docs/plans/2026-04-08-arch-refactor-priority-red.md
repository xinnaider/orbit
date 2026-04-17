# Architecture Refactor — Priority 🔴 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix two correctness bugs (C1 mem::forget, C2 non-atomic DELETE) and split two oversized files into focused modules (commands/ and journal/).

**Architecture:** Tasks 1, 2, 3 touch different files and can run in parallel. Task 4 must run after all three — it splits `journal_reader.rs` and updates imports in files modified by Tasks 2 and 3.

**Tech Stack:** Rust 1.85, Tauri 2, rusqlite 0.31, 86 existing tests must pass after every task.

---

## Dependency Map

```
Task 1 (C2 — database.rs)          ─┐
Task 2 (C1 — spawn_manager +       ─┤──> Task 4 (A2 — journal/ split)
             session_manager)       │
Task 3 (A1 — commands/ split)      ─┘
```

Tasks 1, 2, 3 are fully independent. Task 4 depends on all three completing.

---

## File Map

| File | Action | Task |
|------|--------|------|
| `tauri/src/services/database.rs` | Fix `delete_session` transaction | 1 |
| `tauri/src/services/spawn_manager.rs` | Add `child` field to `SpawnHandle`, remove `mem::forget` | 2 |
| `tauri/src/services/session_manager.rs` | Add `child` param to `reader_loop`, call `wait()` | 2 |
| `tauri/src/commands.rs` | DELETE (replaced by commands/) | 3 |
| `tauri/src/commands/mod.rs` | CREATE — re-exports | 3 |
| `tauri/src/commands/diff.rs` | CREATE — `get_diff`, `get_file_versions` | 3 |
| `tauri/src/commands/files.rs` | CREATE — `list_project_files`, `get_subagent_journal` | 3 |
| `tauri/src/commands/tasks.rs` | CREATE — `get_tasks` | 3 |
| `tauri/src/commands/plugins.rs` | CREATE — `get_slash_commands`, private helpers | 3 |
| `tauri/src/commands/stats.rs` | CREATE — `get_claude_usage_stats`, `get_changelog` | 3 |
| `tauri/src/journal_reader.rs` | DELETE (replaced by journal/) | 4 |
| `tauri/src/journal/mod.rs` | CREATE — re-exports | 4 |
| `tauri/src/journal/state.rs` | CREATE — `JournalState`, `RawEntry`, `detect_pending_approval` | 4 |
| `tauri/src/journal/processor.rs` | CREATE — `process_line` + helpers + tests | 4 |
| `tauri/src/journal/parser.rs` | CREATE — `parse_journal` + helpers + tests | 4 |
| `tauri/src/lib.rs` | Change `pub mod journal_reader` → `pub mod journal` | 4 |
| `tauri/src/services/session_manager.rs` | Update import `journal_reader` → `journal` | 4 |
| `tauri/src/commands/files.rs` | Update import `journal_reader` → `journal` | 4 |

---

## Task 1 — C2: Atomic `delete_session` (parallel)

**Files:**
- Modify: `tauri/src/services/database.rs`

### Context
Currently, `delete_session` runs two `DELETE` statements without a transaction. If the process crashes between them, the session row remains in the DB but its outputs are gone — inconsistent state. Fix: wrap in `BEGIN / COMMIT`.

The existing test `should_delete_session_and_its_outputs_together` already covers the correct behavior. Run it to verify the fix works.

- [ ] **Step 1: Replace `delete_session` in `database.rs`**

Find the `delete_session` function (currently around line 278) and replace it with:

```rust
pub fn delete_session(&self, id: SessionId) -> SqlResult<()> {
    let conn = self.conn.lock().unwrap();
    conn.execute_batch("BEGIN")?;
    conn.execute(
        "DELETE FROM session_outputs WHERE session_id = ?1",
        params![id],
    )?;
    conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
    conn.execute_batch("COMMIT")?;
    Ok(())
}
```

- [ ] **Step 2: Run the database tests**

```bash
cd tauri && cargo test services::database -- --nocapture 2>&1 | tail -10
```

Expected: `test result: ok. 16 passed`.

- [ ] **Step 3: Commit**

```bash
git add tauri/src/services/database.rs
git -c user.email=fernandoschnneider@gmail.com commit -m "fix: wrap delete_session in transaction to prevent partial deletes"
```

---

## Task 2 — C1: Fix `mem::forget` child leak (parallel)

**Files:**
- Modify: `tauri/src/services/spawn_manager.rs`
- Modify: `tauri/src/services/session_manager.rs`

### Context
`spawn_claude` extracts `pid`, `stdout`, `stderr` from a `std::process::Child` then calls `std::mem::forget(child)`. On Unix this creates zombie processes; on Windows it leaks the kernel handle. Fix: include `child` in `SpawnHandle` and move it into `reader_loop`, calling `child.wait()` after reading completes.

`reader_loop` already runs in a background thread (spawned in `send_message`), so blocking on `wait()` at the end has no impact on the UI.

- [ ] **Step 1: Add `child` field to `SpawnHandle` in `spawn_manager.rs`**

Find `pub struct SpawnHandle` (around line 15) and replace it with:

```rust
pub struct SpawnHandle {
    pub pid: u32,
    pub reader: Box<dyn std::io::Read + Send>,
    pub stderr: Box<dyn std::io::Read + Send>,
    pub child: std::process::Child,
}
```

- [ ] **Step 2: Remove `mem::forget`, add `child` to the returned struct in `spawn_claude`**

Find `std::mem::forget(child);` (around line 221) and the `Ok(SpawnHandle { ... })` block immediately after it. Replace both with:

```rust
Ok(SpawnHandle {
    pid,
    reader: Box::new(stdout),
    stderr: Box::new(stderr),
    child,
})
```

- [ ] **Step 3: Verify `spawn_manager.rs` compiles**

```bash
cd tauri && cargo build 2>&1 | grep "^error" | head -10
```

Expected: no errors (or only errors in `session_manager.rs` because `reader_loop` signature hasn't been updated yet — that's fine at this stage).

- [ ] **Step 4: Update `reader_loop` signature in `session_manager.rs`**

Find `fn reader_loop(` (around line 305). Add `mut child: std::process::Child` as the **last** parameter:

```rust
fn reader_loop(
    manager: Arc<Mutex<SessionManager>>,
    app: AppHandle,
    session_id: SessionId,
    reader: Box<dyn std::io::Read + Send>,
    db: Arc<DatabaseService>,
    mut child: std::process::Child,
) {
```

- [ ] **Step 5: Add `child.wait()` at the end of `reader_loop`**

Find the final `let _ = app.emit("session:stopped", ...)` call in `reader_loop`. Add `child.wait()` AFTER it (as the last statement in the function):

```rust
        let _ = app.emit(
            "session:stopped",
            serde_json::json!({ "sessionId": session_id }),
        );

        // Collect exit status — prevents zombie on Unix, releases handle on Windows
        let _ = child.wait();
    }
```

- [ ] **Step 6: Pass `handle.child` when calling `reader_loop` in `do_spawn`**

Find the line (around line 301):
```rust
Self::reader_loop(Arc::clone(&manager), app, session_id, handle.reader, db);
```

Replace with:
```rust
Self::reader_loop(Arc::clone(&manager), app, session_id, handle.reader, db, handle.child);
```

- [ ] **Step 7: Run the full test suite**

```bash
cd tauri && cargo test 2>&1 | grep "test result"
```

Expected: `test result: ok. 86 passed`.

- [ ] **Step 8: Commit**

```bash
git add tauri/src/services/spawn_manager.rs tauri/src/services/session_manager.rs
git -c user.email=fernandoschnneider@gmail.com commit -m "fix: move Child into SpawnHandle to prevent zombie processes"
```

---

## Task 3 — A1: Split `commands.rs` → `commands/` (parallel)

**Files:**
- Delete: `tauri/src/commands.rs`
- Create: `tauri/src/commands/mod.rs`
- Create: `tauri/src/commands/diff.rs`
- Create: `tauri/src/commands/files.rs`
- Create: `tauri/src/commands/tasks.rs`
- Create: `tauri/src/commands/plugins.rs`
- Create: `tauri/src/commands/stats.rs`

### Context
`commands.rs` is 499 lines with 8 unrelated command domains. Splitting into a `commands/` directory makes each domain a focused file. `lib.rs` already uses `commands::fn_name` so it needs **no changes** — a `commands/` directory with `mod.rs` is resolved identically to a `commands.rs` file by Rust.

`commands.rs` currently references `crate::journal_reader::parse_journal` in `get_subagent_journal`. **Keep the import as `crate::journal_reader`** in this task — Task 4 will update it when `journal_reader.rs` is replaced.

- [ ] **Step 1: Create `tauri/src/commands/diff.rs`**

```rust
use crate::diff_builder;
use crate::models::*;

#[tauri::command]
pub fn get_diff(
    session_id: String,
    file_hash: String,
    from_version: u32,
    to_version: u32,
) -> Result<DiffResult, String> {
    diff_builder::build_diff(&session_id, &file_hash, from_version, to_version)
        .ok_or_else(|| "Could not build diff".to_string())
}

#[tauri::command]
pub fn get_file_versions(session_id: String) -> Vec<diff_builder::FileVersionInfo> {
    diff_builder::get_file_versions(&session_id)
}
```

- [ ] **Step 2: Create `tauri/src/commands/files.rs`**

Copy `list_project_files` and `get_subagent_journal` from `commands.rs`. Keep `use crate::journal_reader;` as-is (Task 4 updates it):

```rust
use crate::journal_reader;
use crate::models::*;

#[tauri::command]
pub fn get_subagent_journal(session_id: String, subagent_id: String) -> Vec<JournalEntry> {
    let projects_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("projects"),
        None => return vec![],
    };

    let entries = match std::fs::read_dir(&projects_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for project_entry in entries.flatten() {
        let jsonl_path = project_entry
            .path()
            .join(&session_id)
            .join("subagents")
            .join(format!("{}.jsonl", &subagent_id));

        if jsonl_path.exists() {
            let state = journal_reader::parse_journal(&jsonl_path, 0, None);
            let mut result = state.entries;
            for entry in &mut result {
                entry.session_id = subagent_id.clone();
            }
            return result;
        }
    }

    vec![]
}

#[tauri::command]
pub fn list_project_files(cwd: String) -> Vec<String> {
    use ignore::WalkBuilder;

    let mut files = Vec::new();
    let walker = WalkBuilder::new(&cwd)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .max_depth(Some(12))
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        if let Ok(rel) = entry.path().strip_prefix(&cwd) {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if !rel_str.is_empty() {
                files.push(rel_str.to_string());
                if files.len() >= 5000 {
                    break;
                }
            }
        }
    }

    files.sort();
    files
}
```

- [ ] **Step 3: Create `tauri/src/commands/tasks.rs`**

Copy `get_tasks` from `commands.rs`:

```rust
use crate::models::*;

#[tauri::command]
pub fn get_tasks(
    session_id: String,
    state: tauri::State<crate::ipc::session::SessionState>,
) -> Vec<TaskItem> {
    let id: i64 = match session_id.parse() {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let outputs = {
        let m = state.0.lock().unwrap();
        match m.db.get_outputs(id) {
            Ok(o) => o,
            Err(_) => return vec![],
        }
    };

    let mut last_todos: Option<Vec<TaskItem>> = None;

    for raw in &outputs {
        let val: serde_json::Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if val.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }

        let content = match val
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_array())
        {
            Some(c) => c,
            None => continue,
        };

        for block in content {
            if block.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
                continue;
            }
            if block.get("name").and_then(|n| n.as_str()) != Some("TodoWrite") {
                continue;
            }

            let todos_val = match block.get("input").and_then(|i| i.get("todos")) {
                Some(t) => t,
                None => continue,
            };

            let todos: Vec<TaskItem> = todos_val
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .enumerate()
                .filter_map(|(idx, t)| {
                    let id_str = t
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| (idx + 1).to_string());
                    let subject = t
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let status = t
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("pending")
                        .to_string();
                    if status == "deleted" || subject.is_empty() {
                        return None;
                    }
                    let active_form = t
                        .get("activeForm")
                        .or_else(|| t.get("active_form"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    Some(TaskItem {
                        id: id_str,
                        subject,
                        description: String::new(),
                        active_form,
                        status,
                        blocks: vec![],
                        blocked_by: vec![],
                    })
                })
                .collect();

            last_todos = Some(todos);
        }
    }

    last_todos.unwrap_or_default()
}
```

- [ ] **Step 4: Create `tauri/src/commands/plugins.rs`**

Copy `frontmatter_field`, `scan_plugin`, and `get_slash_commands` from `commands.rs`:

```rust
use std::path::Path;

use crate::models::*;

/// Extract the `name` or `description` value from YAML frontmatter.
fn frontmatter_field(content: &str, field: &str) -> Option<String> {
    let body = content.strip_prefix("---")?;
    let end = body.find("---")?;
    let fm = &body[..end];
    for line in fm.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(field) {
            let rest = rest.trim_start();
            if let Some(val) = rest.strip_prefix(':') {
                let val = val.trim().trim_matches('"');
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

/// Scan a plugin directory for skills, commands and agents.
fn scan_plugin(install_path: &Path, plugin_name: &str, out: &mut Vec<SlashCommand>) {
    let skills_dir = install_path.join("skills");
    if let Ok(entries) = std::fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            let skill_file = entry.path().join("SKILL.md");
            if skill_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&skill_file) {
                    let name = frontmatter_field(&content, "name")
                        .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
                    let desc = frontmatter_field(&content, "description").unwrap_or_default();
                    let desc_short = if desc.len() > 80 {
                        format!("{}...", &desc[..77])
                    } else {
                        desc
                    };
                    out.push(SlashCommand {
                        cmd: format!("/{}:{}", plugin_name, name),
                        desc: desc_short,
                        category: "skill".to_string(),
                    });
                }
            }
        }
    }

    let cmds_dir = install_path.join("commands");
    if let Ok(entries) = std::fs::read_dir(&cmds_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let desc = frontmatter_field(&content, "description").unwrap_or_default();
                    let desc_lower = desc.to_lowercase();
                    if desc_lower.contains("deprecated") {
                        continue;
                    }
                    let stem = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let desc_short = if desc.len() > 80 {
                        format!("{}...", &desc[..77])
                    } else {
                        desc
                    };
                    out.push(SlashCommand {
                        cmd: format!("/{}", stem),
                        desc: desc_short,
                        category: "command".to_string(),
                    });
                }
            }
        }
    }

    let agents_dir = install_path.join("agents");
    if let Ok(entries) = std::fs::read_dir(&agents_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let name = frontmatter_field(&content, "name").unwrap_or_else(|| {
                        path.file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string()
                    });
                    let desc = frontmatter_field(&content, "description").unwrap_or_default();
                    let desc_short = if desc.len() > 80 {
                        format!("{}...", &desc[..77])
                    } else {
                        desc
                    };
                    out.push(SlashCommand {
                        cmd: format!("/{}:{}", plugin_name, name),
                        desc: desc_short,
                        category: "agent".to_string(),
                    });
                }
            }
        }
    }
}

#[tauri::command]
pub fn get_slash_commands() -> Vec<SlashCommand> {
    let mut result: Vec<SlashCommand> = Vec::new();

    let builtins = [
        ("/help", "Show help"),
        ("/compact", "Compact conversation context"),
        ("/clear", "Clear conversation"),
        ("/cost", "Show token usage and cost"),
        ("/model", "Switch model"),
        ("/fast", "Toggle fast output mode"),
        ("/permissions", "Manage tool permissions"),
        ("/status", "Show session status"),
        ("/memory", "Edit CLAUDE.md memory"),
        ("/review", "Review conversation"),
        ("/doctor", "Check installation health"),
        ("/init", "Initialize CLAUDE.md for project"),
        ("/login", "Log in to Anthropic"),
        ("/logout", "Log out"),
        ("/terminal-setup", "Setup terminal integration"),
        ("/vim", "Toggle vim mode"),
    ];
    for (cmd, desc) in builtins {
        result.push(SlashCommand {
            cmd: cmd.to_string(),
            desc: desc.to_string(),
            category: "built-in".to_string(),
        });
    }

    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return result,
    };

    let plugins_file = home
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");
    let content = match std::fs::read_to_string(&plugins_file) {
        Ok(c) => c,
        Err(_) => return result,
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return result,
    };

    if let Some(plugins) = json.get("plugins").and_then(|p| p.as_object()) {
        for (key, entries) in plugins {
            let plugin_name = key.split('@').next().unwrap_or(key);
            if let Some(arr) = entries.as_array() {
                for entry in arr {
                    if let Some(install_path) = entry.get("installPath").and_then(|p| p.as_str()) {
                        scan_plugin(Path::new(install_path), plugin_name, &mut result);
                    }
                }
            }
        }
    }

    let mut seen = std::collections::HashSet::new();
    result.retain(|c| seen.insert(c.cmd.clone()));

    result
}
```

- [ ] **Step 5: Create `tauri/src/commands/stats.rs`**

Copy `ClaudeUsageStats`, `get_claude_usage_stats`, `days_to_date`, `is_leap`, and `get_changelog` from `commands.rs`:

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeUsageStats {
    pub weekly_tokens: u64,
    pub today_tokens: u64,
    pub weekly_messages: u64,
    pub today_messages: u64,
}

#[tauri::command]
pub fn get_claude_usage_stats() -> ClaudeUsageStats {
    let empty = ClaudeUsageStats {
        weekly_tokens: 0,
        today_tokens: 0,
        weekly_messages: 0,
        today_messages: 0,
    };

    let stats_path = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("stats-cache.json"),
        None => return empty,
    };

    let content = match std::fs::read_to_string(&stats_path) {
        Ok(c) => c,
        Err(_) => return empty,
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return empty,
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let secs_per_day: u64 = 86400;
    let today_days = now / secs_per_day;
    let today = days_to_date(today_days);
    let week_start = days_to_date(today_days.saturating_sub(6));

    let mut weekly_tokens: u64 = 0;
    let mut today_tokens: u64 = 0;
    let mut weekly_messages: u64 = 0;
    let mut today_messages: u64 = 0;

    if let Some(arr) = json.get("dailyModelTokens").and_then(|v| v.as_array()) {
        for entry in arr {
            let date = entry.get("date").and_then(|d| d.as_str()).unwrap_or("");
            if date >= week_start.as_str() && date <= today.as_str() {
                if let Some(by_model) = entry.get("tokensByModel").and_then(|v| v.as_object()) {
                    let total: u64 = by_model.values().filter_map(|v| v.as_u64()).sum();
                    weekly_tokens += total;
                    if date == today.as_str() {
                        today_tokens = total;
                    }
                }
            }
        }
    }

    if let Some(arr) = json.get("dailyActivity").and_then(|v| v.as_array()) {
        for entry in arr {
            let date = entry.get("date").and_then(|d| d.as_str()).unwrap_or("");
            if date >= week_start.as_str() && date <= today.as_str() {
                let msgs = entry
                    .get("messageCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                weekly_messages += msgs;
                if date == today.as_str() {
                    today_messages = msgs;
                }
            }
        }
    }

    ClaudeUsageStats {
        weekly_tokens,
        today_tokens,
        weekly_messages,
        today_messages,
    }
}

fn days_to_date(days: u64) -> String {
    let mut remaining = days;
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for &md in &month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        month += 1;
    }
    let day = remaining + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[tauri::command]
pub fn get_changelog() -> String {
    include_str!("../../../CHANGELOG.md").to_string()
}
```

**Important:** `get_changelog` uses `include_str!("../../CHANGELOG.md")` in `commands.rs`. After moving to `commands/stats.rs`, the path changes to `include_str!("../../../CHANGELOG.md")` (one extra `../` because the file is now one level deeper).

- [ ] **Step 6: Create `tauri/src/commands/mod.rs`**

```rust
pub mod diff;
pub mod files;
pub mod plugins;
pub mod stats;
pub mod tasks;

pub use diff::{get_diff, get_file_versions};
pub use files::{get_subagent_journal, list_project_files};
pub use plugins::get_slash_commands;
pub use stats::{get_changelog, get_claude_usage_stats};
pub use tasks::get_tasks;
```

- [ ] **Step 7: Delete `tauri/src/commands.rs`**

```bash
rm tauri/src/commands.rs
```

- [ ] **Step 8: Build to verify no errors**

```bash
cd tauri && cargo build 2>&1 | grep "^error" | head -20
```

Expected: no errors. If you see `include_str! path` errors, fix the `../../../CHANGELOG.md` path in `stats.rs`.

- [ ] **Step 9: Run the full test suite**

```bash
cd tauri && cargo test 2>&1 | grep "test result"
```

Expected: `test result: ok. 86 passed`.

- [ ] **Step 10: Commit**

```bash
git add tauri/src/commands/ && git rm tauri/src/commands.rs
git -c user.email=fernandoschnneider@gmail.com commit -m "refactor: split commands.rs into commands/ module"
```

---

## Task 4 — A2: Split `journal_reader.rs` → `journal/` (AFTER tasks 1, 2, 3)

**Files:**
- Delete: `tauri/src/journal_reader.rs`
- Create: `tauri/src/journal/mod.rs`
- Create: `tauri/src/journal/state.rs`
- Create: `tauri/src/journal/processor.rs`
- Create: `tauri/src/journal/parser.rs`
- Modify: `tauri/src/lib.rs`
- Modify: `tauri/src/services/session_manager.rs`
- Modify: `tauri/src/commands/files.rs`

### Context

`journal_reader.rs` is 1522 lines with two distinct paths (`parse_journal` for file replay, `process_line` for live streaming) plus a growing set of helpers. The split makes each concern a dedicated file.

**Call graph that determines the split:**
- `parse_journal` calls `derive_status_from_tail`, `json_contains`, `detect_pending_approval`
- `process_line` calls `extract_tool_target`, `char_boundary`, `truncate_output`, `detect_pending_approval`
- `detect_pending_approval` is shared → goes in `state.rs`
- `derive_status_from_tail` + `json_contains` are private to parser → go in `parser.rs`
- `extract_tool_target` + `char_boundary` + `truncate_output` are private to processor → go in `processor.rs`

**Tests stay with their functions:**
- `process_line_tests` + `helper_tests` → `processor.rs`
- `parse_journal_tests` → `parser.rs`

### Step-by-step

- [ ] **Step 1: Create `tauri/src/journal/state.rs`**

This file contains `JournalState`, its `Default` impl, `RawEntry`, and `detect_pending_approval`.

Read `tauri/src/journal_reader.rs` lines 1–61 (the struct definitions) and lines 432–458 (`detect_pending_approval`). Create `tauri/src/journal/state.rs` with this exact structure:

```rust
use crate::models::*;
use serde::Deserialize;
use serde_json::Value;

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
    // COPY the body of detect_pending_approval verbatim from journal_reader.rs lines 432–458
    todo!()
}
```

Replace the `todo!()` placeholder with the exact function body copied from `journal_reader.rs` lines 432–458.

- [ ] **Step 2: Create `tauri/src/journal/processor.rs`**

This file contains `process_line` and its private helpers, plus the test modules.

Read `journal_reader.rs` lines 368–428 (helpers: `extract_tool_target`, `char_boundary`, `truncate_output`) and lines 585–873 (`process_line` body). Read lines 875–1230 (`process_line_tests`) and lines 1401–1522 (`helper_tests`).

Create `tauri/src/journal/processor.rs`:

```rust
use serde_json::Value;

use crate::models::*;
use super::state::{detect_pending_approval, JournalState, RawEntry};

// COPY extract_tool_target verbatim from journal_reader.rs lines 368–411
// COPY char_boundary verbatim from journal_reader.rs lines 413–423
// COPY truncate_output verbatim from journal_reader.rs lines 424–431
// COPY process_line verbatim from journal_reader.rs lines 585–873

#[cfg(test)]
mod process_line_tests {
    use super::*;
    use crate::test_utils::{
        assistant_end_turn, assistant_text, assistant_thinking, assistant_tool_use,
        assistant_with_tokens, progress_line, system_stop_hook, tool_result, user_text, TestCase,
    };

    // COPY the entire body of process_line_tests from journal_reader.rs lines 876–1229
}

#[cfg(test)]
mod helper_tests {
    use super::*;
    use crate::journal::state::detect_pending_approval;
    use crate::test_utils::TestCase;

    // COPY the entire body of helper_tests from journal_reader.rs lines 1402–1521
}
```

**Important:** The `helper_tests` module needs `use crate::journal::state::detect_pending_approval;` because `detect_pending_approval` lives in `state.rs`, not `processor.rs`. Add this import inside the `helper_tests` mod block.

- [ ] **Step 3: Create `tauri/src/journal/parser.rs`**

This file contains `parse_journal` and its private helpers, plus the parse_journal tests.

Read `journal_reader.rs` lines 62–367 (`parse_journal` body), lines 459–465 (`json_contains`), and lines 466–583 (`derive_status_from_tail`). Read lines 1231–1400 (`parse_journal_tests`).

Create `tauri/src/journal/parser.rs`:

```rust
use std::fs;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

use crate::models::*;
use super::state::{detect_pending_approval, JournalState, RawEntry};

// COPY parse_journal verbatim from journal_reader.rs lines 62–367
// COPY json_contains verbatim from journal_reader.rs lines 459–465
// COPY derive_status_from_tail verbatim from journal_reader.rs lines 466–583

#[cfg(test)]
mod parse_journal_tests {
    use super::*;
    use crate::test_utils::{
        append_jsonl, assistant_end_turn, assistant_text, assistant_thinking, assistant_tool_use,
        tool_result, write_jsonl, TestCase,
    };

    // COPY the entire body of parse_journal_tests from journal_reader.rs lines 1232–1399
}
```

- [ ] **Step 4: Create `tauri/src/journal/mod.rs`**

```rust
pub mod parser;
pub mod processor;
pub mod state;

pub use parser::parse_journal;
pub use processor::process_line;
pub use state::JournalState;
```

- [ ] **Step 5: Update `tauri/src/lib.rs`**

Find line 5: `pub mod journal_reader;`
Replace with: `pub mod journal;`

- [ ] **Step 6: Update import in `tauri/src/services/session_manager.rs`**

Find line 7: `use crate::journal_reader::{process_line, JournalState};`
Replace with: `use crate::journal::{process_line, JournalState};`

- [ ] **Step 7: Update import in `tauri/src/commands/files.rs`**

Find: `use crate::journal_reader;`
Replace with: `use crate::journal;`

Find: `journal_reader::parse_journal`
Replace with: `journal::parse_journal`

- [ ] **Step 8: Delete `tauri/src/journal_reader.rs`**

```bash
rm tauri/src/journal_reader.rs
```

- [ ] **Step 9: Build to verify no errors**

```bash
cd tauri && cargo build 2>&1 | grep "^error" | head -20
```

Common errors and fixes:
- `cannot find function detect_pending_approval` in `processor.rs` tests → add `use crate::journal::state::detect_pending_approval;` inside the `helper_tests` mod
- `use of undeclared type RawEntry` → add `use super::state::RawEntry;` at top of processor.rs or parser.rs
- `unresolved import journal_reader` → check lib.rs was updated in Step 5
- `include_str` compile errors in stats.rs → check path is `../../../CHANGELOG.md`

- [ ] **Step 10: Run the full test suite**

```bash
cd tauri && cargo test 2>&1 | grep "test result"
```

Expected: `test result: ok. 86 passed`.

- [ ] **Step 11: Commit**

```bash
git add tauri/src/journal/ tauri/src/lib.rs tauri/src/services/session_manager.rs tauri/src/commands/files.rs
git rm tauri/src/journal_reader.rs
git -c user.email=fernandoschnneider@gmail.com commit -m "refactor: split journal_reader.rs into journal/ module"
```

---

## Self-Review

**Spec coverage:**
- C1 (mem::forget) → Task 2 ✓
- C2 (non-atomic DELETE) → Task 1 ✓
- A1 (commands/ split) → Task 3 ✓
- A2 (journal/ split) → Task 4 ✓
- 86 tests must pass after each task → verified in each task's test step ✓

**Placeholder scan:**
- Task 4 Step 1 has `todo!()` placeholder — intentional, instructs implementer to paste function body from journal_reader.rs. The instruction is explicit and complete. ✓

**Type consistency:**
- `SpawnHandle.child: std::process::Child` used consistently across Tasks 2 and 4 ✓
- `reader_loop` gains `mut child: std::process::Child` as last param — call site updated in same step ✓
- `journal::` prefix used consistently in all updated imports ✓
- `commands/stats.rs` path to CHANGELOG: `../../../CHANGELOG.md` (was `../../CHANGELOG.md`) — noted in Step 5 ✓
