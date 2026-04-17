# @ File Picker, Auto-Scroll, Sound Notification & Approval Fix — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `@` file context picker in chat input, auto-scroll on message send, sound notification when agents request permission, and fix approval banner not showing.

**Architecture:** The file picker uses a new Tauri command (`list_project_files`) that walks the agent's `cwd` respecting `.gitignore` via the `ignore` crate. The frontend caches the file list per session and shows an autocomplete dropdown (same pattern as existing `/` slash commands). Auto-scroll reacts to pending message additions. Sound uses Web Audio API. Approval fix removes the redundant status check in the frontend.

**Tech Stack:** Svelte 5 (legacy `export let` props), Rust/Tauri 2, TypeScript, Web Audio API

---

## File Structure

| File | Responsibility |
|------|---------------|
| **Modify:** `src-tauri/Cargo.toml` | Add `ignore` crate dependency |
| **Modify:** `src-tauri/src/commands.rs` | Add `list_project_files` command |
| **Modify:** `src-tauri/src/lib.rs` | Register `list_project_files` |
| **Modify:** `src/lib/tauri.ts` | Add `listProjectFiles()` binding |
| **Modify:** `src/components/CommandInput.svelte` | `@` autocomplete logic + dropdown |
| **Modify:** `src/components/CentralPanel.svelte` | Auto-scroll on pending + fix approval banner |
| **Modify:** `src/App.svelte` | Sound notification on agent `input` transition |

---

### Task 1: Add `ignore` Crate and `list_project_files` Rust Command

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `ignore` crate to Cargo.toml**

In `src-tauri/Cargo.toml`, add to `[dependencies]`:

```toml
ignore = "0.4"
```

- [ ] **Step 2: Add `list_project_files` command to `commands.rs`**

Add at the end of the file (before the closing), after `get_slash_commands`:

```rust
#[tauri::command]
pub fn list_project_files(cwd: String) -> Vec<String> {
    use ignore::WalkBuilder;

    let mut files = Vec::new();
    let walker = WalkBuilder::new(&cwd)
        .hidden(true)        // skip hidden files
        .git_ignore(true)    // respect .gitignore
        .git_global(true)
        .git_exclude(true)
        .max_depth(Some(12))
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }
        if let Ok(rel) = entry.path().strip_prefix(&cwd) {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if !rel_str.is_empty() {
                files.push(rel_str.to_string());
            }
        }
        if files.len() >= 5000 {
            break;
        }
    }

    files.sort();
    files
}
```

- [ ] **Step 3: Register command in `lib.rs`**

In `src-tauri/src/lib.rs`, add `commands::list_project_files` to the invoke handler. The block becomes:

```rust
        .invoke_handler(tauri::generate_handler![
            commands::send_keystroke,
            commands::send_message,
            commands::get_journal,
            commands::get_diff,
            commands::get_file_versions,
            commands::get_subagent_journal,
            commands::get_slash_commands,
            commands::list_project_files,
        ])
```

- [ ] **Step 4: Build to verify**

Run: `cd C:\Users\fernandonepen\Documents\agent-dashboard-v2 && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: `Finished` with no errors

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit --author="Fernando <fernandoschnneider@gmail.com>" -m "feat: add list_project_files Tauri command with gitignore support"
```

---

### Task 2: Frontend Binding for `listProjectFiles`

**Files:**
- Modify: `src/lib/tauri.ts`

- [ ] **Step 1: Add `listProjectFiles` function to `tauri.ts`**

Add after the `getSlashCommands()` function:

```typescript
export async function listProjectFiles(cwd: string): Promise<string[]> {
  return await invoke('list_project_files', { cwd });
}
```

- [ ] **Step 2: Verify no TypeScript errors**

Run: `cd C:\Users\fernandonepen\Documents\agent-dashboard-v2 && npx svelte-check --threshold error 2>&1 | tail -3`
Expected: 0 ERRORS

- [ ] **Step 3: Commit**

```bash
git add src/lib/tauri.ts
git commit --author="Fernando <fernandoschnneider@gmail.com>" -m "feat: add listProjectFiles frontend binding"
```

---

### Task 3: `@` File Autocomplete in CommandInput

**Files:**
- Modify: `src/components/CommandInput.svelte`

This is the largest change. The CommandInput already has `/` slash command autocomplete. We add a parallel `@` file autocomplete that reuses the same dropdown pattern.

- [ ] **Step 1: Replace the full `<script>` block in `CommandInput.svelte`**

Replace the entire `<script lang="ts">` block (lines 1-108) with:

```svelte
<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { sendKeystroke, sendMessage, getSlashCommands, listProjectFiles } from '../lib/tauri';
  import { pendingMessages } from '../lib/stores/journal';
  import type { SlashCommand } from '../lib/types';

  export let sessionId: string;
  export let agentName: string;
  export let agentCwd: string = '';

  let inputText = '';
  let textareaEl: HTMLTextAreaElement;
  let selectedIdx = 0;
  let showSuggestions = false;
  let commands: SlashCommand[] = [];
  let suggestionEls: HTMLButtonElement[] = [];

  // @ file picker state
  let projectFiles: string[] = [];
  let showFilePicker = false;
  let fileSelectedIdx = 0;
  let filePickerEls: HTMLButtonElement[] = [];

  onMount(async () => {
    try {
      commands = await getSlashCommands();
    } catch {
      // Fallback: empty
    }
    if (agentCwd) {
      try {
        projectFiles = await listProjectFiles(agentCwd);
      } catch {
        // Fallback: empty
      }
    }
  });

  // Slash command filtering
  $: query = inputText.startsWith('/') ? inputText.toLowerCase() : '';
  $: filtered = query
    ? commands.filter(c => c.cmd.toLowerCase().includes(query))
    : [];
  $: {
    showSuggestions = filtered.length > 0 && inputText.startsWith('/');
    if (selectedIdx >= filtered.length) selectedIdx = 0;
  }

  // @ file picker: detect "@" token at cursor position
  function getAtQuery(): string | null {
    if (!textareaEl) return null;
    const pos = textareaEl.selectionStart;
    const textBefore = inputText.slice(0, pos);
    // Find the last @ that isn't preceded by a non-space char
    const lastAt = textBefore.lastIndexOf('@');
    if (lastAt === -1) return null;
    // @ must be at start or preceded by whitespace
    if (lastAt > 0 && inputText[lastAt - 1] !== ' ' && inputText[lastAt - 1] !== '\n') return null;
    const queryStr = textBefore.slice(lastAt + 1);
    // If there's a space after the query, the user finished typing the path
    if (queryStr.includes(' ')) return null;
    return queryStr;
  }

  $: atQuery = (() => {
    // Re-run when inputText changes
    void inputText;
    return getAtQuery();
  })();

  $: filteredFiles = (() => {
    if (atQuery === null) return [];
    if (atQuery === '') return projectFiles.slice(0, 15);
    const q = atQuery.toLowerCase();
    const matches: string[] = [];
    for (const f of projectFiles) {
      if (f.toLowerCase().includes(q)) {
        matches.push(f);
        if (matches.length >= 15) break;
      }
    }
    return matches;
  })();

  $: {
    showFilePicker = filteredFiles.length > 0 && atQuery !== null;
    if (fileSelectedIdx >= filteredFiles.length) fileSelectedIdx = 0;
  }

  function scrollSelectedIntoView() {
    tick().then(() => {
      const el = suggestionEls[selectedIdx];
      if (el) el.scrollIntoView({ block: 'nearest' });
    });
  }

  function scrollFileSelectedIntoView() {
    tick().then(() => {
      const el = filePickerEls[fileSelectedIdx];
      if (el) el.scrollIntoView({ block: 'nearest' });
    });
  }

  async function handleSend() {
    if (!inputText.trim()) return;
    const text = inputText;
    inputText = '';
    showSuggestions = false;
    showFilePicker = false;
    if (textareaEl) textareaEl.style.height = 'auto';
    pendingMessages.add(text);
    await sendMessage(sessionId, text);
  }

  async function handleQuickAction(key: string) {
    const display = key === '\x03' ? 'Ctrl+C' : key;
    pendingMessages.add(display);
    await sendKeystroke(sessionId, key);
  }

  function selectCommand(cmd: string) {
    inputText = cmd + ' ';
    showSuggestions = false;
    textareaEl?.focus();
  }

  function selectFile(filePath: string) {
    if (!textareaEl) return;
    const pos = textareaEl.selectionStart;
    const textBefore = inputText.slice(0, pos);
    const lastAt = textBefore.lastIndexOf('@');
    if (lastAt === -1) return;
    const before = inputText.slice(0, lastAt);
    const after = inputText.slice(pos);
    inputText = before + '@' + filePath + ' ' + after;
    showFilePicker = false;
    tick().then(() => {
      if (textareaEl) {
        const newPos = lastAt + 1 + filePath.length + 1;
        textareaEl.selectionStart = newPos;
        textareaEl.selectionEnd = newPos;
        textareaEl.focus();
      }
    });
  }

  function handleKeydown(e: KeyboardEvent) {
    // @ file picker navigation
    if (showFilePicker) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        fileSelectedIdx = (fileSelectedIdx + 1) % filteredFiles.length;
        scrollFileSelectedIntoView();
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        fileSelectedIdx = (fileSelectedIdx - 1 + filteredFiles.length) % filteredFiles.length;
        scrollFileSelectedIntoView();
        return;
      }
      if (e.key === 'Tab') {
        e.preventDefault();
        selectFile(filteredFiles[fileSelectedIdx]);
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        showFilePicker = false;
        return;
      }
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        selectFile(filteredFiles[fileSelectedIdx]);
        return;
      }
    }

    // Slash command navigation (existing)
    if (showSuggestions) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        selectedIdx = (selectedIdx + 1) % filtered.length;
        scrollSelectedIntoView();
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        selectedIdx = (selectedIdx - 1 + filtered.length) % filtered.length;
        scrollSelectedIntoView();
        return;
      }
      if (e.key === 'Tab') {
        e.preventDefault();
        selectCommand(filtered[selectedIdx].cmd);
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        showSuggestions = false;
        return;
      }
      if (e.key === 'Enter' && !e.shiftKey) {
        const exactMatch = filtered.find(c => c.cmd === inputText.trim());
        if (exactMatch) {
          e.preventDefault();
          handleSend();
          return;
        }
        if (filtered.length > 0 && inputText.trim() !== filtered[selectedIdx].cmd) {
          e.preventDefault();
          selectCommand(filtered[selectedIdx].cmd);
          return;
        }
      }
    }
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }
</script>
```

- [ ] **Step 2: Replace the template section (lines 111-149)**

Replace the HTML template (everything between `</script>` and `<style>`) with:

```svelte
<div class="command-input">
  {#if showFilePicker}
    <div class="suggestions">
      {#each filteredFiles as filePath, i}
        <button
          bind:this={filePickerEls[i]}
          class="suggestion"
          class:selected={i === fileSelectedIdx}
          onclick={() => selectFile(filePath)}
          onmouseenter={() => fileSelectedIdx = i}
        >
          <span class="file-icon">📄</span>
          <span class="cmd">{filePath.split('/').pop()}</span>
          <span class="desc">{filePath}</span>
        </button>
      {/each}
    </div>
  {:else if showSuggestions}
    <div class="suggestions">
      {#each filtered as item, i}
        <button
          bind:this={suggestionEls[i]}
          class="suggestion"
          class:selected={i === selectedIdx}
          onclick={() => selectCommand(item.cmd)}
          onmouseenter={() => selectedIdx = i}
        >
          <span class="cmd">{item.cmd}</span>
          <span class="desc">{item.desc}</span>
          <span class="cat {item.category}">{item.category}</span>
        </button>
      {/each}
    </div>
  {/if}
  <div class="input-row">
    <div class="input-wrapper">
      <span class="prompt">$</span>
      <textarea
        bind:this={textareaEl}
        bind:value={inputText}
        onkeydown={handleKeydown}
        placeholder="Send command to {agentName}... (/ for commands, @ for files)"
        rows="1"
        oninput={(e) => { const t = e.currentTarget; t.style.height = 'auto'; t.style.height = Math.min(t.scrollHeight, 120) + 'px'; }}
      ></textarea>
    </div>
    <button class="send-btn" onclick={handleSend}>Send</button>
  </div>
  <div class="quick-actions">
    <button onclick={() => handleQuickAction('y')}>y</button>
    <button onclick={() => handleQuickAction('n')}>n</button>
    <button onclick={() => handleQuickAction('yes, and continue')}>yes, and continue</button>
    <button class="ctrl-c" onclick={() => handleQuickAction('\x03')}>Ctrl+C</button>
  </div>
</div>
```

- [ ] **Step 3: Add file icon style to the `<style>` block**

Add inside the `<style>` block, after the `.suggestion .cmd` rule (after line 248):

```css
  .suggestion .file-icon {
    font-size: 12px;
    flex-shrink: 0;
  }
```

- [ ] **Step 4: Pass `agentCwd` from CentralPanel**

In `src/components/CentralPanel.svelte`, update the `CommandInput` usage (line 170) from:

```svelte
  <CommandInput sessionId={agent.sessionId} agentName={agent.project} />
```

to:

```svelte
  <CommandInput sessionId={agent.sessionId} agentName={agent.project} agentCwd={agent.cwd} />
```

- [ ] **Step 5: Verify no TypeScript errors**

Run: `cd C:\Users\fernandonepen\Documents\agent-dashboard-v2 && npx svelte-check --threshold error 2>&1 | tail -3`
Expected: 0 ERRORS

- [ ] **Step 6: Commit**

```bash
git add src/components/CommandInput.svelte src/components/CentralPanel.svelte
git commit --author="Fernando <fernandoschnneider@gmail.com>" -m "feat: add @ file context picker with autocomplete in chat input"
```

---

### Task 4: Auto-Scroll on Message Send

**Files:**
- Modify: `src/components/CentralPanel.svelte`

- [ ] **Step 1: Add reactive scroll on pending messages**

In `src/components/CentralPanel.svelte`, add the following reactive block after the `prevEntryCount` variable declaration (after line 33):

```typescript
  // Auto-scroll when a new pending message is added
  let prevPendingCount = 0;
  $: {
    const count = $pendingMessages.length;
    if (count > prevPendingCount) {
      requestAnimationFrame(() => {
        if (logContainer) {
          logContainer.scrollTop = logContainer.scrollHeight;
          userScrolledUp = false;
          showScrollBtn = false;
        }
      });
    }
    prevPendingCount = count;
  }
```

- [ ] **Step 2: Verify no TypeScript errors**

Run: `cd C:\Users\fernandonepen\Documents\agent-dashboard-v2 && npx svelte-check --threshold error 2>&1 | tail -3`
Expected: 0 ERRORS

- [ ] **Step 3: Commit**

```bash
git add src/components/CentralPanel.svelte
git commit --author="Fernando <fernandoschnneider@gmail.com>" -m "fix: auto-scroll to bottom when sending a message to queue"
```

---

### Task 5: Fix Approval Banner Not Showing

**Files:**
- Modify: `src/components/CentralPanel.svelte`

- [ ] **Step 1: Remove status condition from approval banner**

In `src/components/CentralPanel.svelte`, change line 154 from:

```svelte
        {#if agent.status === 'input' && agent.pendingApproval}
```

to:

```svelte
        {#if agent.pendingApproval}
```

This makes the banner show whenever `pendingApproval` is set, regardless of the `status` field. The `derive_status_from_tail` function and `detect_pending_approval` function use different logic and may disagree — the presence of `pendingApproval` is the authoritative signal.

- [ ] **Step 2: Commit**

```bash
git add src/components/CentralPanel.svelte
git commit --author="Fernando <fernandoschnneider@gmail.com>" -m "fix: show approval banner whenever pendingApproval is set, regardless of status"
```

---

### Task 6: Sound Notification on Agent Permission Request

**Files:**
- Modify: `src/App.svelte`

- [ ] **Step 1: Add sound notification logic to `App.svelte`**

Replace the entire `<script>` block in `src/App.svelte` with:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { agents, selectedAgentId } from './lib/stores/agents';
  import { onAgentsUpdate } from './lib/tauri';
  import { journal } from './lib/stores/journal';
  import Sidebar from './components/Sidebar.svelte';
  import CentralPanel from './components/CentralPanel.svelte';
  import RightPanel from './components/RightPanel.svelte';

  let prevAgentStatuses: Record<string, string> = {};

  function playNotificationBeep() {
    try {
      const ctx = new AudioContext();
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.frequency.value = 800;
      osc.type = 'sine';
      gain.gain.value = 0.3;
      gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.2);
      osc.start(ctx.currentTime);
      osc.stop(ctx.currentTime + 0.2);
    } catch {
      // Audio not available
    }
  }

  onMount(() => {
    const unlisten = onAgentsUpdate((update) => {
      // Check for agents transitioning to 'input' status
      for (const agent of update) {
        const prev = prevAgentStatuses[agent.sessionId];
        if (agent.status === 'input' && prev && prev !== 'input') {
          playNotificationBeep();
          break; // One beep per update cycle is enough
        }
      }

      // Store current statuses for next comparison
      const newStatuses: Record<string, string> = {};
      for (const agent of update) {
        newStatuses[agent.sessionId] = agent.status;
      }
      prevAgentStatuses = newStatuses;

      agents.set(update);
      // Auto-select first agent if none selected
      if (!$selectedAgentId && update.length > 0) {
        selectedAgentId.set(update[0].sessionId);
      }
    });

    return () => { unlisten.then(fn => fn()); };
  });

  function handleSelect(id: string) {
    selectedAgentId.set(id);
  }

  $: currentAgent = $agents.find(a => a.sessionId === $selectedAgentId) ?? null;
</script>
```

- [ ] **Step 2: Verify no TypeScript errors**

Run: `cd C:\Users\fernandonepen\Documents\agent-dashboard-v2 && npx svelte-check --threshold error 2>&1 | tail -3`
Expected: 0 ERRORS

- [ ] **Step 3: Commit**

```bash
git add src/App.svelte
git commit --author="Fernando <fernandoschnneider@gmail.com>" -m "feat: play notification sound when agent requests permission"
```

---

## Self-Review

**Spec coverage:**

| Requirement | Task |
|------------|------|
| `@` triggers file autocomplete dropdown | Task 3 (CommandInput.svelte) |
| Files come from project directory via backend | Task 1 (Rust `list_project_files`) |
| Respects `.gitignore` | Task 1 (`ignore` crate with `git_ignore(true)`) |
| File list cached on mount | Task 3 (`onMount` calls `listProjectFiles`) |
| Arrow/Tab/Enter/Escape navigation | Task 3 (`handleKeydown` file picker branch) |
| Selected file inserted as `@path` in text | Task 3 (`selectFile` function) |
| Max 15 suggestions visible | Task 3 (`filteredFiles` slices to 15) |
| Frontend binding | Task 2 (`listProjectFiles` in tauri.ts) |
| Auto-scroll on message send | Task 4 (reactive `$pendingMessages` watcher) |
| Approval banner fix | Task 5 (removed `status === 'input'` condition) |
| Sound on permission request | Task 6 (Web Audio API beep in App.svelte) |
| Sound plays every time any agent transitions to `input` | Task 6 (compares prev vs current statuses) |

**Placeholder scan:** No TBD/TODO found. All code blocks are complete.

**Type consistency:** `listProjectFiles(cwd: string)` matches Rust `list_project_files(cwd: String)`. `agentCwd` prop in CommandInput matches `agent.cwd` passed from CentralPanel. `playNotificationBeep` uses standard Web Audio API types.
