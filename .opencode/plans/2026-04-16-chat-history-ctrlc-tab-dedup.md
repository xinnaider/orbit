# Chat History, Ctrl+C Kill, and Tab Dedup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add message history navigation (Arrow Up/Down), Ctrl+C to kill the agent process, and prevent duplicate session tabs in the workspace.

**Architecture:** Three independent features, each touching distinct areas. (1) InputBar gets a per-session message history ring buffer with Arrow Up/Down navigation. (2) InputBar captures Ctrl+C when focused and calls `stopSession`. (3) Workspace assignSession already deduplicates — verify current behavior works and ensure Sidebar click respects it.

**Tech Stack:** Svelte 5, TypeScript, Tauri IPC, Rust (no changes needed)

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `ui/components/InputBar.svelte` | Modify | Add history navigation, Ctrl+C handler |
| `ui/lib/stores/history.ts` | Create | Per-session message history store (ring buffer, localStorage-backed) |
| `ui/lib/mock/tauri-mock.ts` | Modify | No changes needed — `stop_session` mock already exists |
| `ui/components/Sidebar.svelte` | Verify | Ensure `assignSession` dedup works (already implemented) |

No Rust changes needed — `stopSession` IPC already exists and works for all providers.

---

### Task 1: Create message history store

**Files:**
- Create: `ui/lib/stores/history.ts`

- [ ] **Step 1: Create the history store module**

```typescript
// ui/lib/stores/history.ts
const MAX_HISTORY = 50;
const STORAGE_KEY = 'orbit:message-history';

interface SessionHistory {
  messages: string[];
  cursor: number;
  savedText: string;
}

function loadAll(): Record<string, string[]> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function persistAll(all: Record<string, string[]>) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(all));
  } catch {
    // localStorage full
  }
}

function createHistory() {
  const all = loadAll();
  const sessions: Record<string, SessionHistory> = {};

  function get(sessionId: string): SessionHistory {
    if (!sessions[sessionId]) {
      sessions[sessionId] = {
        messages: all[sessionId] ?? [],
        cursor: -1,
        savedText: '',
      };
    }
    return sessions[sessionId];
  }

  function push(sessionId: string, message: string) {
    const h = get(sessionId);
    if (h.messages.length > 0 && h.messages[0] === message) return;
    h.messages.unshift(message);
    if (h.messages.length > MAX_HISTORY) h.messages.length = MAX_HISTORY;
    h.cursor = -1;
    h.savedText = '';
    all[sessionId] = h.messages;
    persistAll(all);
  }

  function up(sessionId: string, currentText: string): string | null {
    const h = get(sessionId);
    if (h.cursor === -1) {
      h.savedText = currentText;
    }
    if (h.cursor < h.messages.length - 1) {
      h.cursor++;
      return h.messages[h.cursor];
    }
    return null;
  }

  function down(sessionId: string): string | null {
    const h = get(sessionId);
    if (h.cursor > 0) {
      h.cursor--;
      return h.messages[h.cursor];
    }
    if (h.cursor === 0) {
      h.cursor = -1;
      return h.savedText;
    }
    return null;
  }

  function resetCursor(sessionId: string) {
    const h = get(sessionId);
    h.cursor = -1;
    h.savedText = '';
  }

  return { push, up, down, resetCursor };
}

export const messageHistory = createHistory();
```

- [ ] **Step 2: Commit**

```bash
git add ui/lib/stores/history.ts
git commit -m "feat: add per-session message history store with localStorage persistence"
```

---

### Task 2: Wire history into InputBar with Arrow Up/Down

**Files:**
- Modify: `ui/components/InputBar.svelte:1-10` (imports), `:27-31` (state), `:366-372` (onKey handler), `:328-337` (send)

- [ ] **Step 1: Add history import**

In `InputBar.svelte`, add to the imports at top of `<script>`:

```typescript
import { messageHistory } from '../lib/stores/history';
```

- [ ] **Step 2: Add `stopSession` to the tauri import**

Find the existing import from `'../lib/tauri'` in InputBar.svelte and add `stopSession`:

```typescript
import { sendSessionMessage, updateSessionModel, updateSessionEffort, stopSession } from '../lib/tauri';
```

- [ ] **Step 3: Push message to history after successful send**

In the `send()` function, find the regular message path (after `await sendSessionMessage(sessionId, msg);` succeeds), add before `text = ''`:

```typescript
messageHistory.push(String(sessionId), msg);
```

The exact location: inside the `send()` function, the block where `text = ''` is set after a regular message send (not `/model`, `/effort`, or `/orchestrate` intercepts).

- [ ] **Step 4: Replace the `onKey` function**

Replace the existing `onKey` function (lines 366-372) with:

```typescript
function onKey(e: KeyboardEvent) {
  if (picker?.handleKey(e)) return;

  if (e.ctrlKey && e.key === 'c' && text === '') {
    e.preventDefault();
    stopSession(sessionId);
    return;
  }

  if (e.key === 'ArrowUp' && text === '') {
    e.preventDefault();
    const prev = messageHistory.up(String(sessionId), text);
    if (prev !== null) text = prev;
    return;
  }

  if (e.key === 'ArrowDown' && text === '') {
    e.preventDefault();
    const next = messageHistory.down(String(sessionId));
    if (next !== null) text = next;
    return;
  }

  if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') {
    messageHistory.resetCursor(String(sessionId));
  }

  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    send();
  }
}
```

- [ ] **Step 5: Commit**

```bash
git add ui/components/InputBar.svelte
git commit -m "feat: Arrow Up/Down message history and Ctrl+C to kill agent in InputBar"
```

---

### Task 3: Verify tab dedup works correctly

**Files:**
- Verify: `ui/lib/stores/workspace.ts:45-58` (assignSession)
- Verify: `ui/components/Sidebar.svelte:250-254` (click handler)

The current `assignSession` already deduplicates correctly:

```typescript
export function assignSession(paneId: string, sessionId: number): void {
  workspace.update((ws) => {
    for (const [pid, pane] of Object.entries(ws.panes)) {
      if (pane.sessionId === sessionId) {
        return { ...ws, focusedPaneId: pid }; // focus existing pane
      }
    }
    ws.panes[paneId] = { sessionId };
    ws.focusedPaneId = paneId;
    return ws;
  });
}
```

- [ ] **Step 1: Manual verification** — Click a session already open in another pane. Confirm it focuses the existing pane instead of duplicating.

- [ ] **Step 2: Commit** (only if a bug was found and fixed)

---

### Task 4: Run lint and type checks

- [ ] **Step 1: Run ESLint + svelte-check**

```bash
npx eslint ui/components/InputBar.svelte ui/lib/stores/history.ts --max-warnings 0
npx svelte-check --fail-on-warnings
```

Expected: 0 errors, 0 warnings

- [ ] **Step 2: Run cargo clippy + cargo test**

```bash
cd tauri && cargo clippy -- -D warnings && cargo test
```

Expected: 0 errors, all 126+ tests pass

- [ ] **Step 3: Commit if any fixes were needed**

---

## Scope Notes

- **Arrow Up/Down only works when input is empty** — avoids breaking text cursor movement in the textarea. Matches shell behavior.
- **Ctrl+C only kills when InputBar is focused AND text is empty** — prevents accidental kills when typing. Matches user's preference.
- **Tab dedup** — already working via `assignSession`. No code changes needed, just verification.
- **SSH sessions** — Ctrl+C uses `stopSession` which calls `kill_pid()` on the local SSH process, terminating the remote agent.
- **All providers** — same `stopSession` IPC path. PID-based kill works for Claude, Codex, OpenCode, ACP, and SSH.