# Provider Trait Refactor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace 6 scattered match-dispatch points with a single `Provider` trait, enabling dependency inversion — services work with the trait, not concrete implementations.

**Architecture:** Define a `Provider` trait with 7 methods covering spawn, JSONL parsing, context window, slash commands, effort support, and resume args. Implement for `ClaudeProvider`, `CodexProvider`, `OpenCodeProvider`. A `ProviderRegistry` maps provider IDs to `&dyn Provider`. `SessionManager` uses the registry instead of match arms. TDD: write trait contract tests first with a `MockProvider`, then migrate each concrete provider.

**Tech Stack:** Rust traits, `dyn Provider + Send + Sync`, existing rusqlite/tauri stack, cargo test.

---

### Current dispatch points (to eliminate)

| # | File | Line | Dispatches on |
|---|------|------|--------------|
| 1 | `session_manager.rs:184` | `do_spawn` | `match provider.as_str()` → 3 `do_spawn_*` methods |
| 2 | `session_manager.rs:900` | `load_session_journal` | `match provider_owned` → 3 `process_line_*` fns |
| 3 | `plugins.rs:115` | `get_slash_commands` | `match backend` → 3 `get_*_commands()` fns |
| 4 | `session_manager.rs:395` | `do_spawn_opencode` | `lookup_context_window()` |
| 5 | `session_manager.rs:483` | `do_spawn_codex` | `codex_context_window()` |
| 6 | `ipc/session.rs:107` | `send_session_message` | `provider != "claude-code"` for /effort |

### File structure after refactor

```
tauri/src/
├── providers/
│   ├── mod.rs           # Provider trait + ProviderRegistry + re-exports
│   ├── claude.rs        # ClaudeProvider impl
│   ├── codex.rs         # CodexProvider impl
│   └── opencode.rs      # OpenCodeProvider impl
├── services/
│   ├── session_manager.rs  # Uses &dyn Provider, no match arms
│   ├── spawn_manager.rs    # Keeps find_claude/find_cli_in_path + SpawnHandle (shared infra)
│   └── database.rs         # Unchanged
├── ipc/
│   └── session.rs          # Uses provider.supports_effort()
├── commands/
│   ├── plugins.rs          # get_slash_commands delegates to provider.slash_commands()
│   └── providers.rs        # get_providers IPC (unchanged, returns UI data)
├── journal/
│   ├── processor.rs        # process_line stays here (used by ClaudeProvider)
│   ├── processor_opencode.rs  # process_line_opencode (used by OpenCodeProvider)
│   ├── processor_codex.rs     # process_line_codex (used by CodexProvider)
│   ├── state.rs            # Unchanged
│   └── mod.rs              # Re-exports
└── lib.rs                  # Registers ProviderRegistry in Tauri state
```

---

### Task 1: Define Provider trait + ProviderRegistry + tests

**Files:**
- Create: `tauri/src/providers/mod.rs`
- Modify: `tauri/src/lib.rs:1` (add `pub mod providers;`)

- [ ] **Step 1: Write the failing test — trait contract**

```rust
// tauri/src/providers/mod.rs

use crate::journal::JournalState;
use crate::models::SlashCommand;
use crate::services::spawn_manager::SpawnHandle;
use std::path::PathBuf;

/// Unified configuration for spawning any provider CLI.
pub struct ProviderSpawnConfig {
    pub session_id: crate::models::SessionId,
    pub cwd: PathBuf,
    pub model: String,
    pub prompt: String,
    /// CLI session ID for follow-ups (claude_session_id, thread_id, sessionID)
    pub resume_id: Option<String>,
    /// Extra env vars (e.g. API key override)
    pub extra_env: Vec<(String, String)>,
}

/// Trait that every CLI backend implements.
pub trait Provider: Send + Sync {
    /// Unique identifier: "claude-code", "codex", "openrouter", etc.
    fn id(&self) -> &str;

    /// Human-readable name for the UI feed label.
    fn display_name(&self) -> &str;

    /// Spawn the CLI process. Returns a handle with stdout/stderr pipes.
    fn spawn(&self, config: ProviderSpawnConfig) -> Result<SpawnHandle, String>;

    /// Parse one JSONL line from the CLI's stdout into journal state.
    fn process_line(&self, state: &mut JournalState, line: &str);

    /// Context window size for the given model, if known.
    fn context_window(&self, model: &str) -> Option<u64>;

    /// Slash commands available for this provider.
    fn slash_commands(&self) -> Vec<SlashCommand>;

    /// Whether this provider supports the /effort command.
    fn supports_effort(&self) -> bool;
}

/// Registry that maps provider IDs to trait objects.
pub struct ProviderRegistry {
    providers: Vec<Box<dyn Provider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        ProviderRegistry {
            providers: Vec::new(),
        }
    }

    pub fn register(&mut self, provider: Box<dyn Provider>) {
        self.providers.push(provider);
    }

    pub fn get(&self, id: &str) -> Option<&dyn Provider> {
        self.providers.iter().find(|p| p.id() == id).map(|p| p.as_ref())
    }

    /// For IDs not directly registered (opencode sub-providers like
    /// "openrouter"), fall back to the "opencode" provider.
    pub fn resolve(&self, id: &str) -> Option<&dyn Provider> {
        self.get(id).or_else(|| self.get("opencode"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestCase;

    struct MockProvider {
        id: String,
        effort: bool,
    }

    impl Provider for MockProvider {
        fn id(&self) -> &str { &self.id }
        fn display_name(&self) -> &str { "Mock" }
        fn spawn(&self, _config: ProviderSpawnConfig) -> Result<SpawnHandle, String> {
            Err("mock provider cannot spawn".to_string())
        }
        fn process_line(&self, _state: &mut JournalState, _line: &str) {}
        fn context_window(&self, _model: &str) -> Option<u64> { Some(100_000) }
        fn slash_commands(&self) -> Vec<SlashCommand> { vec![] }
        fn supports_effort(&self) -> bool { self.effort }
    }

    #[test]
    fn should_register_and_retrieve_provider() {
        let mut t = TestCase::new("should_register_and_retrieve_provider");
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(MockProvider {
            id: "test".to_string(), effort: false,
        }));
        t.phase("Assert");
        t.some("provider found", &registry.get("test"));
        t.none("unknown not found", &registry.get("unknown"));
    }

    #[test]
    fn should_resolve_unknown_id_to_opencode_fallback() {
        let mut t = TestCase::new("should_resolve_unknown_id_to_opencode_fallback");
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(MockProvider {
            id: "opencode".to_string(), effort: false,
        }));
        t.phase("Assert");
        let resolved = registry.resolve("openrouter");
        t.some("resolved to opencode", &resolved);
        t.eq("id is opencode", resolved.unwrap().id(), "opencode");
    }

    #[test]
    fn should_report_effort_support_correctly() {
        let mut t = TestCase::new("should_report_effort_support_correctly");
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(MockProvider {
            id: "claude-code".to_string(), effort: true,
        }));
        registry.register(Box::new(MockProvider {
            id: "codex".to_string(), effort: false,
        }));
        t.phase("Assert");
        t.ok("claude supports effort", registry.get("claude-code").unwrap().supports_effort());
        t.ok("codex does not", !registry.get("codex").unwrap().supports_effort());
    }
}
```

- [ ] **Step 2: Add module to lib.rs**

Add `pub mod providers;` to `tauri/src/lib.rs` after the existing module declarations.

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p orbit -- providers`
Expected: 3 tests PASS

- [ ] **Step 4: Commit**

```bash
git add tauri/src/providers/mod.rs tauri/src/lib.rs
git commit -m "feat: define Provider trait, ProviderRegistry, and contract tests"
```

---

### Task 2: Implement ClaudeProvider

**Files:**
- Create: `tauri/src/providers/claude.rs`
- Modify: `tauri/src/providers/mod.rs` (add `pub mod claude;`)

- [ ] **Step 1: Write failing test**

```rust
// in tauri/src/providers/claude.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestCase;

    #[test]
    fn should_return_claude_code_as_id() {
        let mut t = TestCase::new("should_return_claude_code_as_id");
        let p = ClaudeProvider;
        t.eq("id", p.id(), "claude-code");
    }

    #[test]
    fn should_support_effort() {
        let mut t = TestCase::new("should_support_effort");
        let p = ClaudeProvider;
        t.ok("supports effort", p.supports_effort());
    }

    #[test]
    fn should_return_known_context_window() {
        let mut t = TestCase::new("should_return_known_context_window");
        let p = ClaudeProvider;
        t.some("sonnet has window", &p.context_window("claude-sonnet-4-6"));
        t.eq("sonnet is 1M", p.context_window("claude-sonnet-4-6").unwrap(), 1_000_000);
    }

    #[test]
    fn should_parse_claude_assistant_line() {
        let mut t = TestCase::new("should_parse_claude_assistant_line");
        let p = ClaudeProvider;
        let mut state = crate::journal::JournalState::default();
        let line = crate::test_utils::assistant_text("hello");
        p.process_line(&mut state, &line);
        t.eq("one entry", state.entries.len(), 1);
        t.eq("entry is assistant", &state.entries[0].entry_type, &crate::models::JournalEntryType::Assistant);
    }
}
```

- [ ] **Step 2: Implement ClaudeProvider**

```rust
// tauri/src/providers/claude.rs
use super::{Provider, ProviderSpawnConfig};
use crate::journal::JournalState;
use crate::models::SlashCommand;
use crate::services::spawn_manager::{spawn_claude, SpawnConfig, SpawnHandle};

pub struct ClaudeProvider;

impl Provider for ClaudeProvider {
    fn id(&self) -> &str { "claude-code" }
    fn display_name(&self) -> &str { "claude" }

    fn spawn(&self, config: ProviderSpawnConfig) -> Result<SpawnHandle, String> {
        spawn_claude(SpawnConfig {
            session_id: config.session_id,
            cwd: config.cwd,
            permission_mode: "ignore".to_string(),
            model: if config.model == "auto" { None } else { Some(config.model) },
            effort: None, // set separately via ActiveSession.effort
            prompt: config.prompt,
            claude_session_id: config.resume_id,
        })
    }

    fn process_line(&self, state: &mut JournalState, line: &str) {
        crate::journal::process_line(state, line);
    }

    fn context_window(&self, model: &str) -> Option<u64> {
        let w = crate::models::context_window(model);
        if w > 0 { Some(w) } else { None }
    }

    fn slash_commands(&self) -> Vec<SlashCommand> {
        // Delegate to existing get_claude_commands()
        crate::commands::plugins::get_claude_commands()
    }

    fn supports_effort(&self) -> bool { true }
}
```

- [ ] **Step 3: Export from mod.rs**

Add `pub mod claude;` to `tauri/src/providers/mod.rs`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p orbit -- providers`
Expected: 7 tests PASS (3 registry + 4 claude)

- [ ] **Step 5: Commit**

```bash
git add tauri/src/providers/claude.rs tauri/src/providers/mod.rs
git commit -m "feat: implement ClaudeProvider with tests"
```

---

### Task 3: Implement CodexProvider

**Files:**
- Create: `tauri/src/providers/codex.rs`
- Modify: `tauri/src/providers/mod.rs`

- [ ] **Step 1: Write failing tests**

Test `id()`, `display_name()`, `supports_effort()`, `context_window()`, and `process_line()` for Codex JSONL format (agent_message, command_execution, turn.completed).

- [ ] **Step 2: Implement CodexProvider**

`spawn()` delegates to `spawn_codex()`. `process_line()` delegates to `process_line_codex()`. `context_window()` uses `codex_context_window()`. `slash_commands()` delegates to `get_codex_commands()`. `supports_effort()` returns `false`.

- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat: implement CodexProvider with tests"
```

---

### Task 4: Implement OpenCodeProvider

**Files:**
- Create: `tauri/src/providers/opencode.rs`
- Modify: `tauri/src/providers/mod.rs`

- [ ] **Step 1: Write failing tests**

Test `id()`, `display_name()`, `supports_effort()`, `context_window()` (delegates to `lookup_context_window`), and `process_line()` for OpenCode JSONL format (step_start, text, tool_use, step_finish).

- [ ] **Step 2: Implement OpenCodeProvider**

`spawn()` delegates to `spawn_opencode()`. `process_line()` delegates to `process_line_opencode()`. `context_window()` uses `lookup_context_window()`. `slash_commands()` delegates to `get_opencode_commands()`. `supports_effort()` returns `false`.

Note: `spawn()` needs special logic to build the `provider/model` format and inject API key env vars. The `ProviderSpawnConfig.extra_env` carries the API key override.

- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat: implement OpenCodeProvider with tests"
```

---

### Task 5: Make `get_*_commands()` public in plugins.rs

**Files:**
- Modify: `tauri/src/commands/plugins.rs`

- [ ] **Step 1: Change `fn get_claude_commands`, `fn get_codex_commands`, `fn get_opencode_commands` from private to `pub fn`**

These are currently called only within `get_slash_commands()`. The `Provider` impls need to call them directly.

- [ ] **Step 2: Verify existing tests still pass**

Run: `cargo test -p orbit`

- [ ] **Step 3: Commit**

```bash
git commit -m "refactor: make per-provider command functions public"
```

---

### Task 6: Register providers in lib.rs and inject into SessionState

**Files:**
- Modify: `tauri/src/lib.rs`
- Modify: `tauri/src/ipc/session.rs` (add `ProviderRegistryState`)

- [ ] **Step 1: Create and register ProviderRegistry in app setup**

```rust
// In lib.rs setup closure, after session_manager:
use providers::{ProviderRegistry, claude::ClaudeProvider, codex::CodexProvider, opencode::OpenCodeProvider};

let mut registry = ProviderRegistry::new();
registry.register(Box::new(ClaudeProvider));
registry.register(Box::new(CodexProvider));
registry.register(Box::new(OpenCodeProvider));
app.manage(ProviderRegistryState(Arc::new(registry)));
```

- [ ] **Step 2: Define `ProviderRegistryState` in `ipc/session.rs`**

```rust
pub struct ProviderRegistryState(pub Arc<ProviderRegistry>);
```

- [ ] **Step 3: Verify app compiles**

Run: `cargo check`

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: register ProviderRegistry in Tauri app state"
```

---

### Task 7: Refactor SessionManager to use `&dyn Provider`

**Files:**
- Modify: `tauri/src/services/session_manager.rs`

This is the biggest task. Replace the 3 `do_spawn_*` methods + the `do_spawn` match with a single method that uses the trait.

- [ ] **Step 1: Add `ProviderRegistry` parameter to `do_spawn`**

Change signature:
```rust
pub fn do_spawn(
    manager: Arc<RwLock<SessionManager>>,
    app: AppHandle,
    session_id: SessionId,
    prompt: String,
    registry: &ProviderRegistry,
)
```

- [ ] **Step 2: Replace match dispatch with trait call**

```rust
let provider_id = { /* read from active session */ };
let provider = registry.resolve(&provider_id)
    .unwrap_or_else(|| registry.get("claude-code").unwrap());

// Set context window
if let Some(ctx) = provider.context_window(&model) {
    // set state.context_window
}

// Spawn
let handle = provider.spawn(ProviderSpawnConfig { ... })?;

// reader_loop with provider.process_line as the line processor
Self::reader_loop(..., |state, line| provider.process_line(state, line));
```

- [ ] **Step 3: Delete `do_spawn_claude`, `do_spawn_opencode`, `do_spawn_codex`**

All their logic is now in the Provider impls + the unified `do_spawn`.

- [ ] **Step 4: Update `load_session_journal` to use registry**

Replace the `match provider_owned.as_str()` with `registry.resolve(&provider_owned)`.

- [ ] **Step 5: Update call sites** (`create_session`, `send_message`)

Pass `&registry` to `do_spawn`. The `registry` comes from Tauri state in the IPC handlers.

- [ ] **Step 6: Run all tests**

Run: `cargo test -p orbit`
Expected: 99+ tests PASS

- [ ] **Step 7: Commit**

```bash
git commit -m "refactor: SessionManager uses Provider trait instead of match dispatch"
```

---

### Task 8: Refactor IPC and plugins to use Provider trait

**Files:**
- Modify: `tauri/src/ipc/session.rs`
- Modify: `tauri/src/commands/plugins.rs`

- [ ] **Step 1: `/effort` check uses `provider.supports_effort()`**

Replace `provider != "claude-code"` with `!provider.supports_effort()`.

- [ ] **Step 2: `get_slash_commands` delegates to `provider.slash_commands()`**

Replace the match with a registry lookup.

- [ ] **Step 3: Run full test suite + clippy**

Run: `cargo test && cargo clippy -- -D warnings`

- [ ] **Step 4: Commit**

```bash
git commit -m "refactor: IPC and plugins use Provider trait"
```

---

### Task 9: Clean up dead code

**Files:**
- Modify: `tauri/src/services/spawn_manager.rs` — remove `SpawnConfig`, `OpenCodeConfig`, `CodexConfig` (replaced by `ProviderSpawnConfig`)
- Modify: `tauri/src/journal/mod.rs` — remove re-exports of `process_line_codex`, `process_line_opencode` (called via trait now)
- Modify: `tauri/src/commands/providers.rs` — `lookup_context_window` and `codex_context_window` now called from Provider impls, may need visibility changes

- [ ] **Step 1: Remove unused types and imports**

- [ ] **Step 2: Run `cargo test && cargo clippy -- -D warnings`**

- [ ] **Step 3: Final commit**

```bash
git commit -m "chore: remove dead code after Provider trait refactor"
```

---

### Summary

| Task | What | Estimate |
|------|------|----------|
| 1 | Trait + Registry + mock tests | 5 min |
| 2 | ClaudeProvider | 5 min |
| 3 | CodexProvider | 5 min |
| 4 | OpenCodeProvider | 5 min |
| 5 | Make command fns public | 2 min |
| 6 | Register in lib.rs | 3 min |
| 7 | Refactor SessionManager (biggest) | 15 min |
| 8 | Refactor IPC/plugins | 5 min |
| 9 | Dead code cleanup | 3 min |
| **Total** | | **~48 min** |
