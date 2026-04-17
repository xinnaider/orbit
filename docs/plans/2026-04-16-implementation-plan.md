# Orbit — PTY, ACP, Attention, Permissions, Multi-Agent Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add interactive PTY terminal (portable-pty + xterm.js), ACP protocol support, attention/notification system, configurable permission bypass, sub-session multi-agent hierarchy, and timeline pagination to Orbit.

**Architecture:** Extend the existing Provider trait system with PTY-based spawning and ACP transport. Add attention state to Session/JournalState. Allow users to toggle `--dangerously-skip-permissions` per provider. Add `parent_session_id` for multi-agent. All changes are additive — existing stream-json mode keeps working.

**Tech Stack:** portable-pty 0.9, @xterm/xterm 5.5+, agent-client-protocol crate (Rust SDK), tauri-plugin-notification

---

## Phase 1: PTY + xterm.js (Interactive Terminal)

### Task 1: Add portable-pty dependency to Cargo.toml

**Files:**
- Modify: `tauri/Cargo.toml`

- [ ] **Step 1: Add portable-pty to dependencies**

Add to `[dependencies]` in `tauri/Cargo.toml`:
```toml
portable-pty = "0.9"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check --manifest-path tauri/Cargo.toml`
Expected: Compiles without errors

- [ ] **Step 3: Commit**

```
feat: add portable-pty dependency for PTY support
```

---

### Task 2: Create PtyManager service

**Files:**
- Create: `tauri/src/services/pty_manager.rs`
- Modify: `tauri/src/services/mod.rs` (add `pub mod pty_manager;`)
- Modify: `tauri/src/models.rs` (add PtySize type)

- [ ] **Step 1: Add PtySize type to models.rs**

Add after `pub type SessionId = i64;` in `tauri/src/models.rs`:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}
```

- [ ] **Step 2: Create pty_manager.rs**

Create `tauri/src/services/pty_manager.rs`:
```rust
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize as PtySizeCrate};
use tauri::{AppHandle, Emitter};

use crate::models::SessionId;
use crate::models::PtySize;

struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

pub struct PtyManager {
    sessions: HashMap<SessionId, PtySession>,
    app: AppHandle,
}

impl PtyManager {
    pub fn new(app: AppHandle) -> Self {
        PtyManager {
            sessions: HashMap::new(),
            app,
        }
    }

    pub fn create(
        &mut self,
        session_id: SessionId,
        command: &str,
        args: &[String],
        cwd: &str,
        env: Vec<(String, String)>,
        size: &PtySize,
    ) -> Result<u32, String> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySizeCrate {
                rows: size.rows,
                cols: size.cols,
                pixel_width: size.pixel_width,
                pixel_height: size.pixel_height,
            })
            .map_err(|e| format!("pty open failed: {e}"))?;

        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        cmd.cwd(cwd);

        for (k, v) in &env {
            cmd.env(k, v);
        }

        cmd.env("TERM", if cfg!(windows) { "cygwin" } else { "xterm-256color" });

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("pty spawn failed: {e}"))?;

        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("pty reader clone failed: {e}"))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("pty writer take failed: {e}"))?;

        let master = pair.master;

        let pid = child.process_id().map(|p| p as u32).unwrap_or(0);

        let app = self.app.clone();
        let sid = session_id;
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            let mut reader = reader;
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = app.emit("pty_output", serde_json::json!({
                            "sessionId": sid,
                            "data": "",
                            "eof": true,
                        }));
                        break;
                    }
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).into_owned();
                        let _ = app.emit("pty_output", serde_json::json!({
                            "sessionId": sid,
                            "data": data,
                            "eof": false,
                        }));
                    }
                    Err(_) => break,
                }
            }
        });

        self.sessions.insert(
            session_id,
            PtySession {
                master,
                writer,
                _child: child,
            },
        );

        Ok(pid)
    }

    pub fn write(&mut self, session_id: SessionId, data: &str) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or("pty session not found")?;
        session
            .writer
            .write_all(data.as_bytes())
            .map_err(|e| format!("pty write failed: {e}"))?;
        session.writer.flush().map_err(|e| format!("pty flush failed: {e}"))?;
        Ok(())
    }

    pub fn resize(&mut self, session_id: SessionId, size: &PtySize) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or("pty session not found")?;
        session
            .master
            .resize(PtySizeCrate {
                rows: size.rows,
                cols: size.cols,
                pixel_width: size.pixel_width,
                pixel_height: size.pixel_height,
            })
            .map_err(|e| format!("pty resize failed: {e}"))?;
        Ok(())
    }

    pub fn kill(&mut self, session_id: SessionId) -> Result<(), String> {
        if let Some(mut session) = self.sessions.remove(&session_id) {
            let _ = session.master.close();
            Ok(())
        } else {
            Err("pty session not found".to_string())
        }
    }

    pub fn is_active(&self, session_id: SessionId) -> bool {
        self.sessions.contains_key(&session_id)
    }
}
```

- [ ] **Step 3: Add `pub mod pty_manager;` to services/mod.rs**

- [ ] **Step 4: Verify compilation**

Run: `cargo check --manifest-path tauri/Cargo.toml`
Expected: Compiles without errors

- [ ] **Step 5: Commit**

```
feat: add PtyManager service for interactive terminal sessions
```

---

### Task 3: Create PTY IPC commands

**Files:**
- Create: `tauri/src/ipc/terminal.rs`
- Modify: `tauri/src/ipc/mod.rs` (add `pub mod terminal;`)
- Modify: `tauri/src/lib.rs` (add commands to invoke_handler + manage PtyManager state)

- [ ] **Step 1: Create IPC terminal commands**

Create `tauri/src/ipc/terminal.rs`:
```rust
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager, State};

use crate::models::{PtySize, SessionId};
use crate::services::pty_manager::PtyManager;

pub struct PtyManagerState(pub Arc<Mutex<PtyManager>>);

#[tauri::command]
pub fn pty_create(
    app: AppHandle,
    state: State<'_, PtyManagerState>,
    session_id: SessionId,
    command: String,
    args: Vec<String>,
    cwd: String,
    env: Vec<(String, String)>,
    rows: u16,
    cols: u16,
) -> Result<u32, String> {
    let mut mgr = state.0.lock().map_err(|e| format!("lock error: {e}"))?;
    mgr.create(
        session_id,
        &command,
        &args,
        &cwd,
        env,
        &PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        },
    )
}

#[tauri::command]
pub fn pty_write(
    state: State<'_, PtyManagerState>,
    session_id: SessionId,
    data: String,
) -> Result<(), String> {
    let mut mgr = state.0.lock().map_err(|e| format!("lock error: {e}"))?;
    mgr.write(session_id, &data)
}

#[tauri::command]
pub fn pty_resize(
    state: State<'_, PtyManagerState>,
    session_id: SessionId,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let mut mgr = state.0.lock().map_err(|e| format!("lock error: {e}"))?;
    mgr.resize(
        session_id,
        &PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        },
    )
}

#[tauri::command]
pub fn pty_kill(
    state: State<'_, PtyManagerState>,
    session_id: SessionId,
) -> Result<(), String> {
    let mut mgr = state.0.lock().map_err(|e| format!("lock error: {e}"))?;
    mgr.kill(session_id)
}
```

- [ ] **Step 2: Add `pub mod terminal;` to `tauri/src/ipc/mod.rs`**

- [ ] **Step 3: Wire PtyManager into lib.rs setup**

In `tauri/src/lib.rs`, inside `setup(|app| {`, after the provider registry, add:
```rust
            let pty_manager = Arc::new(Mutex::new(
                PtyManager::new(app.handle().clone()),
            ));
            app.manage(PtyManagerState(pty_manager));
```

Add to `invoke_handler`:
```rust
            ipc::terminal::pty_create,
            ipc::terminal::pty_write,
            ipc::terminal::pty_resize,
            ipc::terminal::pty_kill,
```

- [ ] **Step 4: Add imports to lib.rs**

Add to top of `tauri/src/lib.rs`:
```rust
use std::sync::Mutex;
use ipc::terminal::PtyManagerState;
use services::pty_manager::PtyManager;
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check --manifest-path tauri/Cargo.toml`
Expected: Compiles without errors

- [ ] **Step 6: Commit**

```
feat: add PTY IPC commands (create, write, resize, kill)
```

---

### Task 4: Install xterm.js dependencies

**Files:**
- Modify: `package.json`

- [ ] **Step 1: Install xterm packages**

Run:
```
npm install @xterm/xterm @xterm/addon-fit @xterm/addon-webgl @xterm/addon-unicode11
```

- [ ] **Step 2: Verify installation**

Run: `npm ls @xterm/xterm`
Expected: Shows installed version

- [ ] **Step 3: Commit**

```
chore: add xterm.js and addons for interactive terminal
```

---

### Task 5: Create TerminalPanel Svelte component

**Files:**
- Create: `ui/components/TerminalPanel.svelte`
- Create: `ui/lib/tauri/terminal.ts`

- [ ] **Step 1: Create Tauri terminal bridge**

Create `ui/lib/tauri/terminal.ts`:
```typescript
import { isMock } from './index'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface PtyOutputPayload {
  sessionId: number
  data: string
  eof: boolean
}

export async function ptyCreate(
  sessionId: number,
  command: string,
  args: string[],
  cwd: string,
  env: [string, string][],
  rows: number,
  cols: number,
): Promise<number> {
  if (isMock()) return 0
  return invoke<number>('pty_create', {
    sessionId,
    command,
    args,
    cwd,
    env,
    rows,
    cols,
  })
}

export async function ptyWrite(sessionId: number, data: string): Promise<void> {
  if (isMock()) return
  return invoke('pty_write', { sessionId, data })
}

export async function ptyResize(sessionId: number, rows: number, cols: number): Promise<void> {
  if (isMock()) return
  return invoke('pty_resize', { sessionId, rows, cols })
}

export async function ptyKill(sessionId: number): Promise<void> {
  if (isMock()) return
  return invoke('pty_kill', { sessionId })
}

export function onPtyOutput(cb: (payload: PtyOutputPayload) => void): Promise<UnlistenFn> {
  if (isMock()) return Promise.resolve(() => {})
  return listen<PtyOutputPayload>('pty_output', (event) => cb(event.payload))
}
```

- [ ] **Step 2: Create TerminalPanel component**

Create `ui/components/TerminalPanel.svelte`:
```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { Terminal } from '@xterm/xterm'
  import { FitAddon } from '@xterm/addon-fit'
  import { WebglAddon } from '@xterm/addon-webgl'
  import { Unicode11Addon } from '@xterm/addon-unicode11'
  import { ptyWrite, ptyResize, onPtyOutput } from '../lib/tauri/terminal'
  import '@xterm/xterm/css/xterm.css'

  let { sessionId, command, args, cwd, env = [] }: {
    sessionId: number
    command: string
    args: string[]
    cwd: string
    env?: [string, string][]
  } = $props()

  let container: HTMLDivElement | undefined = $state()
  let terminal: Terminal | undefined = $state()
  let fitAddon: FitAddon | undefined = $state()
  let unlisten: (() => void) | undefined = $state()

  onMount(async () => {
    if (!container) return

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: 'Cascadia Code, Fira Code, Consolas, monospace',
      scrollback: 5000,
    })

    const fit = new FitAddon()
    term.loadAddon(fit)
    term.loadAddon(new Unicode11Addon())

    try {
      term.loadAddon(new WebglAddon())
    } catch {
      // WebGL not available, canvas fallback
    }

    term.open(container)
    fit.fit()

    term.onData(async (data) => {
      try {
        await ptyWrite(sessionId, data)
      } catch (e) {
        console.error('pty write error:', e)
      }
    })

    term.onResize(async ({ cols, rows }) => {
      try {
        await ptyResize(sessionId, rows, cols)
      } catch (e) {
        console.error('pty resize error:', e)
      }
    })

    unlisten = await onPtyOutput(({ sessionId: sid, data, eof }) => {
      if (sid !== sessionId) return
      if (eof) {
        term?.writeln('\r\n[process exited]')
        return
      }
      term?.write(data)
    })

    terminal = term
    fitAddon = fit

    const observer = new ResizeObserver(() => {
      fit?.fit()
    })
    observer.observe(container)

    return () => observer.disconnect()
  })

  onDestroy(() => {
    unlisten?.()
    terminal?.dispose()
  })
</script>

<div class="terminal-panel" bind:this={container}></div>

<style>
  .terminal-panel {
    width: 100%;
    height: 100%;
    min-height: 200px;
  }

  .terminal-panel :global(.xterm) {
    height: 100%;
  }
</style>
```

- [ ] **Step 3: Verify lint passes**

Run: `npm run lint:ui`
Expected: No errors (or fix any that arise)

- [ ] **Step 4: Commit**

```
feat: add TerminalPanel Svelte component with xterm.js
```

---

### Task 6: Integrate TerminalPanel into session view

**Files:**
- Modify: `ui/components/CentralPanel.svelte` (add tab toggle Feed/Terminal)

- [ ] **Step 1: Read CentralPanel.svelte to understand current layout**

Read and understand the existing structure of `ui/components/CentralPanel.svelte`.

- [ ] **Step 2: Add tab toggle for Feed vs Terminal view**

Add a tab bar in CentralPanel header area that toggles between the existing Feed and the new TerminalPanel. When Terminal is selected, call `ptyCreate` if PTY is not yet active for this session. The exact integration depends on the current CentralPanel structure — wire `TerminalPanel` with the session's provider command, cwd, and args.

- [ ] **Step 3: Verify it renders**

Run: `npm run tauri:dev`
Expected: Terminal tab visible, xterm.js renders when selected

- [ ] **Step 4: Commit**

```
feat: integrate terminal panel into session view with tab toggle
```

---

## Phase 2: ACP (Agent Client Protocol)

### Task 7: Add agent-client-protocol dependency

**Files:**
- Modify: `tauri/Cargo.toml`

- [ ] **Step 1: Check crates.io for agent-client-protocol crate availability and version**

Run: `cargo search agent-client-protocol`
Expected: Shows available version

- [ ] **Step 2: Add dependency to Cargo.toml**

Add to `[dependencies]` in `tauri/Cargo.toml`:
```toml
agent-client-protocol = "0.1"
```

Note: If the crate name or version differs from search results, adjust accordingly. If the Rust SDK is not yet published as a crate, we will implement ACP JSON-RPC handling directly using serde_json (which is already a dependency).

- [ ] **Step 3: Verify compilation**

Run: `cargo check --manifest-path tauri/Cargo.toml`

- [ ] **Step 4: Commit**

```
feat: add agent-client-protocol dependency
```

---

### Task 8: Create ACP provider module

**Files:**
- Create: `tauri/src/providers/acp.rs`
- Modify: `tauri/src/providers/mod.rs` (add `pub mod acp;`)
- Modify: `tauri/src/lib.rs` (register ACP providers)

- [ ] **Step 1: Create ACP provider struct**

Create `tauri/src/providers/acp.rs` implementing the `Provider` trait. This provider communicates with agents that support the `--acp` flag via JSON-RPC 2.0 over stdio. The key difference from existing providers: instead of parsing stream-json output, we send structured `initialize`, `session/new`, `session/prompt` requests and parse `session/update` notifications.

The spawn function spawns the agent CLI with `--acp` flag, writes JSON-RPC initialize request to stdin, reads and processes JSON-RPC responses/notifications from stdout. The `process_line` method handles ACP `session/update` notifications and maps them to `JournalEntry` variants.

- [ ] **Step 2: Add `pub mod acp;` to providers/mod.rs**

Add after `pub mod opencode;` in `tauri/src/providers/mod.rs`.

- [ ] **Step 3: Register ACP providers in lib.rs**

In `tauri/src/lib.rs`, after existing provider registrations, add ACP-based providers for Gemini CLI, Copilot, etc. These will use `AcpProvider::new("gemini", "Gemini CLI", &["gemini", "--acp"])` etc.

- [ ] **Step 4: Verify compilation**

Run: `cargo check --manifest-path tauri/Cargo.toml`

- [ ] **Step 5: Commit**

```
feat: add ACP provider for agent-client-protocol support
```

---

### Task 9: Implement ACP permission handling

**Files:**
- Modify: `tauri/src/providers/acp.rs`
- Create: `ui/components/PermissionDialog.svelte`
- Modify: `ui/lib/tauri/terminal.ts` or create `ui/lib/tauri/acp.ts`

- [ ] **Step 1: Add permission request event**

When ACP provider receives `session/request_permission` JSON-RPC request, emit a `session:permission-request` Tauri event with `{ sessionId, toolCallId, options }`.

- [ ] **Step 2: Create permission dialog component**

Create `ui/components/PermissionDialog.svelte` — a modal that shows the tool call title, kind, and options (Allow/Deny). On user selection, calls `invoke('acp_respond_permission', { sessionId, requestId, optionId })`.

- [ ] **Step 3: Add ACP permission response IPC command**

Create `acp_respond_permission` command in the ACP provider that writes the JSON-RPC response back to the agent's stdin.

- [ ] **Step 4: Commit**

```
feat: add ACP permission request handling and dialog
```

---

## Phase 3: Attention / Notification System

### Task 10: Add AttentionState to models

**Files:**
- Modify: `tauri/src/models.rs`
- Modify: `tauri/src/journal/state.rs`

- [ ] **Step 1: Add AttentionReason and AttentionState to models.rs**

Add to `tauri/src/models.rs`:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AttentionReason {
    Permission,
    Completed,
    Error,
    RateLimit,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionState {
    pub requires_attention: bool,
    pub reason: Option<AttentionReason>,
    pub since: Option<String>,
}
```

- [ ] **Step 2: Add AttentionState to Session struct**

Add to `Session` in models.rs:
```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attention: Option<AttentionState>,
```

- [ ] **Step 3: Add attention to JournalState**

Add to `JournalState` in `tauri/src/journal/state.rs`:
```rust
pub attention: crate::models::AttentionState,
```

- [ ] **Step 4: Add attention to SessionStateEvent**

Add to `SessionStateEvent` in session_manager.rs:
```rust
pub attention: crate::models::AttentionState,
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check --manifest-path tauri/Cargo.toml`

- [ ] **Step 6: Commit**

```
feat: add AttentionState to Session, JournalState, and SessionStateEvent
```

---

### Task 11: Set attention on status transitions

**Files:**
- Modify: `tauri/src/journal/processor.rs` (set attention on status changes)
- Modify: `tauri/src/services/session_manager.rs` (set attention on session stop/error/rate-limit)

- [ ] **Step 1: Set attention in journal processors**

In each `process_line*` function, when status transitions to `Input` (permission needed), `Idle` (completed/turn finished), set `state.attention = AttentionState { requires_attention: true, reason: Some(Permission), since: Some(now) }`.

- [ ] **Step 2: Set attention on session stop/error/rate-limit**

In `session_manager.rs`, when a session stops (child exits), rate-limit is detected, or spawn fails, set the attention state.

- [ ] **Step 3: Clear attention on user focus**

Add a new IPC command `clear_attention(session_id)` that sets `attention.requires_attention = false`. The frontend calls this when the user clicks/focuses a session.

- [ ] **Step 4: Commit**

```
feat: set and clear attention on session status transitions
```

---

### Task 12: Add attention UI to sidebar and notifications

**Files:**
- Modify: `ui/components/Sidebar.svelte`
- Create: `ui/lib/tauri/attention.ts`
- Modify: `ui/lib/types.ts` (add AttentionReason, AttentionState)

- [ ] **Step 1: Add TS types for AttentionState**

Add to `ui/lib/types.ts`:
```typescript
export type AttentionReason = 'permission' | 'completed' | 'error' | 'rateLimit'

export interface AttentionState {
  requiresAttention: boolean
  reason: AttentionReason | null
  since: string | null
}
```

- [ ] **Step 2: Add attention Tauri bridge**

Create `ui/lib/tauri/attention.ts` with `clearAttention(sessionId)` and `onPermissionRequest(cb)`.

- [ ] **Step 3: Show attention badge in sidebar**

In `Sidebar.svelte`, show a colored dot next to session names that have `attention.requiresAttention === true`. Color by reason: amber=permission, green=completed, red=error, yellow=rateLimit.

- [ ] **Step 4: Add desktop notifications (optional)**

Use `@tauri-apps/plugin-notification` to send OS notifications when attention is set. Only notify if the app is not focused on that session.

- [ ] **Step 5: Commit**

```
feat: add attention badges in sidebar and desktop notifications
```

---

## Phase 4: Configurable Permission Bypass

### Task 13: Add skip_permissions field to Session and Provider

**Files:**
- Modify: `tauri/src/models.rs` (add skip_permissions to Session)
- Modify: `tauri/src/services/database.rs` (add column migration)
- Modify: `tauri/src/ipc/session.rs` (add parameter to create_session)
- Modify: `tauri/src/providers/claude.rs` (conditionally use --dangerously-skip-permissions)
- Modify: `tauri/src/providers/codex.rs` (conditionally use --dangerously-bypass-approvals-and-sandbox)
- Modify: `ui/components/NewSessionModal.svelte` (add toggle)

- [ ] **Step 1: Add skip_permissions column to DB**

In `tauri/src/services/database.rs`, add migration:
```sql
ALTER TABLE sessions ADD COLUMN skip_permissions INTEGER DEFAULT 1
```

- [ ] **Step 2: Add skip_permissions to Session struct**

Add to `Session` in models.rs:
```rust
pub skip_permissions: bool,
```

Default to `true` for backward compatibility (existing behavior).

- [ ] **Step 3: Update claude.rs to conditionally use bypass flag**

In `ClaudeProvider::spawn()`, only add `--dangerously-skip-permissions` when `config.skip_permissions == true`. When false, the agent will prompt for permissions through the normal flow (caught via ACP/PTY/attention system).

This requires adding `skip_permissions: bool` to `ProviderSpawnConfig`.

- [ ] **Step 4: Update codex.rs similarly**

In `CodexProvider::spawn()`, only add `--dangerously-bypass-approvals-and-sandbox` when skip_permissions is true.

- [ ] **Step 5: Add toggle to NewSessionModal**

Add a checkbox/switch in `NewSessionModal.svelte`: "Skip permissions (auto-approve all tool calls)". Default ON for Claude/Codex. When OFF, permission requests will appear as attention events.

- [ ] **Step 6: Verify**

Run: `cargo test --manifest-path tauri/Cargo.toml`
Run: `npm run lint`

- [ ] **Step 7: Commit**

```
feat: add configurable permission bypass per session
```

---

## Phase 5: Multi-Agent (Sub-sessions)

### Task 14: Add parent_session_id to DB and models

**Files:**
- Modify: `tauri/src/services/database.rs` (add columns)
- Modify: `tauri/src/models.rs` (add fields to Session)

- [ ] **Step 1: Add DB migration**

```sql
ALTER TABLE sessions ADD COLUMN parent_session_id INTEGER REFERENCES sessions(id)
ALTER TABLE sessions ADD COLUMN depth INTEGER DEFAULT 0
```

- [ ] **Step 2: Add fields to Session struct**

Add to `Session` in models.rs:
```rust
pub parent_session_id: Option<SessionId>,
pub depth: i32,
```

- [ ] **Step 3: Verify compilation and tests**

Run: `cargo test --manifest-path tauri/Cargo.toml`

- [ ] **Step 4: Commit**

```
feat: add parent_session_id and depth for multi-agent hierarchy
```

---

### Task 15: Detect sub-agent spawns and create child sessions

**Files:**
- Modify: `tauri/src/services/session_manager.rs`
- Modify: `tauri/src/journal/processor.rs`

- [ ] **Step 1: Detect Agent tool calls**

In `process_line` (Claude parser), when a `ToolCall` entry has `tool: "Agent"` or `tool: "Task"`, set a flag in JournalState indicating a sub-agent was spawned.

- [ ] **Step 2: Emit session:subagent-created event**

In `reader_loop`, after parsing, if the Agent tool call is detected, emit `session:subagent-created` event with `{ parentId, description, toolInput }`.

- [ ] **Step 3: Create child session in frontend**

In the frontend, on `session:subagent-created`, create a new child session row (with `parent_session_id` set). Show it nested under the parent in the sidebar with indentation.

- [ ] **Step 4: Commit**

```
feat: detect sub-agent spawns and create child sessions
```

---

### Task 16: Add nested session UI in sidebar

**Files:**
- Modify: `ui/components/Sidebar.svelte`

- [ ] **Step 1: Group sessions by parent**

In Sidebar, render sessions with `parent_session_id` as collapsible children under their parent session. Indent them, with a tree-line visual.

- [ ] **Step 2: Show sub-agent count badge**

Add a badge on parent sessions showing how many sub-agents they have (e.g. "3 agents").

- [ ] **Step 3: Verify visually**

Run: `npm run tauri:dev`

- [ ] **Step 4: Commit**

```
feat: add nested session UI for multi-agent hierarchy
```

---

## Phase 6: Timeline Pagination

### Task 17: Add seq and epoch to JournalEntry

**Files:**
- Modify: `tauri/src/models.rs` (add seq, epoch to JournalEntry)
- Modify: `tauri/src/journal/state.rs` (add seq counter)
- Modify: `tauri/src/services/database.rs` (add columns)

- [ ] **Step 1: Add fields to JournalEntry**

Add to `JournalEntry` in models.rs:
```rust
pub seq: u32,
pub epoch: String,
```

- [ ] **Step 2: Add seq counter to JournalState**

In JournalState, add `next_seq: u32` and `epoch: String`. When creating entries, assign sequential seq numbers. When session is resumed, generate a new epoch string.

- [ ] **Step 3: Add seq and epoch to session_outputs table**

Add DB migration for the new columns.

- [ ] **Step 4: Update get_outputs to support cursor pagination**

Add parameters to `get_outputs(session_id, cursor, limit, direction)` for cursor-based pagination.

- [ ] **Step 5: Commit**

```
feat: add seq/epoch to journal entries and cursor-based pagination
```

---

## Execution Order

The tasks are ordered by dependency:

1. **Task 1-6**: PTY + xterm.js (foundation — enables interactive terminal)
2. **Task 7-9**: ACP (builds on PTY for terminal delegation)
3. **Task 10-12**: Attention system (uses PTY/ACP for permission detection)
4. **Task 13**: Configurable permission bypass (depends on attention for non-bypass mode)
5. **Task 14-16**: Multi-agent sub-sessions (independent feature)
6. **Task 17**: Timeline pagination (independent, can be done anytime)

Quick wins that can go first: Task 10 (attention models) and Task 17 (timeline pagination) have no dependencies and can be done in parallel with PTY work.