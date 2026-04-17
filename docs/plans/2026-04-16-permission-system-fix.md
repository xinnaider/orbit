# Permission System Fix — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the half-implemented permission/approval system so users can approve/deny tool calls from ACP agents, and the banner displays meaningful information instead of "Allow ?".

**Architecture:** Store the ACP agent's stdin handle in ActiveSession instead of leaking it. When the user clicks Allow/Deny in the UI, write a JSON-RPC response back to the agent's stdin. For Claude Code sessions (journal-based detection), improve the approval message and display but keep it read-only since those sessions use `--dangerously-skip-permissions`.

**Tech Stack:** Rust (session_manager, acp.rs), Svelte 5, TypeScript, Tauri IPC

---

### Task 1: Store ACP stdin handle in ActiveSession

**Files:**
- Modify: `tauri/src/services/session_manager.rs:86-97` (ActiveSession struct)
- Modify: `tauri/src/providers/acp.rs:70-82` (stdin handling in spawn)
- Modify: `tauri/src/services/spawn_manager.rs:14-21` (SpawnHandle struct)

- [ ] **Step 1: Add stdin field to SpawnHandle**

In `tauri/src/services/spawn_manager.rs`, add an optional stdin handle to SpawnHandle:

```rust
pub struct SpawnHandle {
    pub pid: u32,
    pub reader: Box<dyn std::io::Read + Send>,
    pub stderr: Box<dyn std::io::Read + Send>,
    pub child: std::process::Child,
    /// Optional stdin for bidirectional protocols (ACP).
    pub stdin: Option<Box<dyn std::io::Write + Send>>,
    /// Keeps the askpass temp dir alive for the duration of an SSH session.
    pub _askpass: Option<tempfile::TempDir>,
}
```

- [ ] **Step 2: Update all SpawnHandle construction sites**

In `spawn_manager.rs` `spawn_claude()` — set `stdin: None` (Claude Code doesn't use stdin).

In `services/ssh.rs` wherever SpawnHandle is constructed — set `stdin: None`.

- [ ] **Step 3: Return stdin from ACP provider spawn**

In `tauri/src/providers/acp.rs`, stop leaking stdin. Instead, use a channel to get it back:

```rust
// In spawn():
let mut stdin = child.stdin.take().ok_or("no stdin")?;
let cwd_str = config.cwd.to_string_lossy().to_string();
let prompt = config.prompt.clone();
let session_id_str = format!("orbit-{}", config.session_id);

// Use a channel to return stdin after handshake
let (tx, rx) = std::sync::mpsc::channel();
std::thread::spawn(move || {
    let _ = write_jsonrpc_handshake(&mut stdin, &cwd_str, &prompt, &session_id_str);
    let _ = tx.send(stdin);
});

let stdout = child.stdout.take().ok_or("no stdout")?;
let stderr = child.stderr.take().ok_or("no stderr")?;

// Wait for handshake to complete (up to 5s)
let stdin_handle: Option<Box<dyn std::io::Write + Send>> =
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .ok()
        .map(|s| Box::new(s) as Box<dyn std::io::Write + Send>);

Ok(SpawnHandle {
    pid,
    reader: Box::new(stdout),
    stderr: Box::new(stderr),
    child,
    stdin: stdin_handle,
    _askpass: None,
})
```

- [ ] **Step 4: Store stdin in ActiveSession**

In `tauri/src/services/session_manager.rs`, add stdin to ActiveSession:

```rust
struct ActiveSession {
    session: Session,
    pub claude_session_id: Option<String>,
    pub effort: Option<String>,
    pub api_key: Option<String>,
    pub ssh_password: Option<String>,
    /// Stdin handle for bidirectional protocols (ACP). Protected by Mutex for thread-safe writes.
    pub stdin: Option<Arc<Mutex<Box<dyn std::io::Write + Send>>>>,
}
```

In `do_spawn`, after getting the handle, store the stdin:

```rust
let stdin_handle = handle.stdin.map(|s| Arc::new(Mutex::new(s)));
// ... later when updating active session:
// Store stdin for permission responses
{
    let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
    if let Some(a) = m.active.get_mut(&session_id) {
        a.stdin = stdin_handle;
    }
}
```

Initialize all `ActiveSession` construction sites with `stdin: None`.

- [ ] **Step 5: Verify compilation**

Run: `cargo check --manifest-path tauri/Cargo.toml`
Expected: compiles without errors.

---

### Task 2: Store JSON-RPC request ID for permission responses

**Files:**
- Modify: `tauri/src/journal/state.rs:10` (JournalState struct)
- Modify: `tauri/src/providers/acp.rs:251-266` (permission request parsing)

- [ ] **Step 1: Add permission_request_id to JournalState**

In `tauri/src/journal/state.rs`:

```rust
pub struct JournalState {
    // ... existing fields ...
    pub pending_approval: Option<String>,
    /// JSON-RPC request ID from the agent's permission request (for ACP responses).
    pub pending_approval_id: Option<serde_json::Value>,
    // ... rest ...
}
```

Initialize it as `None` in `Default::default()`.

- [ ] **Step 2: Store the request ID when parsing permission requests**

In `tauri/src/providers/acp.rs`, in the `requestPermission` handler:

```rust
if method == "requestPermission" || method == "client/requestPermission" {
    let request_id = msg.get("id").cloned();
    let tool = msg
        .pointer("/params/title")
        .or_else(|| msg.pointer("/params/name"))
        .and_then(|t| t.as_str())
        .unwrap_or("unknown tool");
    let desc = msg
        .pointer("/params/description")
        .and_then(|d| d.as_str())
        .unwrap_or("");

    let approval_text = if desc.is_empty() {
        format!("Allow {tool}?")
    } else {
        format!("Allow {tool}? {desc}")
    };

    state.pending_approval = Some(approval_text);
    state.pending_approval_id = request_id;
    // ... rest unchanged ...
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check --manifest-path tauri/Cargo.toml`
Expected: compiles without errors.

---

### Task 3: Add `respond_permission` Tauri command

**Files:**
- Modify: `tauri/src/services/session_manager.rs` (new method)
- Modify: `tauri/src/ipc/session.rs` (new IPC command)
- Modify: `tauri/src/lib.rs` (register command)

- [ ] **Step 1: Add respond_permission to SessionManager**

In `tauri/src/services/session_manager.rs`:

```rust
/// Respond to a pending ACP permission request by writing a JSON-RPC response to stdin.
pub fn respond_permission(&mut self, session_id: SessionId, allow: bool) -> Result<(), String> {
    let request_id = {
        let state = self.journal_states.get(&session_id)
            .ok_or("Session journal state not found")?;
        state.pending_approval_id.clone()
            .ok_or("No pending permission request")?
    };

    let stdin = {
        let a = self.active.get(&session_id)
            .ok_or("Session not active")?;
        a.stdin.clone()
            .ok_or("Session does not support interactive permissions (no stdin)")?
    };

    // Build JSON-RPC response
    let response = if allow {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": { "approved": true }
        })
    } else {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": { "approved": false }
        })
    };

    // Write to stdin
    {
        let mut writer = stdin.lock().map_err(|e| format!("stdin lock failed: {e}"))?;
        let line = serde_json::to_string(&response).map_err(|e| format!("serialize: {e}"))?;
        writer.write_all(line.as_bytes()).map_err(|e| format!("write: {e}"))?;
        writer.write_all(b"\n").map_err(|e| format!("write newline: {e}"))?;
        writer.flush().map_err(|e| format!("flush: {e}"))?;
    }

    // Clear pending state
    if let Some(state) = self.journal_states.get_mut(&session_id) {
        state.pending_approval = None;
        state.pending_approval_id = None;
        state.status = crate::models::AgentStatus::Working;
        state.attention = crate::models::AttentionState {
            requires_attention: false,
            reason: None,
            since: None,
        };
    }

    Ok(())
}
```

- [ ] **Step 2: Add IPC command in ipc/session.rs**

```rust
#[tauri::command]
pub fn respond_permission(
    session_id: SessionId,
    allow: bool,
    state: State<SessionState>,
) -> Result<(), IpcError> {
    state.write().respond_permission(session_id, allow)?;
    Ok(())
}
```

- [ ] **Step 3: Register the command in lib.rs**

Add `ipc::session::respond_permission` to the `.invoke_handler(tauri::generate_handler![...])` list.

- [ ] **Step 4: Verify compilation**

Run: `cargo check --manifest-path tauri/Cargo.toml`
Expected: compiles without errors.

---

### Task 4: Add frontend IPC wrapper and update PermissionDialog

**Files:**
- Modify: `ui/lib/tauri/attention.ts` (add respondPermission function)
- Modify: `ui/components/PermissionDialog.svelte` (update to call backend)

- [ ] **Step 1: Add respondPermission to tauri/attention.ts**

```typescript
export async function respondPermission(sessionId: number, allow: boolean): Promise<void> {
  return invoke('respond_permission', { sessionId, allow });
}
```

- [ ] **Step 2: Update PermissionDialog to call backend**

Replace the event-dispatcher approach with direct backend calls:

```svelte
<script lang="ts">
  import Modal from './Modal.svelte';
  import { respondPermission } from '../lib/tauri/attention';

  export let sessionId: number;
  export let toolName: string;
  export let description: string;
  export let onDismiss: () => void = () => {};

  async function handleAllow() {
    await respondPermission(sessionId, true);
    onDismiss();
  }

  async function handleDeny() {
    await respondPermission(sessionId, false);
    onDismiss();
  }
</script>

<Modal title="Permission Request" on:close={handleDeny}>
  <div class="perm-body">
    <div class="perm-tool">{toolName}</div>
    {#if description}
      <div class="perm-desc">{description}</div>
    {/if}
  </div>
  <div class="perm-actions">
    <button class="btn deny" on:click={handleDeny}>Deny</button>
    <button class="btn allow" on:click={handleAllow}>Allow</button>
  </div>
</Modal>
```

Keep the existing `<style>` block unchanged.

---

### Task 5: Wire PermissionDialog into CentralPanel

**Files:**
- Modify: `ui/components/CentralPanel.svelte` (import and render PermissionDialog)

- [ ] **Step 1: Replace the static approval banner with PermissionDialog**

In CentralPanel.svelte, import the component:

```typescript
import PermissionDialog from './PermissionDialog.svelte';
```

Replace the approval banner block:

```svelte
<!-- OLD: static banner -->
{#if session.pendingApproval && (session.status as string) !== 'working'}
  <div class="approval">
    <span class="approval-icon">⚑</span>
    <span class="approval-text">{session.pendingApproval}</span>
  </div>
{/if}
```

With:

```svelte
<!-- Approval dialog — inline for ACP sessions, banner for others -->
{#if session.pendingApproval && (session.status as string) !== 'working'}
  {#if session.attention?.reason === 'permission'}
    <PermissionDialog
      sessionId={session.id}
      toolName={parseToolName(session.pendingApproval)}
      description={parseToolDesc(session.pendingApproval)}
      onDismiss={() => {
        // State will be updated via session:state event from backend
      }}
    />
  {:else}
    <div class="approval">
      <span class="approval-icon">⚑</span>
      <span class="approval-text">{session.pendingApproval}</span>
    </div>
  {/if}
{/if}
```

- [ ] **Step 2: Add helper functions to parse tool name and description**

```typescript
function parseToolName(approval: string): string {
  // Format: "Allow <tool>? <desc>" or "Allow <tool>?"
  const match = approval.match(/^Allow\s+(.+?)\?/);
  return match ? match[1] : approval;
}

function parseToolDesc(approval: string): string {
  const match = approval.match(/^Allow\s+.+?\?\s*(.*)/);
  return match ? match[1].trim() : '';
}
```

- [ ] **Step 3: Verify with svelte-check**

Run: `npx svelte-check --tsconfig tsconfig.json`
Expected: zero errors.

---

### Task 6: Fix empty "Allow ?" message in journal detection

**Files:**
- Modify: `tauri/src/journal/state.rs:82-97` (detect_pending_approval)

- [ ] **Step 1: Improve detect_pending_approval message**

```rust
pub(crate) fn detect_pending_approval(entries: &[JournalEntry]) -> Option<String> {
    for entry in entries.iter().rev() {
        match entry.entry_type {
            JournalEntryType::ToolResult => return None,
            JournalEntryType::ToolCall => {
                let tool = entry.tool.as_deref().unwrap_or("tool");
                if tool == "Bash" {
                    return None;
                }
                let target = entry
                    .tool_input
                    .as_ref()
                    .and_then(|v| v.get("file_path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if target.is_empty() {
                    return Some(format!("Allow {tool}?"));
                }
                return Some(format!("Allow {tool} to {target}?"));
            }
            _ => {}
        }
    }
    None
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check --manifest-path tauri/Cargo.toml`
Expected: compiles without errors.

---

### Task 7: Render PermissionDialog inline (not modal) and remove old approval CSS

**Files:**
- Modify: `ui/components/PermissionDialog.svelte` (render as inline banner, not modal)

- [ ] **Step 1: Redesign PermissionDialog as an inline approval bar**

The current modal approach interrupts workflow. Make it an inline bar that sits where the approval banner was:

```svelte
<script lang="ts">
  import { respondPermission } from '../lib/tauri/attention';

  export let sessionId: number;
  export let toolName: string;
  export let description: string;
  export let onDismiss: () => void = () => {};

  let loading = false;

  async function handleAllow() {
    loading = true;
    try {
      await respondPermission(sessionId, true);
      onDismiss();
    } finally {
      loading = false;
    }
  }

  async function handleDeny() {
    loading = true;
    try {
      await respondPermission(sessionId, false);
      onDismiss();
    } finally {
      loading = false;
    }
  }
</script>

<div class="perm-bar">
  <span class="perm-icon">⚑</span>
  <div class="perm-info">
    <span class="perm-tool">{toolName}</span>
    {#if description}
      <span class="perm-desc">{description}</span>
    {/if}
  </div>
  <div class="perm-actions">
    <button class="btn deny" on:click={handleDeny} disabled={loading}>Deny</button>
    <button class="btn allow" on:click={handleAllow} disabled={loading}>Allow</button>
  </div>
</div>

<style>
  .perm-bar {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-3) var(--sp-5);
    background: rgba(255, 180, 50, 0.08);
    border-bottom: 1px solid rgba(255, 180, 50, 0.2);
    font-size: var(--sm);
  }

  .perm-icon {
    color: var(--s-input);
    font-size: var(--md);
    flex-shrink: 0;
  }

  .perm-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .perm-tool {
    font-weight: 600;
    color: var(--t0);
  }

  .perm-desc {
    color: var(--t1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .perm-actions {
    display: flex;
    gap: var(--sp-3);
    flex-shrink: 0;
  }

  .btn {
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: var(--sp-2) var(--sp-5);
    font-size: var(--xs);
    cursor: pointer;
    font-family: var(--mono);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .deny {
    background: none;
    color: var(--t1);
  }

  .deny:hover:not(:disabled) {
    border-color: var(--s-error);
    color: var(--s-error);
  }

  .allow {
    background: rgba(0, 212, 126, 0.1);
    color: var(--ac);
    border-color: var(--ac);
  }

  .allow:hover:not(:disabled) {
    background: rgba(0, 212, 126, 0.2);
  }
</style>
```

- [ ] **Step 2: Update CentralPanel to not wrap in Modal**

Since PermissionDialog is now inline (not a modal), the CentralPanel integration from Task 5 already works — it renders the bar inline where the old `.approval` div was. Remove the Modal import from PermissionDialog.svelte if it was there.

---

### Task 8: Commit and verify

- [ ] **Step 1: Run full lint check**

```bash
cargo clippy --manifest-path tauri/Cargo.toml -- -D warnings
npx svelte-check --tsconfig tsconfig.json --fail-on-warnings
```

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "feat: interactive permission approval for ACP agents

- Store stdin handle in ActiveSession for bidirectional ACP communication
- Add respond_permission Tauri command to approve/deny tool calls
- Replace static approval banner with inline PermissionDialog (Allow/Deny buttons)
- Fix empty 'Allow ?' message when description is missing
- Store JSON-RPC request ID to send proper responses back to agents"
```
