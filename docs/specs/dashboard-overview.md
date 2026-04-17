# Dashboard Overview — Design Spec

## Goal

Add a dedicated dashboard view in the central panel that shows an aggregated overview of all running Claude Code agents — metrics, estimated costs, and a real-time activity feed. The dashboard appears when no agent is selected, serving as the app's "home screen".

## Architecture

### Where It Lives

The dashboard replaces the central panel content when `selectedAgent` is `null`. No new routes or pages — it's a conditional render inside the existing layout.

- `App.svelte`: When `selectedAgentId` is null, render `DashboardView` instead of `CentralPanel`
- `Sidebar.svelte`: Add a "Dashboard" button/clickable header that sets `selectedAgentId` to `null`
- On app startup with no agents: show dashboard with empty state message

### Data Sources

All data comes from the existing `agents` store (populated by polling) and journal states already tracked in the Rust backend. No new polling mechanisms needed.

## Layout

Two-column layout inside the central panel area:

### Left Column (~40%) — Metric Cards

Three stacked cards:

#### 1. Agents Card
- Count per status with colored dots: green (working), amber (input), gray (idle)
- Total agent count
- Example: `● 2 working · ● 1 input · ● 1 idle`

#### 2. Cost Card
- Total estimated cost in USD (large number)
- Breakdown per agent, ordered by cost descending
- Each row: agent name + cost. Agent name is clickable (selects that agent)
- Example:
  ```
  $4.82
  api-server      $1.85
  agent-dashboard  $1.42
  ml-pipeline     $1.23
  docs-site       $0.32
  ```

#### 3. Tokens Card
- Total tokens (large number, formatted as K/M)
- Breakdown by type with distinct colors:
  - Input (blue)
  - Output (purple)
  - Cache Read (green)
  - Cache Write (amber)
- Example:
  ```
  1.2M
  Input       340K
  Output       85K
  Cache Read  680K
  Cache Write  95K
  ```

### Right Column (~60%) — Activity Feed

Real-time feed of important actions across all agents.

#### What Shows in the Feed
- **Tool calls**: Read, Edit, Write, Bash, Grep, Glob, Agent, Skill — with tool name and target
- **Status changes**: agent went idle, started working, awaiting input
- **NOT shown**: thinking blocks, assistant text, system messages

#### Feed Entry Format
Each line: `[timestamp] [status dot] [agent name] [action description]`

Example:
```
14:32  ● agent-dashboard  Edit src/components/Dashboard.svelte
14:31  ● api-server       ⏳ Awaiting permission for Bash
14:30  ● docs-site        Write docs/api-reference.md
14:28  ● agent-dashboard  Bash cargo build
14:25  ● ml-pipeline      Completed — idle
```

#### Feed Behavior
- Most recent entries at top
- Limited to 50 entries for performance
- Auto-updates as polling refreshes (no manual refresh needed)
- Agent name is clickable (selects that agent, opens chat)
- Scrollable if feed exceeds visible area

## Cost Estimation

### Pricing Table (Hardcoded)

Per 1M tokens:

| Model | Input | Output | Cache Read | Cache Write |
|-------|-------|--------|------------|-------------|
| claude-opus-4-6 | $15.00 | $75.00 | $1.50 | $18.75 |
| claude-sonnet-4-6 | $3.00 | $15.00 | $0.30 | $3.75 |
| claude-sonnet-4-5-20250514 | $3.00 | $15.00 | $0.30 | $3.75 |
| claude-haiku-4-5-20251001 | $0.80 | $4.00 | $0.08 | $1.00 |
| claude-opus-4-20250514 | $15.00 | $75.00 | $1.50 | $18.75 |
| claude-sonnet-4-20250514 | $3.00 | $15.00 | $0.30 | $3.75 |

### Calculation

```
cost = (tokens.input * price_input
      + tokens.output * price_output
      + tokens.cache_read * price_cache_read
      + tokens.cache_write * price_cache_write) / 1_000_000
```

### Implementation

Cost can be calculated on the frontend from `AgentState.tokens` and `AgentState.model`. No backend change needed for cost — just a utility function in TypeScript.

## Activity Feed Backend

### New Tauri Command: `get_activity_feed`

Returns the 50 most recent important entries across all agents.

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    pub agent_name: String,
    pub agent_session_id: String,
    pub timestamp: String,
    pub action: String,       // "Edit src/foo.rs", "Bash cargo build", "Idle"
    pub status: String,       // "working", "input", "idle"
}
```

Logic:
1. Iterate all journal states from `AppState`
2. For each agent, extract `toolCall` entries → map to `ActivityEntry`
3. Merge all entries, sort by timestamp descending
4. Return top 50

## Navigation

### Entering the Dashboard
- Click "Dashboard" button in sidebar header → `selectedAgentId.set(null)`
- App startup with no agents → dashboard shown by default (empty state)
- App startup with agents → dashboard shown (changes current behavior which auto-selects first agent; dashboard is now the home screen)

### Leaving the Dashboard
- Click agent in sidebar → selects agent, shows chat (existing behavior)
- Click agent name in activity feed → selects that agent
- Click agent name in cost breakdown → selects that agent

## Empty State

When no agents are running, the dashboard shows a centered message:
- "No agents running"
- Subtitle: "Start a Claude Code session to see it here"

## Styling

Follows existing theme system with CSS variables. Dark/light mode supported via existing `[data-theme]` approach. Cards use the same `--bg-secondary`, `--border`, `--text-*` variables as the rest of the app.

## Files to Create/Modify

### Create
- `src/components/DashboardView.svelte` — Main dashboard component
- `src/lib/cost.ts` — Cost calculation utility (pricing table + calculate function)

### Modify
- `src/App.svelte` — Conditional render: DashboardView when no agent selected
- `src/components/Sidebar.svelte` — Add Dashboard button to header
- `src-tauri/src/commands.rs` — Add `get_activity_feed` command
- `src-tauri/src/models.rs` — Add `ActivityEntry` struct
- `src-tauri/src/lib.rs` — Register new command
- `src/lib/tauri.ts` — Add `getActivityFeed()` binding
- `src/lib/types.ts` — Add `ActivityEntry` interface
