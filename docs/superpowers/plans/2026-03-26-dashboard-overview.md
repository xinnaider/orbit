# Dashboard Overview Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a dashboard home screen showing aggregated agent metrics, cost estimates, and a cross-agent activity feed.

**Architecture:** The dashboard is a new Svelte component (`DashboardView`) rendered in the central panel when no agent is selected. Cost calculation lives in a frontend utility (`cost.ts`). The activity feed is powered by a new Tauri command (`get_activity_feed`) that aggregates toolCall entries from all agent journal states.

**Tech Stack:** Svelte 5 (with legacy `export let` props), Rust/Tauri 2, TypeScript

---

## File Structure

| File | Responsibility |
|------|---------------|
| **Create:** `src/lib/cost.ts` | Pricing table and cost calculation from tokens + model |
| **Create:** `src/components/DashboardView.svelte` | Dashboard layout: metrics cards + activity feed |
| **Modify:** `src-tauri/src/models.rs` | Add `ActivityEntry` struct |
| **Modify:** `src-tauri/src/commands.rs` | Add `get_activity_feed` command |
| **Modify:** `src-tauri/src/lib.rs` | Register `get_activity_feed` |
| **Modify:** `src/lib/types.ts` | Add `ActivityEntry` interface |
| **Modify:** `src/lib/tauri.ts` | Add `getActivityFeed()` binding |
| **Modify:** `src/App.svelte` | Render DashboardView when no agent selected; remove auto-select |
| **Modify:** `src/components/Sidebar.svelte` | Add Dashboard button in header |

---

### Task 1: Cost Calculation Utility

**Files:**
- Create: `src/lib/cost.ts`

- [ ] **Step 1: Create the cost utility**

```typescript
// src/lib/cost.ts

import type { TokenUsage } from './types';

interface ModelPricing {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
}

// Prices per 1M tokens (USD)
const PRICING: Record<string, ModelPricing> = {
  'claude-opus-4-6':            { input: 15,   output: 75,  cacheRead: 1.5,  cacheWrite: 18.75 },
  'claude-opus-4-20250514':     { input: 15,   output: 75,  cacheRead: 1.5,  cacheWrite: 18.75 },
  'claude-sonnet-4-6':          { input: 3,    output: 15,  cacheRead: 0.3,  cacheWrite: 3.75  },
  'claude-sonnet-4-5-20250514': { input: 3,    output: 15,  cacheRead: 0.3,  cacheWrite: 3.75  },
  'claude-sonnet-4-20250514':   { input: 3,    output: 15,  cacheRead: 0.3,  cacheWrite: 3.75  },
  'claude-haiku-4-5-20251001':  { input: 0.8,  output: 4,   cacheRead: 0.08, cacheWrite: 1     },
};

const DEFAULT_PRICING: ModelPricing = { input: 3, output: 15, cacheRead: 0.3, cacheWrite: 3.75 };

export function estimateCost(tokens: TokenUsage, model: string | null): number {
  const p = (model && PRICING[model]) || DEFAULT_PRICING;
  return (
    tokens.input * p.input +
    tokens.output * p.output +
    tokens.cacheRead * p.cacheRead +
    tokens.cacheWrite * p.cacheWrite
  ) / 1_000_000;
}

export function formatCost(cost: number): string {
  return cost < 0.01 ? '<$0.01' : `$${cost.toFixed(2)}`;
}

export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${Math.round(n / 1_000)}K`;
  return String(n);
}
```

- [ ] **Step 2: Verify no TypeScript errors**

Run: `cd C:\Users\fernandonepen\Documents\agent-dashboard-v2 && npx svelte-check --threshold error 2>&1 | tail -3`
Expected: 0 ERRORS

- [ ] **Step 3: Commit**

```bash
git add src/lib/cost.ts
git commit -m "feat: add cost estimation utility with model pricing table"
```

---

### Task 2: ActivityEntry Backend — Model and Command

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `ActivityEntry` struct to `models.rs`**

Add after the `SlashCommand` struct (around line 143):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    pub agent_name: String,
    pub agent_session_id: String,
    pub timestamp: String,
    pub action: String,
    pub status: String,
}
```

- [ ] **Step 2: Add `get_activity_feed` command to `commands.rs`**

Add at the end of the file:

```rust
#[tauri::command]
pub fn get_activity_feed(state: State<AppState>) -> Vec<ActivityEntry> {
    let journal_states = match state.journal_states.lock() {
        Ok(js) => js,
        Err(_) => return vec![],
    };

    let mut entries: Vec<ActivityEntry> = Vec::new();

    for (session_id, js) in journal_states.iter() {
        let agent_name = session_id.clone();

        for entry in &js.entries {
            if entry.entry_type != JournalEntryType::ToolCall {
                continue;
            }

            let tool = entry.tool.as_deref().unwrap_or("unknown");
            let target = entry.tool_input.as_ref()
                .and_then(|input| {
                    input.get("command")
                        .or_else(|| input.get("file_path"))
                        .or_else(|| input.get("pattern"))
                        .or_else(|| input.get("prompt"))
                        .and_then(|v| v.as_str())
                })
                .unwrap_or("");

            // Truncate target to 60 chars
            let target_short = if target.len() > 60 {
                format!("{}...", &target[..57])
            } else {
                target.to_string()
            };

            let action = if target_short.is_empty() {
                tool.to_string()
            } else {
                format!("{} {}", tool, target_short)
            };

            let status = match js.status {
                AgentStatus::Working => "working",
                AgentStatus::Input => "input",
                AgentStatus::Idle => "idle",
                AgentStatus::New => "new",
            };

            entries.push(ActivityEntry {
                agent_name: agent_name.clone(),
                agent_session_id: session_id.clone(),
                timestamp: entry.timestamp.clone(),
                action,
                status: status.to_string(),
            });
        }
    }

    // Sort by timestamp descending
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Return top 50
    entries.truncate(50);
    entries
}
```

- [ ] **Step 3: Register in `lib.rs`**

In `src-tauri/src/lib.rs`, add `commands::get_activity_feed` to the `invoke_handler` macro. The full block becomes:

```rust
        .invoke_handler(tauri::generate_handler![
            commands::send_keystroke,
            commands::send_message,
            commands::get_journal,
            commands::get_diff,
            commands::get_file_versions,
            commands::get_subagent_journal,
            commands::get_slash_commands,
            commands::get_activity_feed,
        ])
```

- [ ] **Step 4: Build to verify**

Run: `cd C:\Users\fernandonepen\Documents\agent-dashboard-v2 && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: `Finished` with no errors

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add get_activity_feed Tauri command"
```

---

### Task 3: Frontend Bindings — TypeScript Type and Tauri Wrapper

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`

- [ ] **Step 1: Add `ActivityEntry` interface to `types.ts`**

Add after the `SlashCommand` interface (around line 61):

```typescript
export interface ActivityEntry {
  agentName: string;
  agentSessionId: string;
  timestamp: string;
  action: string;
  status: string;
}
```

- [ ] **Step 2: Add `getActivityFeed` to `tauri.ts`**

Add the import for `ActivityEntry` to the existing import line:

```typescript
import type { AgentState, JournalEntry, SlashCommand, ActivityEntry } from './types';
```

Add the function after `getSlashCommands()`:

```typescript
export async function getActivityFeed(): Promise<ActivityEntry[]> {
  return await invoke('get_activity_feed');
}
```

- [ ] **Step 3: Verify no TypeScript errors**

Run: `cd C:\Users\fernandonepen\Documents\agent-dashboard-v2 && npx svelte-check --threshold error 2>&1 | tail -3`
Expected: 0 ERRORS

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/tauri.ts
git commit -m "feat: add ActivityEntry type and getActivityFeed binding"
```

---

### Task 4: DashboardView Component

**Files:**
- Create: `src/components/DashboardView.svelte`

This is the main component. It receives the `agents` array as a prop and renders two columns: metrics (left) and activity feed (right).

- [ ] **Step 1: Create `DashboardView.svelte`**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import type { AgentState, ActivityEntry } from '../lib/types';
  import { selectedAgentId } from '../lib/stores/agents';
  import { estimateCost, formatCost, formatTokens } from '../lib/cost';
  import { getActivityFeed } from '../lib/tauri';

  export let agents: AgentState[];

  let feed: ActivityEntry[] = [];
  let feedInterval: ReturnType<typeof setInterval>;

  $: workingCount = agents.filter(a => a.status === 'working').length;
  $: inputCount = agents.filter(a => a.status === 'input').length;
  $: idleCount = agents.filter(a => a.status === 'idle' || a.status === 'new').length;

  $: totalTokens = agents.reduce((s, a) => ({
    input: s.input + a.tokens.input,
    output: s.output + a.tokens.output,
    cacheRead: s.cacheRead + a.tokens.cacheRead,
    cacheWrite: s.cacheWrite + a.tokens.cacheWrite,
  }), { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 });

  $: totalCost = agents.reduce((sum, a) => sum + estimateCost(a.tokens, a.model), 0);

  $: agentsByCost = [...agents]
    .map(a => ({ ...a, cost: estimateCost(a.tokens, a.model) }))
    .sort((a, b) => b.cost - a.cost);

  function selectAgent(sessionId: string) {
    selectedAgentId.set(sessionId);
  }

  async function refreshFeed() {
    try {
      feed = await getActivityFeed();
    } catch { /* ignore */ }
  }

  function formatTime(ts: string): string {
    try {
      const d = new Date(ts);
      return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    } catch {
      return '';
    }
  }

  function statusColor(status: string): string {
    if (status === 'working') return 'var(--green)';
    if (status === 'input') return 'var(--amber)';
    return 'var(--text-dim)';
  }

  onMount(() => {
    refreshFeed();
    feedInterval = setInterval(refreshFeed, 3000);
    return () => clearInterval(feedInterval);
  });
</script>

{#if agents.length === 0}
  <div class="empty-state">
    <div class="empty-title">No agents running</div>
    <div class="empty-subtitle">Start a Claude Code session to see it here</div>
  </div>
{:else}
  <div class="dashboard">
    <div class="dashboard-header">
      <span class="dashboard-title">Dashboard</span>
      <span class="agent-count">{agents.length} agent{agents.length !== 1 ? 's' : ''}</span>
    </div>

    <div class="columns">
      <!-- Left: Metrics -->
      <div class="metrics">
        <!-- Agents card -->
        <div class="card">
          <div class="card-label">Agents</div>
          <div class="status-row">
            {#if workingCount > 0}
              <span class="status-item"><span class="dot working"></span>{workingCount} working</span>
            {/if}
            {#if inputCount > 0}
              <span class="status-item"><span class="dot input"></span>{inputCount} input</span>
            {/if}
            {#if idleCount > 0}
              <span class="status-item"><span class="dot idle"></span>{idleCount} idle</span>
            {/if}
          </div>
        </div>

        <!-- Cost card -->
        <div class="card">
          <div class="card-label">Estimated Cost</div>
          <div class="big-number">{formatCost(totalCost)}</div>
          <div class="breakdown">
            {#each agentsByCost as a}
              <button class="breakdown-row" onclick={() => selectAgent(a.sessionId)}>
                <span class="breakdown-name">{a.project}</span>
                <span class="breakdown-value">{formatCost(a.cost)}</span>
              </button>
            {/each}
          </div>
        </div>

        <!-- Tokens card -->
        <div class="card">
          <div class="card-label">Tokens</div>
          <div class="big-number">{formatTokens(totalTokens.input + totalTokens.output + totalTokens.cacheRead + totalTokens.cacheWrite)}</div>
          <div class="breakdown">
            <div class="breakdown-row">
              <span class="breakdown-name"><span class="color-dot" style="background:var(--blue)"></span>Input</span>
              <span class="breakdown-value">{formatTokens(totalTokens.input)}</span>
            </div>
            <div class="breakdown-row">
              <span class="breakdown-name"><span class="color-dot" style="background:var(--purple)"></span>Output</span>
              <span class="breakdown-value">{formatTokens(totalTokens.output)}</span>
            </div>
            <div class="breakdown-row">
              <span class="breakdown-name"><span class="color-dot" style="background:var(--green)"></span>Cache Read</span>
              <span class="breakdown-value">{formatTokens(totalTokens.cacheRead)}</span>
            </div>
            <div class="breakdown-row">
              <span class="breakdown-name"><span class="color-dot" style="background:var(--amber)"></span>Cache Write</span>
              <span class="breakdown-value">{formatTokens(totalTokens.cacheWrite)}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Right: Activity Feed -->
      <div class="feed-column">
        <div class="card feed-card">
          <div class="card-label">Activity Feed</div>
          <div class="feed">
            {#if feed.length === 0}
              <div class="feed-empty">No activity yet</div>
            {:else}
              {#each feed as entry}
                <div class="feed-entry">
                  <span class="feed-time">{formatTime(entry.timestamp)}</span>
                  <span class="feed-dot" style="color:{statusColor(entry.status)}">●</span>
                  <button class="feed-agent" onclick={() => selectAgent(entry.agentSessionId)}>{entry.agentName}</button>
                  <span class="feed-action">{entry.action}</span>
                </div>
              {/each}
            {/if}
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }
  .empty-title { font-size: 16px; color: var(--text-muted); font-weight: 600; }
  .empty-subtitle { font-size: 13px; color: var(--text-dim); }

  .dashboard {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
  .dashboard-header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .dashboard-title { font-weight: 600; font-size: 14px; }
  .agent-count {
    font-size: 11px;
    color: var(--green);
    background: var(--green-dim);
    padding: 1px 8px;
    border-radius: 8px;
  }

  .columns {
    flex: 1;
    display: flex;
    gap: 12px;
    padding: 12px 16px;
    overflow: hidden;
  }
  .metrics {
    width: 40%;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow-y: auto;
    flex-shrink: 0;
  }
  .feed-column {
    flex: 1;
    display: flex;
    min-width: 0;
  }

  .card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px 14px;
  }
  .card-label {
    font-size: 10px;
    text-transform: uppercase;
    color: var(--text-dim);
    letter-spacing: 0.5px;
    margin-bottom: 8px;
  }

  .status-row {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }
  .status-item {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 13px;
    color: var(--text-primary);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .dot.working { background: var(--green); }
  .dot.input { background: var(--amber); }
  .dot.idle { background: var(--text-dim); }

  .big-number {
    font-size: 22px;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 8px;
  }

  .breakdown {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .breakdown-row {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-secondary);
    background: none;
    border: none;
    padding: 3px 0;
    cursor: default;
    text-align: left;
    width: 100%;
  }
  button.breakdown-row {
    cursor: pointer;
    border-radius: 4px;
    padding: 3px 4px;
  }
  button.breakdown-row:hover {
    background: var(--bg-hover);
  }
  .breakdown-name {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .breakdown-value { color: var(--text-primary); font-weight: 500; }
  .color-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .feed-card {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .feed {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .feed-empty {
    color: var(--text-dim);
    font-size: 12px;
    padding: 20px 0;
    text-align: center;
  }
  .feed-entry {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: 12px;
    padding: 4px 0;
  }
  .feed-time {
    color: var(--text-dim);
    flex-shrink: 0;
    font-size: 11px;
    min-width: 42px;
  }
  .feed-dot {
    flex-shrink: 0;
    font-size: 8px;
    margin-top: 2px;
  }
  .feed-agent {
    color: var(--text-primary);
    font-weight: 600;
    flex-shrink: 0;
    background: none;
    border: none;
    padding: 0;
    font-size: 12px;
    cursor: pointer;
  }
  .feed-agent:hover { color: var(--blue); }
  .feed-action {
    color: var(--text-secondary);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
```

- [ ] **Step 2: Verify no errors**

Run: `cd C:\Users\fernandonepen\Documents\agent-dashboard-v2 && npx svelte-check --threshold error 2>&1 | tail -3`
Expected: 0 ERRORS

- [ ] **Step 3: Commit**

```bash
git add src/components/DashboardView.svelte
git commit -m "feat: add DashboardView component with metrics and activity feed"
```

---

### Task 5: Wire Dashboard Into App and Sidebar

**Files:**
- Modify: `src/App.svelte`
- Modify: `src/components/Sidebar.svelte`

- [ ] **Step 1: Update `App.svelte`**

Replace the entire `<script>` block and template with:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { agents, selectedAgentId } from './lib/stores/agents';
  import { onAgentsUpdate } from './lib/tauri';
  import { journal } from './lib/stores/journal';
  import Sidebar from './components/Sidebar.svelte';
  import CentralPanel from './components/CentralPanel.svelte';
  import RightPanel from './components/RightPanel.svelte';
  import DashboardView from './components/DashboardView.svelte';

  onMount(() => {
    const unlisten = onAgentsUpdate((update) => {
      agents.set(update);
    });

    return () => { unlisten.then(fn => fn()); };
  });

  function handleSelect(id: string) {
    selectedAgentId.set(id);
  }

  $: currentAgent = $agents.find(a => a.sessionId === $selectedAgentId) ?? null;
</script>

<div class="workspace">
  <Sidebar
    agents={$agents}
    selectedId={$selectedAgentId}
    onSelect={handleSelect}
  />

  <main class="central">
    {#if currentAgent}
      <CentralPanel agent={currentAgent} />
    {:else}
      <DashboardView agents={$agents} />
    {/if}
  </main>

  {#if currentAgent}
    <RightPanel agent={currentAgent} />
  {/if}
</div>
```

Key changes from current code:
- Import `DashboardView`
- Remove auto-select logic (`if (!$selectedAgentId && update.length > 0)` is deleted)
- Replace empty state `<div>` with `<DashboardView>`
- Remove empty right panel fallback (no right panel when on dashboard)

Keep the existing `<style>` block unchanged.

- [ ] **Step 2: Update `Sidebar.svelte`**

Add a Dashboard button in the header. Replace the header `<div>` with:

```svelte
  <div class="sidebar-header">
    <button class="title-btn" onclick={() => onSelect('')}>
      <span class="title">Dashboard</span>
    </button>
    <div class="header-right">
      <button class="theme-toggle" onclick={() => theme.toggle()} title="Toggle theme">
        {$theme === 'dark' ? '☀' : '☾'}
      </button>
      <span class="badge">{agents.length}</span>
    </div>
  </div>
```

The `onSelect('')` call needs to be handled. Update the `onSelect` prop type and the handler in `App.svelte`. Actually, it's simpler to add a separate prop. Instead, change the approach:

In `Sidebar.svelte`, add a new prop and button:

```svelte
  export let onDashboard: () => void;
```

Then update the header:

```svelte
  <div class="sidebar-header">
    <button class="title-btn" onclick={onDashboard}>
      <span class="title">Dashboard</span>
    </button>
    <div class="header-right">
      <button class="theme-toggle" onclick={() => theme.toggle()} title="Toggle theme">
        {$theme === 'dark' ? '☀' : '☾'}
      </button>
      <span class="badge">{agents.length}</span>
    </div>
  </div>
```

Add the CSS for `title-btn`:

```css
  .title-btn {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    display: flex;
    align-items: center;
  }
  .title-btn:hover .title { color: var(--blue); }
```

In `App.svelte`, pass the new prop to `Sidebar`:

```svelte
  <Sidebar
    agents={$agents}
    selectedId={$selectedAgentId}
    onSelect={handleSelect}
    onDashboard={() => selectedAgentId.set(null)}
  />
```

- [ ] **Step 3: Verify no errors**

Run: `cd C:\Users\fernandonepen\Documents\agent-dashboard-v2 && npx svelte-check --threshold error 2>&1 | tail -3`
Expected: 0 ERRORS

- [ ] **Step 4: Build full app**

Run: `cd C:\Users\fernandonepen\Documents\agent-dashboard-v2 && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: `Finished` with no errors

- [ ] **Step 5: Commit**

```bash
git add src/App.svelte src/components/Sidebar.svelte
git commit -m "feat: wire dashboard as home screen, add Dashboard button to sidebar"
```

---

### Task 6: Activity Feed — Use Agent Project Name Instead of Session ID

The `get_activity_feed` command currently uses the session ID as `agent_name`, but the dashboard should show the project name (e.g., "agent-dashboard") not the session UUID. The project name is available in the `AgentState` emitted by polling but not stored in `JournalState`.

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Update `get_activity_feed` to accept agent names from frontend**

The simplest approach: the frontend already has the agents store with project names. Instead of making the backend resolve names, pass a mapping from the frontend.

Actually, it's cleaner to look up agent names from the live polling data. Change the command to also read the last emitted agents list. But `AppState` only stores `journal_states`, not the agent list.

The cleanest fix: add a `project_name` field to `JournalState` so the feed command can use it.

In `src-tauri/src/journal_reader.rs`, add to the `JournalState` struct:

```rust
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
    pub project_name: Option<String>,  // <-- add this
}
```

Initialize it as `None` in `parse_journal` (in the `JournalState` construction). Then in `src-tauri/src/polling.rs`, after creating the journal, set the project name:

In `poll_once`, after line `let journal = journal_reader::parse_journal(...)` and after computing `project`, add:

```rust
journal.project_name = Some(project.clone());
```

Note: `journal` needs to be `mut`:

```rust
let mut journal = journal_reader::parse_journal(&jsonl_path, prev_file_size, prev);
```

Then in `commands.rs`, change `get_activity_feed` to use `js.project_name`:

```rust
let agent_name = js.project_name.as_deref().unwrap_or(session_id).to_string();
```

- [ ] **Step 2: Build to verify**

Run: `cd C:\Users\fernandonepen\Documents\agent-dashboard-v2 && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: `Finished` with no errors

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/journal_reader.rs src-tauri/src/polling.rs
git commit -m "fix: use project name instead of session ID in activity feed"
```

---

## Self-Review

**Spec coverage check:**

| Spec Requirement | Task |
|-----------------|------|
| Dashboard when no agent selected | Task 5 (App.svelte conditional render) |
| Dashboard button in sidebar | Task 5 (Sidebar.svelte) |
| No auto-select on startup | Task 5 (removed auto-select) |
| Agents card (count by status) | Task 4 (DashboardView) |
| Cost card (total + breakdown) | Task 1 (cost.ts) + Task 4 (DashboardView) |
| Tokens card (total + breakdown by type) | Task 4 (DashboardView) |
| Activity feed (tool calls only) | Task 2 (backend) + Task 4 (frontend) |
| Feed: 50 entries max, timestamp desc | Task 2 (backend truncate + sort) |
| Clickable agent names (feed + cost) | Task 4 (onclick → selectAgent) |
| Empty state message | Task 4 (DashboardView empty-state block) |
| Pricing table hardcoded | Task 1 (cost.ts PRICING table) |
| Cost calculation formula | Task 1 (estimateCost function) |
| ActivityEntry backend struct | Task 2 (models.rs) |
| get_activity_feed command | Task 2 (commands.rs) |
| Frontend bindings | Task 3 (types.ts + tauri.ts) |
| Agent name in feed (not session ID) | Task 6 |
| Dark/light theme support | Task 4 (uses CSS variables throughout) |
| No right panel on dashboard | Task 5 (removed fallback right panel) |

**Placeholder scan:** No TBD/TODO found. All code blocks are complete.

**Type consistency:** `ActivityEntry` fields match across models.rs, types.ts, and DashboardView usage. `estimateCost` signature matches usage in DashboardView. `formatCost`/`formatTokens` match usage.
