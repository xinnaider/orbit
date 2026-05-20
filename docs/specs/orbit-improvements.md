# Orbit Architecture Improvements

Inspired by Paseo (https://github.com/getpaseo/paseo), prioritizing real-time feedback and error handling while maintaining Tauri backend.

---

## Phase 1: Critical Fixes

### 1.1. Provider Registry Trait

**File:** `tauri/src/lib.rs`

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;

    async fn spawn(&self, config: ProviderSpawnConfig) -> Result<SpawnHandle, OrbitError>;
    fn process_line(&self, state: &mut JournalState, line: &str);

    fn format_model(&self, raw_model: &str) -> String;
    fn context_window(&self, model: &str) -> Option<u64>;

    fn slash_commands(&self) -> Vec<SlashCommand>;
    fn supports_effort(&self) -> bool;
    fn supports_ssh(&self) -> bool;

    fn cli_name(&self) -> &str;
    fn find_cli(&self) -> Option<String>;
    fn install_hint(&self) -> &str;
}
```

**Benefit:** Remove hardcoded provider strings, enable new providers without touching session_manager.rs

---

### 1.2. JSON Parsing with Logging

**File:** `tauri/src/journal/processor.rs:127-130`

```rust
let raw: RawEntry = match serde_json::from_str(trimmed) {
    Ok(r) => r,
    Err(e) => {
        tracing::warn!(
            session_id = %session_id,
            line_preview = %trimmed.chars().take(100).collect::<String>(),
            error = %e,
            "Failed to parse JSONL entry, silently dropping"
        );
        return;
    }
};
```

**Benefit:** Error visibility in debug logs, easier debugging, prevents silent data loss

---

### 1.3. Remove RwLock Write Blocking

**File:** `tauri/src/services/session_manager.rs:680`

**Problem:** `let mut m = manager.write()` blocks UI updates

**Solution:** Separate read and write operations:

```rust
let (new_entries, state_event, is_rate_limit) = {
    let m = manager.read().unwrap_or_else(|e| e.into_inner());
    // ... only read operations
    let state = m.journal_states.get(&session_id);
    // ... build event without write lock

    (new_entries, state_event, is_rate_limit)
};

// Write lock only for updates
let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
// ... update state
```

**Benefit:** UI updates not blocked by JSONL processing

---

### 1.4. Add Timeout to Read Lock

**File:** `tauri/src/services/session_manager.rs:297`

**Problem:** `let m = manager.read()` can block indefinitely

**Solution:** Use `try_read()` with timeout:

```rust
let m = manager.try_read().ok_or_else(|| {
    tracing::warn!(session_id = %session_id, "Failed to acquire read lock, skipping line");
    OrbitError::LockAcquisitionTimeout
})?;
```

**Benefit:** Prevents deadlocks, more resilient system

---

### 1.5. Stderr Event Emission

**File:** `tauri/src/services/session_manager.rs:465-484`

**Problem:** Errors only visible in debug mode

**Solution:** Emit events to frontend:

```rust
std::thread::spawn(move || {
    let mut reader = std::io::BufReader::new(stderr_reader);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    tracing::warn!(session_id = %session_id, stderr = %trimmed);
                    let _ = app.emit("session:stderr", serde_json::json!({
                        "sessionId": session_id,
                        "line": trimmed
                    }));
                }
            }
        }
    }
});
```

**Benefit:** Real-time error visibility, permission fallbacks shown to user

---

## Phase 2: Real-Time Git

### 2.1. Git Service with Watching

**New file:** `tauri/src/services/git_service.rs`

```rust
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct GitService {
    watchers: Arc<Mutex<Map<PathBuf, Box<dyn Fn()>>>>,
}

impl GitService {
    pub async def watch_working_tree(
        &self,
        cwd: PathBuf,
        callback: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<(), GitError> {
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                if let Ok(events) = res {
                    for event in events {
                        if event.kind.contains(notify::EventKind::Modify(_)) {
                            callback();
                        }
                    }
                }
            },
            notify::Config::default(),
        )?;

        watcher.watch(&cwd, RecursiveMode::Recursive)?;

        // Store watcher
        let mut watchers = self.watchers.lock().await;
        watchers.insert(cwd, Box::new(move || {
            // Unwatch logic
        }));

        Ok(())
    }
}
```

**Benefit:** Automatic file change detection in real-time

---

### 2.2. CheckoutDiffManager

**New file:** `tauri/src/services/diff_manager.rs`

```rust
pub struct CheckoutDiffManager {
    git_service: Arc<GitService>,
    debouncer: Arc<Mutex<Debouncer>>,
}

const DEBOUNCE_MS: u64 = 150;

impl CheckoutDiffManager {
    pub async def request_snapshot(&self, cwd: PathBuf) -> Result<GitOverview, GitError> {
        // Debounce requests
        let mut debouncer = self.debouncer.lock().await;
        debouncer.schedule(DEBOUNCE_MS, move || {
            // Run git commands
        });

        // Get snapshot
        let snapshot = self.git_service.get_snapshot(&cwd).await?;
        Ok(snapshot)
    }
}
```

**Benefit:** Auto-updated diffs when editing files, no manual refresh

---

## Phase 3: Feedback Mechanisms

### 3.1. OrbitError Enum

**New file:** `tauri/src/errors.rs`

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrbitError {
    #[error("Lock acquisition timeout")]
    LockAcquisitionTimeout,

    #[error("Session timeout after {0:?}")]
    SessionTimeout(std::time::Duration),

    #[error("Provider unavailable: {0}")]
    ProviderUnavailable(String),

    #[error("Git error: {0}")]
    GitError(#[from] GitError),

    #[error("JSON parsing failed: {0}")]
    JsonParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, OrbitError>;
```

**Benefit:** Typed errors, clear hierarchy, easy to add new variants

---

### 3.2. Attention System

**File:** `tauri/src/models.rs` (add to Session struct)

```rust
pub enum AttentionReason {
    Permission,
    RateLimit,
    Error,
}

pub struct AttentionState {
    pub requires_attention: bool,
    pub reason: Option<AttentionReason>,
    pub timestamp: Option<DateTime<Utc>>,
}

impl AttentionState {
    pub fn requires_permission() -> Self {
        Self {
            requires_attention: true,
            reason: Some(AttentionReason::Permission),
            timestamp: Some(Utc::now()),
        }
    }

    pub fn clear(self) -> Self {
        Self {
            requires_attention: false,
            reason: None,
            timestamp: None,
        }
    }
}
```

**Benefit:** Explicit attention states, frontend can show toasts/debounces

---

### 3.3. Toast Notifications (Frontend)

**File:** `ui/components/NotificationToast.svelte`

```svelte
<script lang="ts">
  export let message: string;
  export let type: 'error' | 'warning' | 'info';

  function dismiss() {
    // emit event to remove from state
  }
</script>

<div class="toast {type}">
  {message}
  <button on:click={dismiss}>✕</button>
</div>
```

**Benefit:** Immediate visual feedback for errors, permissions, rate limits

---

### 3.4. Graceful Session Reload

**File:** `tauri/src/services/session_manager.rs`

```rust
const RELOAD_SESSION_CLOSE_TIMEOUT_MS: u64 = 3_000;

async fn close_session_with_timeout(session: &mut Session, timeout: Duration) -> Result<()> {
    tokio::time::timeout(
        timeout,
        session.close()
    ).await
        .map_err(|_| OrbitError::SessionTimeout(timeout))??;
    Ok(())
}
```

**Benefit:** Session closes don't freeze the app, 3s timeout for graceful shutdown

---

## Phase 4: Diff Visibility

### 4.1. Fix ToolCallEntry.svelte

**File:** `ui/components/ToolCallEntry.svelte`

**Problem:** Diffs only shown when `resultEntry` exists (delayed)

**Solution:** Emit diffs in real-time during streaming:

```svelte
<script>
  export let streamingEntries: JournalEntry[];
  export let resultEntry: JournalEntry | null;

  // New: extract diffs from streaming entries
  let diffs = [];

  if (streamingEntries.length > 0 && !resultEntry) {
    diffs = streamingEntries
      .filter(e => e.tool === 'edit' || e.tool === 'write')
      .map(e => ({
        old_string: e.toolInput?.old_string,
        new_string: e.toolInput?.new_string,
        content: e.toolInput?.content,
      }));
  }
</script>

{#if diffs.length > 0}
  <div class="realtime-diffs">
    {#each diffs as diff}
      <DiffBlock old={diff.old_string} new={diff.new_string} />
    {/each}
  </div>
{/if}
```

**Benefit:** Diffs appear instantly while agent edits files

---

## Implementation Order

### Week 1: Critical Fixes
1. Provider Registry trait
2. JSON parsing logging
3. Remove RwLock write blocking
4. Add timeout to read lock
5. Stderr event emission

### Week 2: Real-Time Git
6. Git Service with watching
7. CheckoutDiffManager
8. Integrate git watching in session_manager

### Week 3: Feedback & Error Handling
9. OrbitError enum
10. Attention system
11. Toast notifications frontend
12. Graceful session reload

### Week 4: Testing & Polish
13. Fix ToolCallEntry.svelte diffs
14. Run cargo clippy fix
15. Update CHANGELOG.md
16. Test end-to-end

---

## Expected Benefits

| Problem | Solution | Impact |
|---------|----------|--------|
| Silent parse failures | JSON error logging | Debugging + rate limit detection |
| UI blocking on JSONL processing | Remove write lock | Maintained responsiveness |
| Errors only in debug logs | Stderr events | Real-time visual feedback |
| Git not updating | Working-tree watching | Auto-updated diffs |
| Diffs visible with delay | Real-time diff emission | Immediate visualization |
| Provider techniques hardcoded | Provider Registry | Extensibility |

---

## References

- Paseo Architecture: https://github.com/getpaseo/paseo
- Orbit Codebase: `C:\Users\fernandonepen\Documents\orbit`
- Rust Async Patterns: Apollo GraphQL handbook
- Rust Best Practices: Apollo GraphQL handbook