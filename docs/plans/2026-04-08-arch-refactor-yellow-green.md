# Architecture Refactor — Yellow + Green Items

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the 🟡 (performance/maintainability) and 🟢 (refinement) items from `docs/architecture-review-rust.md`, covering 12 improvements across the Rust backend.

**Architecture:** Eight independent-ish tasks targeting database (WAL mode, batched writes), session manager (lazy restore, RwLock, lock helper), journal module (parse_journal reuses process_line), models (enum status, default entry), and IPC (typed errors). Each task produces a passing build with tests.

**Tech Stack:** Rust 1.85, rusqlite 0.31, Tauri 2, `thiserror = "1"` (new, Task 8 only)

**Prerequisite:** PR #13 (`orbit/arch-refactor-red`) must be merged into `dev` before starting Task 5 (M1) and Task 8 (A6). These tasks reference `journal/processor.rs`, `journal/parser.rs`, and `commands/plugins.rs` which only exist after that merge. Tasks 1–4 and 6–7 can start on the current `dev` branch immediately.

**Dependency order:**
```
Task 1 (P5+C3+P2)  → no dependencies
Task 2 (A5+A7+T1)  → no dependencies
Task 3 (M5)         → no dependencies
Task 4 (P3)         → no dependencies
Task 5 (M1)         → arch-refactor-red must be merged
Task 6 (P4)         → no dependencies
Task 7 (P8)         → after Task 2 (A7 introduces lock() pattern P8 adapts to RwLock)
Task 8 (A6)         → after arch-refactor-red merged; after Tasks 1–7 (touches every file)
```

---

### Task 1: P5 + C3 + P2 — WAL mode, UTF-8 truncation fix, zero-alloc rate limit

**Files:**
- Modify: `tauri/src/services/database.rs:30` (P5: WAL pragma)
- Modify: `tauri/src/commands.rs:76-159` (C3: unsafe `&desc[..77]` → safe truncation) — will be `commands/plugins.rs` after arch-refactor-red
- Modify: `tauri/src/services/session_manager.rs:226-247` and `601-609` (P2: zero-alloc rate limit check)

**Why P5:** WAL mode lets reads proceed concurrently with one writer — 1 line, immediate gain.  
**Why C3:** `&desc[..77]` panics at runtime if byte 77 lands inside a multi-byte character (emoji, accented letter). Affects `get_slash_commands`.  
**Why P2:** `is_rate_limit_line` allocates a `String` via `to_lowercase()` on every line of Claude's stdout. Zero-alloc byte-level comparison eliminates this entirely.

- [ ] **Step 1.1: Write failing test for UTF-8 truncation**

In `tauri/src/commands.rs` tests (or `commands/plugins.rs` tests after arch-refactor-red), add:

```rust
#[test]
fn should_not_panic_on_multibyte_truncation() {
    let mut t = TestCase::new("should_not_panic_on_multibyte_truncation");
    // "é" is 2 bytes — placing it at index 77 would panic with &desc[..77]
    let long_desc = format!("{}é{}", "x".repeat(76), "y".repeat(10));
    t.phase("Act");
    let result = truncate_desc(&long_desc, 77);
    t.phase("Assert");
    t.ok("does not panic", true);
    t.ok("result is valid UTF-8", std::str::from_utf8(result.as_bytes()).is_ok());
}
```

Run: `cd tauri && cargo test should_not_panic_on_multibyte_truncation 2>&1 | tail -20`  
Expected: FAIL — `truncate_desc` does not exist yet.

- [ ] **Step 1.2: Add WAL mode pragma to database.rs**

In `tauri/src/services/database.rs`, inside `migrate()`, add WAL as the first statement before the ALTER TABLE lines:

```rust
fn migrate(&self) -> SqlResult<()> {
    let conn = self.conn.lock().unwrap();
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;  // ← ADD THIS
    // Run schema migrations (errors ignored — column may already exist)
    let _ = conn.execute_batch("ALTER TABLE sessions ADD COLUMN claude_session_id TEXT");
    // ... rest unchanged
```

- [ ] **Step 1.3: Add safe truncation helper**

In `tauri/src/commands.rs` (or `commands/plugins.rs`), add before `scan_plugin`:

```rust
/// Truncate `s` to at most `max_bytes` bytes at a UTF-8 character boundary.
/// Appends "..." if truncated.
fn truncate_desc(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let boundary = (0..=max_bytes)
        .rev()
        .find(|&i| s.is_char_boundary(i))
        .unwrap_or(0);
    format!("{}...", &s[..boundary])
}
```

- [ ] **Step 1.4: Replace the three unsafe truncations in scan_plugin**

In the same file, replace every:
```rust
let desc_short = if desc.len() > 80 {
    format!("{}...", &desc[..77])
} else {
    desc
};
```
With:
```rust
let desc_short = truncate_desc(&desc, 77);
```

There are exactly 3 occurrences (skills block ~line 86, commands block ~line 117, agents block ~line 145).

- [ ] **Step 1.5: Add zero-alloc rate limit check helper**

In `tauri/src/services/session_manager.rs`, replace the `is_rate_limit_line` function (around line 601) with:

```rust
/// Check if a JSON line from Claude's stdout indicates a rate limit error.
/// Uses byte-level case-insensitive windows — zero allocation.
fn is_rate_limit_line(line: &str) -> bool {
    let has_rate = ascii_ci_contains(line, "rate_limit")
        || ascii_ci_contains(line, "rate limit")
        || ascii_ci_contains(line, "overloaded");
    let has_error = ascii_ci_contains(line, "\"type\":\"error\"")
        || ascii_ci_contains(line, "\"type\": \"error\"")
        || ascii_ci_contains(line, "error_type")
        || ascii_ci_contains(line, "\"subtype\":\"error\"");
    has_rate && has_error
}

/// Case-insensitive substring search without allocation (ASCII only).
fn ascii_ci_contains(haystack: &str, needle: &str) -> bool {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if h.len() < n.len() {
        return false;
    }
    h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
}
```

- [ ] **Step 1.6: Fix the stderr thread allocation**

In the stderr reader thread (around line 226-247 in session_manager.rs), replace:
```rust
let trimmed = line.trim().to_lowercase();
if trimmed.contains("rate limit")
    || trimmed.contains("rate_limit")
    || trimmed.contains("overloaded")
```
With:
```rust
let trimmed = line.trim();
if ascii_ci_contains(trimmed, "rate limit")
    || ascii_ci_contains(trimmed, "rate_limit")
    || ascii_ci_contains(trimmed, "overloaded")
```

- [ ] **Step 1.7: Run tests**

```bash
cd tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass including `should_not_panic_on_multibyte_truncation`.

- [ ] **Step 1.8: Commit**

```bash
git add tauri/src/services/database.rs tauri/src/commands.rs tauri/src/services/session_manager.rs
git commit -m "perf: WAL mode, zero-alloc rate limit check, safe UTF-8 truncation (C3+P2+P5)"
```

---

### Task 2: A5 + A7 + T1 — JournalEntry::default(), SessionState::lock() with poison recovery

**Files:**
- Modify: `tauri/src/models.rs` (A5: `impl Default for JournalEntry`)
- Modify: `tauri/src/ipc/session.rs` (A7: `SessionState::lock()` method)
- Modify: `tauri/src/ipc/project.rs` (T1: replace `.unwrap()`)
- Modify: `tauri/src/services/session_manager.rs` (T1: internal `.unwrap()` calls)
- Modify: `tauri/src/services/database.rs` (T1: `conn.lock().unwrap()` calls)
- Modify: `tauri/src/journal_reader.rs` (A5: update 14 JournalEntry construction sites) — will be `journal/processor.rs` + `journal/parser.rs` after arch-refactor-red

**Why A5:** `JournalEntry` has 11 fields, 9 optional. All 14 construction sites list them explicitly. A future field addition currently breaks 14 call sites — with `Default`, only the field's own `impl` needs updating.  
**Why A7+T1:** A panicking thread holding the `SessionManager` Mutex poisons it. Every subsequent `.unwrap()` re-panics, crashing the app. `unwrap_or_else(|e| e.into_inner())` recovers the guard from a poisoned Mutex.

- [ ] **Step 2.1: Write a failing test**

In `tauri/src/models.rs` tests (or inline in the file), add:

```rust
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
}
```

Run: `cd tauri && cargo test should_construct_journal_entry_with_default 2>&1 | tail -20`  
Expected: FAIL — `JournalEntry` does not implement `Default`.

- [ ] **Step 2.2: Add Default impl for JournalEntry**

In `tauri/src/models.rs`, after the `JournalEntry` struct definition (after line 95), add:

```rust
impl Default for JournalEntry {
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
        }
    }
}
```

- [ ] **Step 2.3: Update the user entry construction in session_manager.rs**

In `session_manager.rs` around line 266-278:

```rust
// BEFORE:
let user_entry = crate::models::JournalEntry {
    session_id: session_id.to_string(),
    timestamp: chrono::Utc::now().to_rfc3339(),
    entry_type: crate::models::JournalEntryType::User,
    text: Some(prompt_text.clone()),
    thinking: None,
    thinking_duration: None,
    tool: None,
    tool_input: None,
    output: None,
    exit_code: None,
    lines_changed: None,
};

// AFTER:
let user_entry = crate::models::JournalEntry {
    session_id: session_id.to_string(),
    timestamp: chrono::Utc::now().to_rfc3339(),
    entry_type: crate::models::JournalEntryType::User,
    text: Some(prompt_text.clone()),
    ..crate::models::JournalEntry::default()
};
```

- [ ] **Step 2.4: Update the 14 JournalEntry construction sites in journal_reader.rs**

In `tauri/src/journal_reader.rs` (or `journal/processor.rs` + `journal/parser.rs` after arch-refactor-red), grep for `JournalEntry {` and apply struct update syntax to each. Every construction that ends with explicit `None` fields becomes `..JournalEntry::default()`.

Example pattern — before:
```rust
JournalEntry {
    session_id: String::new(),
    timestamp: raw.timestamp.clone().unwrap_or_default(),
    entry_type: JournalEntryType::Assistant,
    text: Some(text),
    thinking: None,
    thinking_duration: None,
    tool: None,
    tool_input: None,
    output: None,
    exit_code: None,
    lines_changed: None,
}
```

After:
```rust
JournalEntry {
    timestamp: raw.timestamp.clone().unwrap_or_default(),
    entry_type: JournalEntryType::Assistant,
    text: Some(text),
    ..JournalEntry::default()
}
```

Note: `session_id` in `journal_reader.rs` is usually `String::new()` (filled in later by the caller), matching `Default`. Remove it from the constructor.

- [ ] **Step 2.5: Add lock() method to SessionState**

In `tauri/src/ipc/session.rs`, after `pub struct SessionState(pub Arc<Mutex<SessionManager>>);`:

```rust
impl SessionState {
    /// Acquire the session manager, recovering from a poisoned Mutex.
    pub fn lock(&self) -> std::sync::MutexGuard<'_, SessionManager> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}
```

Then replace every `state.0.lock().unwrap()` in `session.rs` with `state.lock()`.  
Also update `project.rs`: replace `state.0.lock().unwrap()` with `state.lock()`.

- [ ] **Step 2.6: Replace manager.lock().unwrap() in session_manager.rs**

In `do_spawn` and `reader_loop` (which take `Arc<Mutex<SessionManager>>`), replace all:
```rust
manager.lock().unwrap()
```
With:
```rust
manager.lock().unwrap_or_else(|e| e.into_inner())
```

There are approximately 4 call sites inside `do_spawn` (lines 166, 251, 288, 301) and 2 inside `reader_loop` (lines 329, 418).

- [ ] **Step 2.7: Replace conn.lock().unwrap() in database.rs**

In `database.rs`, replace all:
```rust
self.conn.lock().unwrap()
```
With:
```rust
self.conn.lock().unwrap_or_else(|e| e.into_inner())
```

There are approximately 15 occurrences.

- [ ] **Step 2.8: Run tests**

```bash
cd tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 2.9: Commit**

```bash
git add tauri/src/models.rs tauri/src/ipc/session.rs tauri/src/ipc/project.rs \
        tauri/src/services/session_manager.rs tauri/src/services/database.rs \
        tauri/src/journal_reader.rs
git commit -m "refactor: JournalEntry::default(), SessionState::lock() with Mutex poison recovery (A5+A7+T1)"
```

---

### Task 3: M5 — Session.status as SessionStatus enum

**Files:**
- Modify: `tauri/src/models.rs` (add `FromSql`/`ToSql` for `SessionStatus`, change `Session.status` field)
- Modify: `tauri/src/services/database.rs` (row mapping — `row.get()` now returns `SessionStatus` directly)
- Modify: `tauri/src/services/session_manager.rs` (remove `.as_str().to_string()` from all assignments)

**Why:** `session.status == "stopped"` — a typo (`"stoped"`) compiles and silently fails. `session.status == SessionStatus::Stopped` fails at compile time.

- [ ] **Step 3.1: Write a failing test**

In `tauri/src/services/database.rs` tests, add:

```rust
#[test]
fn should_round_trip_session_status_as_enum() {
    let mut t = TestCase::new("should_round_trip_session_status_as_enum");
    t.phase("Seed");
    let db = make_db();
    let sid = db.create_session(None, None, "/tmp", "ignore", None).expect("session");
    db.update_session_status(sid, "stopped").expect("update");
    t.phase("Act");
    let sessions = db.get_sessions().expect("get");
    t.phase("Assert");
    t.eq(
        "status is SessionStatus::Stopped",
        sessions[0].status,
        crate::models::SessionStatus::Stopped,
    );
}
```

Run: `cd tauri && cargo test should_round_trip_session_status_as_enum 2>&1 | tail -20`  
Expected: FAIL — does not compile because `Session.status` is `String` and `PartialEq<SessionStatus>` is not implemented.

- [ ] **Step 3.2: Add rusqlite FromSql and ToSql impls for SessionStatus**

At the bottom of `tauri/src/models.rs`, add:

```rust
impl rusqlite::types::FromSql for SessionStatus {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let s = String::column_result(value)?;
        Ok(match s.as_str() {
            "initializing" => SessionStatus::Initializing,
            "running" => SessionStatus::Running,
            "waiting" => SessionStatus::Waiting,
            "completed" => SessionStatus::Completed,
            "error" => SessionStatus::Error,
            _ => SessionStatus::Stopped, // covers "stopped" + unknown/legacy values
        })
    }
}

impl rusqlite::types::ToSql for SessionStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        Ok(rusqlite::types::ToSqlOutput::from(self.as_str()))
    }
}
```

Also add `use rusqlite;` at the top of `models.rs` or add it to the import block.

- [ ] **Step 3.3: Change Session.status field type**

In `tauri/src/models.rs`, the `Session` struct:

```rust
// BEFORE:
pub status: String,

// AFTER:
pub status: SessionStatus,
```

- [ ] **Step 3.4: Fix session_manager.rs — remove .as_str().to_string() from all assignments**

In `session_manager.rs`, replace every `SessionStatus::Xxx.as_str().to_string()` assignment:

```rust
// BEFORE:
status: crate::models::SessionStatus::Initializing.as_str().to_string(),
// ...
a.session.status = crate::models::SessionStatus::Running.as_str().to_string();
// ...
a.session.status = crate::models::SessionStatus::Completed.as_str().to_string();

// AFTER:
status: crate::models::SessionStatus::Initializing,
// ...
a.session.status = crate::models::SessionStatus::Running;
// ...
a.session.status = crate::models::SessionStatus::Completed;
```

Also fix `get_sessions()` — the `reader_loop` status string:
```rust
// BEFORE:
let status_str = match state.status {
    AgentStatus::Working => "working",
    // ...
}.to_string();
// event.status = status_str

// This is for the SessionStateEvent.status field (String) — leave it as String
// since it's a different type (AgentStatus → String, not SessionStatus)
```

Only `Session.status` changes to enum; `SessionStateEvent.status` remains `String`.

- [ ] **Step 3.5: Fix test assertions**

In `session_manager.rs` tests, update string comparisons to enum comparisons:

```rust
// BEFORE:
t.eq("status is initializing", s.status.as_str(), "initializing");
t.eq("status is stopped", sessions[0].status.as_str(), "stopped");

// AFTER (SessionStatus derives PartialEq + Debug):
t.eq("status is initializing", &s.status, &crate::models::SessionStatus::Initializing);
t.eq("status is stopped", &sessions[0].status, &crate::models::SessionStatus::Stopped);
```

- [ ] **Step 3.6: Run tests**

```bash
cd tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass including `should_round_trip_session_status_as_enum`.

- [ ] **Step 3.7: Commit**

```bash
git add tauri/src/models.rs tauri/src/services/database.rs tauri/src/services/session_manager.rs
git commit -m "refactor: Session.status as SessionStatus enum — compile-time typo safety (M5)"
```

---

### Task 4: P3 — Lazy restore_from_db

**Files:**
- Modify: `tauri/src/services/session_manager.rs` (add `load_session_journal`, update `get_journal`/`get_sessions` to lazy-load)
- Modify: `tauri/src/lib.rs` (remove upfront `restore_from_db()` call)

**Why:** Startup currently replays every JSONL for every session synchronously, blocking app launch. With lazy loading, sessions are loaded from DB only when the user opens them.

**Trade-off:** `get_sessions` (the session list) will show no token counts until each session's journal is first accessed. Token counts appear immediately after the user opens a session.

- [ ] **Step 4.1: Write failing tests for lazy behavior**

In `session_manager.rs` tests, add:

```rust
#[test]
fn should_not_preload_journal_state_on_creation() {
    let mut t = TestCase::new("should_not_preload_journal_state_on_creation");
    t.phase("Seed — DB has session with outputs, manager is fresh");
    let db = make_db();
    let sid = db.create_session(None, None, "/tmp", "ignore", None).expect("session");
    seed_outputs(&db, sid, &[&crate::test_utils::assistant_text("hello")]);
    t.phase("Act — create manager, no restore call");
    let sm = SessionManager::new(Arc::clone(&db));
    t.phase("Assert — journal not loaded yet");
    t.ok("journal_states empty before access", !sm.journal_states.contains_key(&sid));
}

#[test]
fn should_lazy_load_journal_on_first_get_journal() {
    let mut t = TestCase::new("should_lazy_load_journal_on_first_get_journal");
    t.phase("Seed");
    let db = make_db();
    let sid = db.create_session(None, None, "/tmp", "ignore", None).expect("session");
    seed_outputs(&db, sid, &[&crate::test_utils::assistant_text("hello")]);
    t.phase("Act");
    let mut sm = SessionManager::new(Arc::clone(&db));
    let journal = sm.get_journal(sid);
    t.phase("Assert");
    t.len("one entry loaded on demand", &journal, 1);
}
```

Run: `cd tauri && cargo test should_not_preload_journal_state_on_creation 2>&1 | tail -20`  
Expected: FAIL — currently `restore_from_db` is called in `lib.rs` at startup (but not in these tests, so the first test may pass; the second may fail because `get_journal` returns empty for unloaded states).

Note: these tests must pass WITHOUT calling `restore_from_db`.

- [ ] **Step 4.2: Add private load_session_journal helper**

In `session_manager.rs`, add a private method:

```rust
/// Load journal state for `session_id` from DB into `journal_states` if not already present.
fn load_session_journal(&mut self, session_id: SessionId) {
    if self.journal_states.contains_key(&session_id) {
        return;
    }
    let rows = match self.db.get_outputs(session_id) {
        Ok(r) => r,
        Err(_) => return,
    };
    let mut state = JournalState::default();
    for line in &rows {
        process_line(&mut state, line);
    }
    self.journal_states.insert(session_id, state);
}
```

- [ ] **Step 4.3: Change get_journal to &mut self and trigger lazy load**

```rust
// BEFORE signature:
pub fn get_journal(&self, session_id: SessionId) -> Vec<crate::models::JournalEntry> {

// AFTER:
pub fn get_journal(&mut self, session_id: SessionId) -> Vec<crate::models::JournalEntry> {
    self.load_session_journal(session_id);
    // ... rest unchanged
```

- [ ] **Step 4.4: Change get_sessions to &mut self and trigger lazy load per session**

```rust
// BEFORE:
pub fn get_sessions(&self) -> Vec<Session> {
    let mut sessions = self.db.get_sessions().unwrap_or_default();
    for s in &mut sessions {
        if let Some(state) = self.journal_states.get(&s.id) {

// AFTER:
pub fn get_sessions(&mut self) -> Vec<Session> {
    let mut sessions = self.db.get_sessions().unwrap_or_default();
    for s in &mut sessions {
        self.load_session_journal(s.id);  // ← lazy load
        if let Some(state) = self.journal_states.get(&s.id) {
```

- [ ] **Step 4.5: Rewrite restore_from_db to use load_session_journal**

Simplify `restore_from_db` to delegate to the shared helper:

```rust
pub fn restore_from_db(&mut self) {
    let session_ids: Vec<SessionId> = self
        .db
        .get_sessions()
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.id)
        .collect();
    for id in session_ids {
        self.load_session_journal(id);
    }
}
```

- [ ] **Step 4.6: Remove restore_from_db call from lib.rs**

In `tauri/src/lib.rs`:

```rust
// BEFORE:
let session_manager = {
    let mut sm = SessionManager::new(db);
    sm.restore_from_db();
    Arc::new(Mutex::new(sm))
};

// AFTER:
let session_manager = Arc::new(Mutex::new(SessionManager::new(db)));
```

- [ ] **Step 4.7: Run tests**

```bash
cd tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass including the two new lazy-load tests. The existing `should_rebuild_journal_state_from_stored_outputs` test calls `restore_from_db()` explicitly and must still pass.

- [ ] **Step 4.8: Commit**

```bash
git add tauri/src/services/session_manager.rs tauri/src/lib.rs
git commit -m "perf: lazy journal restore — load per-session state on first access (P3)"
```

---

### Task 5: M1 — parse_journal delegates to process_line

**Prerequisite:** PR #13 (`orbit/arch-refactor-red`) merged into `dev`.

**Files:**
- Modify: `tauri/src/journal/parser.rs`
- Read: `tauri/src/journal/processor.rs` (to understand what process_line already handles)

**Why:** After the arch-refactor split, `parse_journal` and `process_line` still contain ~250 lines of nearly-identical assistant/user message parsing. Every bug fix must be applied in two places. Making `parse_journal` call `process_line` line by line eliminates the duplication.

**Approach:** Replace `parser.rs`'s internal message loop with calls to `process_line`. Add a post-processing step for `thinking_duration` (a replay-only concern) and keep `derive_status_from_tail` (also replay-only).

- [ ] **Step 5.1: Verify parity between the two paths (regression baseline)**

Run the existing consistency test to confirm current parity:

```bash
cd tauri && cargo test should_return_same_entry_count_as_process_line_for_identical_input 2>&1 | tail -10
```

Expected: PASS. This test must continue to pass after the refactor.

- [ ] **Step 5.2: Refactor parse_journal to delegate to process_line**

In `tauri/src/journal/parser.rs`, replace the internal parsing loop. The new structure:

```rust
use std::io::BufRead;
use super::processor::process_line;

pub fn parse_journal(
    path: &Path,
    prev_file_size: u64,
    prev_state: Option<&JournalState>,
) -> JournalState {
    let mut state = match prev_state {
        Some(prev) => JournalState {
            status: AgentStatus::Idle,
            pending_approval: None,
            ..prev.clone()
        },
        None => JournalState::default(),
    };

    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return state,
    };

    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
    state.file_size = prev_file_size + file_size;

    if prev_file_size > 0 {
        use std::io::Seek;
        let _ = file.seek(std::io::SeekFrom::Start(prev_file_size));
    }

    let prev_entry_count = state.entries.len();
    let reader = std::io::BufReader::new(file);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('{') {
            continue;
        }
        process_line(&mut state, trimmed);
    }

    // Post-process: fill thinking_duration for newly-added entries
    // (process_line doesn't track this; parse_journal can compute it from timestamps)
    patch_thinking_duration(&mut state.entries[prev_entry_count..]);

    // Override final status with tail-derived value (more accurate for completed replays)
    state.status = derive_status_from_tail(path).unwrap_or(state.status);

    state
}

/// Compute thinking_duration for thinking entries by measuring the gap to the next entry's timestamp.
fn patch_thinking_duration(entries: &mut [crate::models::JournalEntry]) {
    for i in 0..entries.len().saturating_sub(1) {
        if entries[i].thinking.is_some() && entries[i].thinking_duration.is_none() {
            let t0 = chrono::DateTime::parse_from_rfc3339(&entries[i].timestamp).ok();
            let t1 = chrono::DateTime::parse_from_rfc3339(&entries[i + 1].timestamp).ok();
            if let (Some(t0), Some(t1)) = (t0, t1) {
                let ms = (t1 - t0).num_milliseconds();
                entries[i].thinking_duration = Some((ms.max(0) as f64) / 1000.0);
            }
        }
    }
}
```

Note: `derive_status_from_tail` should return `Option<AgentStatus>` — update its signature if it currently returns `AgentStatus` directly.

- [ ] **Step 5.3: Remove the now-dead private parsing code from parser.rs**

After the refactor, the large assistant/user message parsing blocks inside `parser.rs` are dead code. Remove:
- The manual `assistant`, `user`, `progress`, `result`, `system` match arms from the old loop
- Any imports only needed by those blocks (`extract_tool_target`, `truncate_output` if no longer called directly)

Keep: `derive_status_from_tail`, `json_contains`, `patch_thinking_duration`.

- [ ] **Step 5.4: Run tests**

```bash
cd tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass. The parity test from Step 5.1 must still pass.

- [ ] **Step 5.5: Commit**

```bash
git add tauri/src/journal/parser.rs tauri/src/journal/processor.rs
git commit -m "refactor: parse_journal delegates to process_line, removes ~250 lines of duplication (M1)"
```

---

### Task 6: P4 — Batched INSERT for session_outputs

**Files:**
- Modify: `tauri/src/services/database.rs` (add channel, background flush worker)
- No other files need changes — `insert_output`'s signature stays the same

**Why:** Claude at full speed emits 50–100 JSON lines/second. Each line is a separate `INSERT INTO session_outputs` + `Mutex` acquisition. A background worker accumulates lines and flushes in a `BEGIN/COMMIT` batch every 100ms, reducing DB writes by ~10×.

- [ ] **Step 6.1: Change conn field to Arc for sharing with worker**

Currently `conn: Mutex<Connection>`. Change to `Arc<Mutex<Connection>>` so the background worker can hold a clone:

```rust
pub struct DatabaseService {
    conn: std::sync::Arc<Mutex<Connection>>,
    output_tx: std::sync::mpsc::SyncSender<WorkerMsg>,
}

enum WorkerMsg {
    Row(SessionId, String),
    Flush(std::sync::mpsc::SyncSender<()>),
}
```

Update all `self.conn.lock()` call sites — no change needed since `Arc<Mutex<T>>` derefs the same way.

Update `open` and `open_in_memory` constructors to wrap with `Arc`:
```rust
let conn = Arc::new(Mutex::new(Connection::open(path)?));
```

- [ ] **Step 6.2: Add the background flush worker**

Add a private constructor helper:

```rust
fn start_output_worker(
    conn: std::sync::Arc<Mutex<Connection>>,
) -> std::sync::mpsc::SyncSender<WorkerMsg> {
    let (tx, rx) = std::sync::mpsc::sync_channel::<WorkerMsg>(1024);
    std::thread::spawn(move || {
        let mut buf: Vec<(SessionId, String)> = Vec::with_capacity(64);
        loop {
            let deadline = std::time::Instant::now()
                + std::time::Duration::from_millis(100);
            loop {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                if remaining.is_zero() {
                    break;
                }
                match rx.recv_timeout(remaining) {
                    Ok(WorkerMsg::Row(sid, data)) => buf.push((sid, data)),
                    Ok(WorkerMsg::Flush(reply)) => {
                        flush_batch(&conn, &mut buf);
                        let _ = reply.send(());
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        flush_batch(&conn, &mut buf);
                        return;
                    }
                }
            }
            if !buf.is_empty() {
                flush_batch(&conn, &mut buf);
            }
        }
    });
    tx
}

fn flush_batch(
    conn: &Mutex<Connection>,
    buf: &mut Vec<(SessionId, String)>,
) {
    if buf.is_empty() {
        return;
    }
    let conn = conn.lock().unwrap_or_else(|e| e.into_inner());
    let _ = conn.execute_batch("BEGIN");
    for (session_id, data) in buf.drain(..) {
        let _ = conn.execute(
            "INSERT INTO session_outputs (session_id, data) VALUES (?1, ?2)",
            params![session_id, data],
        );
    }
    let _ = conn.execute_batch("COMMIT");
}
```

- [ ] **Step 6.3: Update open() and open_in_memory() to start the worker**

```rust
pub fn open(path: &Path) -> SqlResult<Self> {
    let conn = Arc::new(Mutex::new(Connection::open(path)?));
    let output_tx = Self::start_output_worker(Arc::clone(&conn));
    let db = DatabaseService { conn, output_tx };
    db.migrate()?;
    Ok(db)
}

pub fn open_in_memory() -> SqlResult<Self> {
    let conn = Arc::new(Mutex::new(Connection::open_in_memory()?));
    let output_tx = Self::start_output_worker(Arc::clone(&conn));
    let db = DatabaseService { conn, output_tx };
    db.migrate()?;
    Ok(db)
}
```

- [ ] **Step 6.4: Change insert_output to send to channel**

```rust
pub fn insert_output(&self, session_id: SessionId, data: &str) -> SqlResult<()> {
    let _ = self.output_tx.send(WorkerMsg::Row(session_id, data.to_string()));
    Ok(())
}
```

- [ ] **Step 6.5: Add flush_outputs for tests**

```rust
/// Block until all pending output rows are written. Required before calling get_outputs in tests.
pub fn flush_outputs(&self) {
    let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(0);
    let _ = self.output_tx.send(WorkerMsg::Flush(reply_tx));
    let _ = reply_rx.recv();
}
```

- [ ] **Step 6.6: Update database tests to flush before reading**

In `database.rs` tests, any test that calls `insert_output` then `get_outputs` must call `db.flush_outputs()` between them. Example:

```rust
// Anywhere you have:
db.insert_output(sid, "...").expect("insert");
let rows = db.get_outputs(sid).expect("get");

// Change to:
db.insert_output(sid, "...").expect("insert");
db.flush_outputs();
let rows = db.get_outputs(sid).expect("get");
```

Also update any test that uses `seed_outputs` from `test_utils.rs` and then reads back — add `flush_outputs` after the seed call.

- [ ] **Step 6.7: Run tests**

```bash
cd tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 6.8: Commit**

```bash
git add tauri/src/services/database.rs tauri/src/test_utils.rs
git commit -m "perf: batch session_outputs INSERTs — background worker, 100ms flush interval (P4)"
```

---

### Task 7: P8 — RwLock for SessionManager

**Files:**
- Modify: `tauri/src/ipc/session.rs` (change `SessionState` to use `RwLock`, add `write()`/`read()` methods)
- Modify: `tauri/src/ipc/project.rs` (use `state.write()`)
- Modify: `tauri/src/services/session_manager.rs` (change `Arc<Mutex<…>>` parameter to `Arc<RwLock<…>>`)
- Modify: `tauri/src/lib.rs` (wrap in `RwLock`)

**Why:** `journal_states` and `active` are read far more often than written. A `Mutex` blocks all readers when a writer holds it — two concurrent reads block each other. `RwLock` allows unlimited concurrent reads; only writes serialize.

**Approach:** Change `Arc<Mutex<SessionManager>>` to `Arc<RwLock<SessionManager>>` throughout. Replace `state.lock()` (from Task 2) with `state.write()` for mutations and `state.read()` for queries.

- [ ] **Step 7.1: Update SessionState in ipc/session.rs**

```rust
use std::sync::{Arc, RwLock};
// ...

pub struct SessionState(pub Arc<RwLock<SessionManager>>);

impl SessionState {
    pub fn write(&self) -> std::sync::RwLockWriteGuard<'_, SessionManager> {
        self.0.write().unwrap_or_else(|e| e.into_inner())
    }

    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, SessionManager> {
        self.0.read().unwrap_or_else(|e| e.into_inner())
    }
}
```

Remove the `lock()` method added in Task 2 — it's replaced by `write()`/`read()`.

In each command function in `session.rs`, replace `state.lock()` with:
- `state.write()` for functions that call mutating methods (`init_session`, `stop_session`, `delete_session`, `rename_session`, `send_message`)
- `state.read()` for read-only calls (`get_sessions`, `get_session_journal`, `is_session_active`)

Note: `get_sessions` and `get_journal` were changed to `&mut self` in Task 4. `RwLockReadGuard` only gives `&T`. If those methods need `&mut self`, they must use `state.write()` instead. Read the signatures after Task 4 to decide.

- [ ] **Step 7.2: Update project.rs**

In `tauri/src/ipc/project.rs`, replace `state.lock()` with `state.write()` (both commands mutate or query DB through `SessionManager`).

- [ ] **Step 7.3: Update do_spawn and reader_loop parameter types**

In `session_manager.rs`, both `do_spawn` and `reader_loop` take `Arc<Mutex<SessionManager>>`. Change to `Arc<RwLock<SessionManager>>`:

```rust
// BEFORE:
pub fn do_spawn(manager: Arc<Mutex<SessionManager>>, ...) {
    let m = manager.lock().unwrap_or_else(|e| e.into_inner());

// AFTER:
pub fn do_spawn(manager: Arc<RwLock<SessionManager>>, ...) {
    let m = manager.write().unwrap_or_else(|e| e.into_inner());
```

Apply the same change to `reader_loop`. Replace all `manager.lock().unwrap_or_else(...)` with `manager.write().unwrap_or_else(...)`.

- [ ] **Step 7.4: Update lib.rs**

```rust
use std::sync::{Arc, RwLock};
// ...
let session_manager = Arc::new(RwLock::new(SessionManager::new(db)));
app.manage(SessionState(session_manager));
```

- [ ] **Step 7.5: Update ipc/session.rs Arc::clone call**

In `create_session`, the `Arc::clone` for the background thread still works — `Arc<RwLock<…>>` is clonable.

- [ ] **Step 7.6: Run tests**

```bash
cd tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 7.7: Commit**

```bash
git add tauri/src/ipc/session.rs tauri/src/ipc/project.rs \
        tauri/src/services/session_manager.rs tauri/src/lib.rs
git commit -m "perf: RwLock<SessionManager> — concurrent reads for journal list and session list (P8)"
```

---

### Task 8: A6 — Typed IPC errors via thiserror

**Prerequisite:** PR #13 (`orbit/arch-refactor-red`) merged into `dev` (commands live in `commands/` directory).

**Files:**
- Modify: `tauri/Cargo.toml` (add `thiserror = "1"`)
- Create: `tauri/src/ipc/error.rs`
- Modify: `tauri/src/ipc/mod.rs` (add `pub mod error; pub use error::IpcError;`)
- Modify: `tauri/src/ipc/session.rs` (use `IpcError`)
- Modify: `tauri/src/ipc/project.rs` (use `IpcError`)
- Modify: `tauri/src/commands/diff.rs` (use `IpcError`)
- Modify: `tauri/src/commands/files.rs` (use `IpcError`)
- Modify: `tauri/src/commands/tasks.rs` (use `IpcError`)
- Modify: `tauri/src/commands/plugins.rs` (use `IpcError`)
- Modify: `tauri/src/commands/stats.rs` (use `IpcError`)

**Why:** All commands return `Result<T, String>`. The frontend cannot distinguish "session not found" from "database error" without parsing the string. `IpcError` gives structured error codes; `thiserror` generates the boilerplate.

- [ ] **Step 8.1: Add thiserror to Cargo.toml**

```toml
[dependencies]
# ... existing deps ...
thiserror = "1"
```

- [ ] **Step 8.2: Create tauri/src/ipc/error.rs**

```rust
/// Typed error returned by all Tauri IPC commands.
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("Session {0} not found")]
    SessionNotFound(crate::models::SessionId),

    #[error("Project not found")]
    ProjectNotFound,

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl From<String> for IpcError {
    fn from(s: String) -> Self {
        IpcError::Other(s)
    }
}

// Tauri requires command errors to implement Serialize
impl serde::Serialize for IpcError {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.to_string())
    }
}
```

- [ ] **Step 8.3: Export from ipc/mod.rs**

```rust
pub mod error;
pub mod project;
pub mod session;
pub mod updater;
pub use error::IpcError;
```

- [ ] **Step 8.4: Change all commands from Result<T, String> to Result<T, IpcError>**

For each command file, replace `Result<T, String>` with `Result<T, crate::ipc::IpcError>`.

Replace `.map_err(|e| e.to_string())` with `?` where the source error type implements `From<…> for IpcError` (e.g., `rusqlite::Error` → `IpcError::Database`).

Replace `.map_err(|e| e.to_string())` with `.map_err(IpcError::from)` where the source is `String`.

Example — `ipc/session.rs`:
```rust
use crate::ipc::IpcError;

#[tauri::command]
pub fn create_session(/* ... */) -> Result<Session, IpcError> {
    let session = {
        let mut m = state.write();
        m.init_session(/* ... */).map_err(IpcError::from)?
    };
    // ...
    Ok(session)
}
```

Example — `ipc/project.rs`:
```rust
use crate::ipc::IpcError;

#[tauri::command]
pub fn create_project(/* ... */) -> Result<Project, IpcError> {
    state
        .write()
        .db
        .create_project(&name, &path)
        .map_err(IpcError::Database)
}
```

Example — `commands/diff.rs`:
```rust
use crate::ipc::IpcError;

#[tauri::command]
pub fn get_diff(/* ... */) -> Result<DiffResult, IpcError> {
    diff_builder::build_diff(/* ... */)
        .ok_or_else(|| IpcError::Other("Could not build diff".to_string()))
}
```

- [ ] **Step 8.5: Run tests**

```bash
cd tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 8.6: Commit**

```bash
git add tauri/Cargo.toml tauri/src/ipc/ tauri/src/commands/
git commit -m "refactor: typed IpcError via thiserror — structured errors for all Tauri commands (A6)"
```

---

## Self-Review

### Spec coverage

| Item | Task |
|------|------|
| P5 — WAL mode | Task 1 ✅ |
| C3 — UTF-8 truncation | Task 1 ✅ |
| P2 — rate limit alloc | Task 1 ✅ |
| A5 — JournalEntry::default() | Task 2 ✅ |
| A7 — SessionState::lock() | Task 2 ✅ |
| T1 — Mutex poison recovery | Task 2 ✅ |
| M5 — Session.status enum | Task 3 ✅ |
| P3 — lazy restore | Task 4 ✅ |
| M1 — parse_journal → process_line | Task 5 ✅ |
| P4 — INSERT batching | Task 6 ✅ |
| P8 — RwLock | Task 7 ✅ |
| A6 — thiserror IpcError | Task 8 ✅ |

All 12 items covered.

### Key cross-task dependencies to watch

- **Task 7 overrides Task 2**: Task 2 adds `SessionState::lock()`, Task 7 replaces it with `read()`/`write()`. The implementer of Task 7 must remove `lock()` and update all call sites added in Task 2.
- **Task 4 changes method signatures**: `get_journal` and `get_sessions` become `&mut self`. Task 7's `read()` guard (immutable) cannot call `&mut self` methods — those callers must use `write()` instead.
- **Task 5 requires arch-refactor-red merged**: References `journal/processor.rs` and `journal/parser.rs` which don't exist on `dev` until PR #13 is merged.
- **Task 8 requires arch-refactor-red merged**: References `commands/diff.rs`, `commands/files.rs`, etc.
