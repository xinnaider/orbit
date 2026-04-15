# OpenRouter Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add OpenRouter as an alternative LLM provider alongside Claude Code CLI, enabling Orbit to use any model available on OpenRouter.

**Architecture:** Provider is a session-level property stored in DB. A `spawn_openrouter()` function produces a `Box<dyn Read + Send>` reader that yields JSONL lines — same interface as Claude CLI stdout — so `reader_loop()` and `process_line()` are reused without changes. OpenRouter uses HTTP SSE streaming (OpenAI-compatible), converted to JSONL in the reader wrapper. API key stored per-session in memory (not DB).

**Tech Stack:** Rust (reqwest for HTTP), Tauri 2, Svelte 5, SQLite, OpenRouter API (OpenAI-compatible)

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `tauri/src/models.rs` | Modify | Add `provider` field to `Session` |
| `tauri/src/services/database.rs` | Modify | Migration + queries for `provider` column |
| `tauri/src/services/session_manager.rs` | Modify | `init_session()` accepts provider, `do_spawn()` dispatches by provider |
| `tauri/src/services/spawn_manager.rs` | Modify | Add `spawn_openrouter()` + SSE-to-JSONL reader wrapper |
| `tauri/src/ipc/session.rs` | Modify | `create_session` accepts `provider` param |
| `tauri/Cargo.toml` | Modify | Add `reqwest` dependency |
| `ui/lib/tauri.ts` | Modify | Add `provider` to `CreateSessionOptions` |
| `ui/lib/stores/sessions.ts` | Modify | Add `provider` to `Session` interface |
| `ui/components/NewSessionModal.svelte` | Modify | Add provider selector, conditional API key input |
| `ui/components/InputBar.svelte` | Modify | Disable Claude-specific commands for OpenRouter sessions |

---

### Task 1: Database Migration — Add `provider` Column

**Files:**
- Modify: `tauri/src/services/database.rs`

- [ ] **Step 1: Add migration for provider column**

In `database.rs`, after the existing ALTER TABLE migrations (around line 92), add:

```rust
let _ = conn.execute_batch("ALTER TABLE sessions ADD COLUMN provider TEXT DEFAULT 'claude-code'");
```

- [ ] **Step 2: Update `create_session()` to accept and store provider**

Change signature from:
```rust
pub fn create_session(
    &self,
    project_id: Option<i64>,
    name: Option<&str>,
    cwd: &str,
    permission_mode: &str,
    model: Option<&str>,
) -> SqlResult<SessionId>
```

To:
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

Update the INSERT to include `provider`:
```rust
conn.execute(
    "INSERT INTO sessions (project_id, name, cwd, status, permission_mode, model, provider) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    params![project_id, name, cwd, "initializing", permission_mode, model,
            provider.unwrap_or("claude-code")],
)?;
```

- [ ] **Step 3: Update `get_session()` and `get_sessions()` to read provider**

Add `provider` to the SELECT column list and map it to the Session struct field.

- [ ] **Step 4: Run `cargo test` to verify migration doesn't break existing tests**

Run: `cargo test -p orbit`
Expected: All tests pass (in-memory DB gets migration applied)

- [ ] **Step 5: Commit**

```bash
git add tauri/src/services/database.rs
git commit -m "feat: add provider column to sessions table with migration"
```

---

### Task 2: Rust Models — Add `provider` to Session Struct

**Files:**
- Modify: `tauri/src/models.rs`

- [ ] **Step 1: Add `provider` field to Session struct**

After the `model` field in the Session struct:
```rust
pub struct Session {
    // ... existing fields ...
    pub model: Option<String>,
    pub provider: String,  // "claude-code" | "openrouter"
    pub pid: Option<i32>,
    // ... rest ...
}
```

- [ ] **Step 2: Update Default / construction sites**

In `session_manager.rs` `init_session()`, where Session is constructed:
```rust
let session = Session {
    // ... existing fields ...
    provider: provider.unwrap_or("claude-code").to_string(),
    // ... rest ...
};
```

- [ ] **Step 3: Run clippy to find all construction sites that need updating**

Run: `cargo clippy -p orbit -- -D warnings`
Expected: Errors at every `Session { ... }` literal that's missing `provider`

- [ ] **Step 4: Fix all construction sites (tests, mocks, etc)**

Add `provider: "claude-code".to_string()` to each.

- [ ] **Step 5: Commit**

```bash
git add tauri/src/models.rs tauri/src/services/session_manager.rs
git commit -m "feat: add provider field to Session struct"
```

---

### Task 3: IPC Layer — Accept `provider` in create_session

**Files:**
- Modify: `tauri/src/ipc/session.rs`
- Modify: `tauri/src/services/session_manager.rs`

- [ ] **Step 1: Add `provider` param to `create_session` IPC command**

```rust
#[tauri::command]
pub fn create_session(
    project_path: String,
    prompt: String,
    model: Option<String>,
    permission_mode: Option<String>,
    session_name: Option<String>,
    use_worktree: Option<bool>,
    provider: Option<String>,  // NEW
    state: State<SessionState>,
    app: AppHandle,
) -> Result<Session, IpcError> {
```

Pass `provider.as_deref()` to `init_session()`.

- [ ] **Step 2: Update `init_session()` signature**

```rust
pub fn init_session(
    &mut self,
    project_path: &str,
    session_name: Option<&str>,
    permission_mode: &str,
    model: Option<&str>,
    use_worktree: bool,
    provider: Option<&str>,  // NEW
) -> Result<Session, String>
```

Pass `provider` to `db.create_session()`.

- [ ] **Step 3: Store provider in ActiveSession**

Add `provider: String` to `ActiveSession` struct:
```rust
struct ActiveSession {
    session: Session,
    pub claude_session_id: Option<String>,
    pub effort: Option<String>,
    pub api_key: Option<String>,  // For OpenRouter sessions
}
```

- [ ] **Step 4: Run clippy, fix all call sites**

Run: `cargo clippy -p orbit -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add tauri/src/ipc/session.rs tauri/src/services/session_manager.rs
git commit -m "feat: IPC layer accepts provider parameter for session creation"
```

---

### Task 4: Frontend — Add `provider` to Types and IPC

**Files:**
- Modify: `ui/lib/tauri.ts`
- Modify: `ui/lib/stores/sessions.ts`

- [ ] **Step 1: Add `provider` to CreateSessionOptions**

```typescript
export interface CreateSessionOptions {
  projectPath: string;
  prompt: string;
  model?: string;
  permissionMode?: 'ignore' | 'approve';
  sessionName?: string;
  useWorktree?: boolean;
  provider?: string;  // NEW: 'claude-code' | 'openrouter'
}
```

- [ ] **Step 2: Pass provider in createSession()**

```typescript
export async function createSession(opts: CreateSessionOptions): Promise<Session> {
  return await invoke('create_session', {
    projectPath: opts.projectPath,
    prompt: opts.prompt,
    model: opts.model ?? null,
    permissionMode: opts.permissionMode ?? 'ignore',
    sessionName: opts.sessionName ?? null,
    useWorktree: opts.useWorktree ?? false,
    provider: opts.provider ?? 'claude-code',  // NEW
  });
}
```

- [ ] **Step 3: Add `provider` to Session interface**

```typescript
export interface Session {
  // ... existing fields ...
  provider: string;  // 'claude-code' | 'openrouter'
  // ...
}
```

- [ ] **Step 4: Run svelte-check**

Run: `npx svelte-check --fail-on-warnings`
Expected: Errors in mock and test files that need `provider` field

- [ ] **Step 5: Fix mock data and tests**

Add `provider: 'claude-code'` to all mock Session objects in `tauri-mock.ts` and `sessions.test.ts`.

- [ ] **Step 6: Commit**

```bash
git add ui/lib/tauri.ts ui/lib/stores/sessions.ts ui/lib/mock/tauri-mock.ts ui/lib/stores/sessions.test.ts
git commit -m "feat: frontend types support provider field"
```

---

### Task 5: NewSessionModal — Provider Selector UI

**Files:**
- Modify: `ui/components/NewSessionModal.svelte`

- [ ] **Step 1: Add provider state and selector**

Add state variable:
```typescript
let provider = 'claude-code';
let apiKey = '';
```

Add selector above model dropdown:
```svelte
<div class="field">
  <label>Provider</label>
  <select bind:value={provider}>
    <option value="claude-code">Claude Code</option>
    <option value="openrouter">OpenRouter</option>
  </select>
</div>

{#if provider === 'openrouter'}
  <div class="field">
    <label>API Key</label>
    <input type="password" bind:value={apiKey} placeholder="sk-or-..." />
  </div>
{/if}
```

- [ ] **Step 2: Conditionally show model field**

For Claude Code: keep existing model dropdown (auto/sonnet/opus/haiku).
For OpenRouter: show a text input for model name (free-form, since there are hundreds):

```svelte
{#if provider === 'claude-code'}
  <div class="field">
    <label>Model</label>
    <select bind:value={model}>
      {#each models as m}
        <option value={m.v}>{m.l}</option>
      {/each}
    </select>
  </div>
{:else}
  <div class="field">
    <label>Model</label>
    <input type="text" bind:value={model} placeholder="anthropic/claude-sonnet-4" />
  </div>
{/if}
```

- [ ] **Step 3: Pass provider to createSession**

```typescript
await createSession({
  projectPath: path.trim(),
  prompt: prompt.trim() || 'Hello',
  model: model === 'auto' ? undefined : model,
  permissionMode: 'ignore',
  sessionName: finalName,
  useWorktree: provider === 'claude-code' ? useWorktree : false,
  provider,
});
```

- [ ] **Step 4: Disable worktree toggle for OpenRouter**

Worktree is a git feature tied to Claude Code's file operations. Disable when provider is OpenRouter:
```svelte
{#if provider === 'claude-code'}
  <label class="toggle">
    <input type="checkbox" bind:checked={useWorktree} />
    Use worktree
  </label>
{/if}
```

- [ ] **Step 5: Run svelte-check**

Run: `npx svelte-check --fail-on-warnings`
Expected: 0 errors

- [ ] **Step 6: Commit**

```bash
git add ui/components/NewSessionModal.svelte
git commit -m "feat: provider selector in NewSessionModal with OpenRouter API key input"
```

---

### Task 6: Add reqwest Dependency

**Files:**
- Modify: `tauri/Cargo.toml`

- [ ] **Step 1: Add reqwest with streaming support**

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json", "stream"] }
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p orbit`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add tauri/Cargo.toml tauri/Cargo.lock
git commit -m "chore: add reqwest HTTP client dependency"
```

---

### Task 7: OpenRouter Spawn — HTTP SSE to Reader

**Files:**
- Modify: `tauri/src/services/spawn_manager.rs`

This is the core task. `spawn_openrouter()` sends HTTP request to OpenRouter API, reads SSE stream, converts each SSE chunk to a JSONL line compatible with `process_line()`.

- [ ] **Step 1: Define OpenRouter config struct**

```rust
pub struct OpenRouterConfig {
    pub session_id: crate::models::SessionId,
    pub api_key: String,
    pub model: String,
    pub messages: Vec<serde_json::Value>,  // conversation history
}
```

- [ ] **Step 2: Implement SSE-to-JSONL reader wrapper**

The key insight: `reader_loop()` expects `Box<dyn Read + Send>` that yields lines. We create a pipe — one thread reads SSE and writes JSONL to pipe, `reader_loop()` reads from the other end.

```rust
use std::io::{Read, Write};
use std::sync::mpsc;

pub struct OpenRouterHandle {
    pub reader: Box<dyn Read + Send>,
    pub abort_tx: mpsc::Sender<()>,
}

pub fn spawn_openrouter(config: OpenRouterConfig) -> Result<OpenRouterHandle, String> {
    let (read_end, write_end) = os_pipe::pipe()
        .map_err(|e| format!("pipe failed: {e}"))?;

    let (abort_tx, abort_rx) = mpsc::channel::<()>();

    let api_key = config.api_key.clone();
    let model = config.model.clone();
    let messages = config.messages.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let client = reqwest::Client::new();
            let body = serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": true,
            });

            let resp = match client
                .post("https://openrouter.ai/api/v1/chat/completions")
                .header("Authorization", format!("Bearer {api_key}"))
                .header("X-Title", "Orbit")
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let err_line = serde_json::json!({
                        "type": "system",
                        "subtype": "error",
                        "message": { "content": [{ "type": "text", "text": format!("OpenRouter error: {e}") }] }
                    });
                    let _ = writeln!(&write_end, "{}", err_line);
                    return;
                }
            };

            let mut stream = resp.bytes_stream();
            let mut buf = String::new();
            let mut writer = std::io::BufWriter::new(&write_end);
            let mut model_name: Option<String> = None;
            let mut input_tokens: u64 = 0;
            let mut output_tokens: u64 = 0;

            use futures_util::StreamExt;
            while let Some(chunk) = stream.next().await {
                if abort_rx.try_recv().is_ok() { break; }

                let bytes = match chunk {
                    Ok(b) => b,
                    Err(_) => break,
                };

                buf.push_str(&String::from_utf8_lossy(&bytes));

                // Parse SSE lines: "data: {...}\n\n"
                while let Some(pos) = buf.find("\n\n") {
                    let event = buf[..pos].to_string();
                    buf = buf[pos + 2..].to_string();

                    for line in event.lines() {
                        let data = match line.strip_prefix("data: ") {
                            Some(d) => d.trim(),
                            None => continue,
                        };

                        if data == "[DONE]" {
                            // Emit result line
                            let result_line = serde_json::json!({
                                "type": "result",
                                "subtype": "success",
                                "is_error": false,
                                "result": "",
                                "stop_reason": "end_turn",
                            });
                            let _ = writeln!(writer, "{}", result_line);
                            let _ = writer.flush();
                            return;
                        }

                        if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(data) {
                            // Extract model from first chunk
                            if model_name.is_none() {
                                model_name = chunk.get("model")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                            }

                            // Extract usage if present
                            if let Some(usage) = chunk.get("usage") {
                                input_tokens = usage.get("prompt_tokens")
                                    .and_then(|v| v.as_u64()).unwrap_or(input_tokens);
                                output_tokens = usage.get("completion_tokens")
                                    .and_then(|v| v.as_u64()).unwrap_or(output_tokens);
                            }

                            // Convert to Claude-compatible JSONL
                            if let Some(choices) = chunk.get("choices").and_then(|v| v.as_array()) {
                                for choice in choices {
                                    let delta = match choice.get("delta") {
                                        Some(d) => d,
                                        None => continue,
                                    };

                                    if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                                        if !content.is_empty() {
                                            let assistant_line = serde_json::json!({
                                                "type": "assistant",
                                                "message": {
                                                    "model": model_name.as_deref().unwrap_or(&model),
                                                    "content": [{ "type": "text", "text": content }],
                                                    "usage": {
                                                        "input_tokens": input_tokens,
                                                        "output_tokens": output_tokens,
                                                    },
                                                }
                                            });
                                            let _ = writeln!(writer, "{}", assistant_line);
                                            let _ = writer.flush();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    });

    Ok(OpenRouterHandle {
        reader: Box::new(read_end),
        abort_tx,
    })
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p orbit`

- [ ] **Step 4: Commit**

```bash
git add tauri/src/services/spawn_manager.rs
git commit -m "feat: spawn_openrouter — HTTP SSE to JSONL reader for OpenRouter API"
```

---

### Task 8: Session Manager — Provider Dispatch

**Files:**
- Modify: `tauri/src/services/session_manager.rs`

- [ ] **Step 1: Update `do_spawn()` to dispatch by provider**

In `do_spawn()`, after extracting session config, add provider dispatch:

```rust
let provider = {
    let m = manager.read().unwrap_or_else(|e| e.into_inner());
    m.active.get(&session_id)
        .map(|a| a.session.provider.clone())
        .unwrap_or_else(|| "claude-code".to_string())
};

match provider.as_str() {
    "openrouter" => {
        // Build conversation history from journal state
        let messages = {
            let m = manager.read().unwrap_or_else(|e| e.into_inner());
            let state = m.journal_states.get(&session_id);
            build_openrouter_messages(state, &prompt_text)
        };

        let api_key = {
            let m = manager.read().unwrap_or_else(|e| e.into_inner());
            m.active.get(&session_id)
                .and_then(|a| a.api_key.clone())
                .unwrap_or_default()
        };

        let or_config = OpenRouterConfig {
            session_id,
            api_key,
            model: model.unwrap_or_else(|| "anthropic/claude-sonnet-4".to_string()),
            messages,
        };

        let handle = match spawn_openrouter(or_config) {
            Ok(h) => h,
            Err(e) => {
                let _ = db.update_session_status(session_id, SessionStatus::Error);
                let _ = app.emit("session:error",
                    serde_json::json!({ "sessionId": session_id, "error": e }));
                return;
            }
        };

        // No PID for HTTP sessions
        let _ = app.emit("session:running",
            serde_json::json!({ "sessionId": session_id, "pid": null }));

        // Emit user message
        // ... (same as existing code)

        // Use a dummy child process for reader_loop compatibility
        // OR refactor reader_loop to not need Child
        Self::reader_loop_stream(
            Arc::clone(&manager), app, session_id,
            handle.reader, db,
        );
    }
    _ => {
        // Existing Claude Code spawn logic (unchanged)
        let config = SpawnConfig { session_id, cwd, permission_mode, model, effort, prompt, claude_session_id };
        // ... existing spawn_claude() flow
    }
}
```

- [ ] **Step 2: Add `build_openrouter_messages()` helper**

Converts `JournalState` entries to OpenAI messages format:

```rust
fn build_openrouter_messages(
    state: Option<&JournalState>,
    new_prompt: &str,
) -> Vec<serde_json::Value> {
    let mut messages = vec![];

    if let Some(state) = state {
        for entry in &state.entries {
            match entry.entry_type {
                JournalEntryType::User => {
                    if let Some(ref text) = entry.text {
                        messages.push(serde_json::json!({
                            "role": "user",
                            "content": text,
                        }));
                    }
                }
                JournalEntryType::Assistant => {
                    if let Some(ref text) = entry.text {
                        messages.push(serde_json::json!({
                            "role": "assistant",
                            "content": text,
                        }));
                    }
                }
                _ => {} // Skip tool calls, thinking, etc
            }
        }
    }

    // Add new prompt as user message
    messages.push(serde_json::json!({
        "role": "user",
        "content": new_prompt,
    }));

    messages
}
```

- [ ] **Step 3: Add `reader_loop_stream()` — variant without Child process**

```rust
fn reader_loop_stream(
    manager: Arc<RwLock<SessionManager>>,
    app: AppHandle,
    session_id: SessionId,
    reader: Box<dyn std::io::Read + Send>,
    db: Arc<DatabaseService>,
) {
    // Same as reader_loop() but without child.wait() at the end
    use std::io::BufRead;
    let mut reader = std::io::BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                // ... identical parsing logic as reader_loop() ...
            }
        }
    }

    // Mark session completed (no child to wait)
    {
        let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
        if let Some(a) = m.active.get_mut(&session_id) {
            a.session.status = SessionStatus::Completed;
        }
        if let Some(state) = m.journal_states.get_mut(&session_id) {
            state.status = AgentStatus::Idle;
        }
        let _ = db.update_session_status(session_id, SessionStatus::Completed);
    }

    let _ = app.emit("session:stopped",
        serde_json::json!({ "sessionId": session_id }));
}
```

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p orbit -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add tauri/src/services/session_manager.rs
git commit -m "feat: session manager dispatches by provider — OpenRouter uses HTTP streaming"
```

---

### Task 9: InputBar — Provider-Aware Commands

**Files:**
- Modify: `ui/components/InputBar.svelte`

- [ ] **Step 1: Accept provider prop**

```typescript
export let provider: string = 'claude-code';
```

- [ ] **Step 2: Disable Claude-specific commands for OpenRouter**

```typescript
const ORBIT_COMMANDS: SlashCommand[] = [
  { cmd: '/model', desc: 'Switch model', category: 'orbit' },
  ...(provider === 'claude-code'
    ? [{ cmd: '/effort', desc: 'Set thinking effort (low, medium, high, max)', category: 'orbit' }]
    : []),
];
```

- [ ] **Step 3: Skip effort interception for non-Claude providers**

In `send()`, wrap the `/effort` handler:
```typescript
if (cmd === '/effort' && provider === 'claude-code') {
  // ... existing handler
}
```

- [ ] **Step 4: Pass provider from CentralPanel/Pane**

Update the parent component that renders InputBar to pass `provider={session.provider}`.

- [ ] **Step 5: Run svelte-check**

Run: `npx svelte-check --fail-on-warnings`

- [ ] **Step 6: Commit**

```bash
git add ui/components/InputBar.svelte ui/components/Pane.svelte
git commit -m "feat: InputBar adapts commands based on session provider"
```

---

### Task 10: API Key Persistence (Secure)

**Files:**
- Modify: `tauri/src/ipc/session.rs`
- Modify: `tauri/src/services/session_manager.rs`

- [ ] **Step 1: Add `set_api_key` IPC command**

```rust
#[tauri::command]
pub fn set_session_api_key(
    session_id: SessionId,
    api_key: String,
    state: State<SessionState>,
) -> Result<(), IpcError> {
    let mut m = state.write();
    if let Some(a) = m.active.get_mut(&session_id) {
        a.api_key = Some(api_key);
    }
    Ok(())
}
```

API key lives ONLY in memory — never persisted to DB.

- [ ] **Step 2: Register command in lib.rs**

Add `ipc::session::set_session_api_key` to `generate_handler![]`.

- [ ] **Step 3: Add frontend wrapper**

In `ui/lib/tauri.ts`:
```typescript
export async function setSessionApiKey(sessionId: number, apiKey: string): Promise<void> {
  await invoke('set_session_api_key', { sessionId, apiKey });
}
```

- [ ] **Step 4: Call after session creation in NewSessionModal**

```typescript
const session = await createSession({ ...opts, provider });
if (provider === 'openrouter' && apiKey) {
  await setSessionApiKey(session.id, apiKey);
}
```

- [ ] **Step 5: Commit**

```bash
git add tauri/src/ipc/session.rs tauri/src/lib.rs ui/lib/tauri.ts ui/components/NewSessionModal.svelte
git commit -m "feat: secure API key storage — in-memory only, never persisted to DB"
```

---

### Task 11: Integration Test — End-to-End OpenRouter Session

**Files:**
- Manual testing

- [ ] **Step 1: Build and run the app**

Run: `npm run tauri:dev`

- [ ] **Step 2: Create OpenRouter session**

1. Click "+" to create new session
2. Select "OpenRouter" provider
3. Enter API key
4. Enter model name (e.g., `anthropic/claude-sonnet-4`)
5. Enter prompt: "Say hello"
6. Click Create

- [ ] **Step 3: Verify session lifecycle**

Expected:
- Session appears in sidebar with status "initializing" → "running"
- Response streams into feed
- Model detected and shown in header and MetaPanel
- Token counts update
- Session completes with "idle" status

- [ ] **Step 4: Test follow-up message**

Type a follow-up message. Verify:
- Previous context is included (conversation history)
- Response streams correctly

- [ ] **Step 5: Test alongside Claude Code session**

Create a second session with "Claude Code" provider. Verify both work simultaneously.

- [ ] **Step 6: Commit any fixes found during testing**

```bash
git add -A
git commit -m "fix: integration test fixes for OpenRouter provider"
```

---

Plan complete and saved to `docs/superpowers/plans/2026-04-12-openrouter-provider.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?