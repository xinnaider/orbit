# Orbit Quiet Journal Redesign

## Status

Approved for implementation planning.

## Goal

Simplify Orbit's frontend so the app feels calmer, more legible, and less visually polluted while preserving its power-user workspace model: multiple simultaneous chats, terminals, split panes, tool calls, git context, and agent telemetry.

The approved direction combines:

- **Sidebar/topbar from mockup A — Quiet Console**
- **Chat timeline from mockup B — Command Journal**
- **Multipanel behavior from the approved split-pane mockup**
- **Existing Orbit icon unchanged**

Reference mockups:

- `docs/mock-orbit-minimal-options.html`
- `docs/mock-orbit-merged-ab.html`
- `docs/mock-orbit-quiet-journal-panels.html`

## Design Principles

### 1. Chat first, telemetry second

The primary screen should be readable conversation. Tokens, context, recent tools, raw logs, tasks, subagents, and git details remain available, but they should not compete with the chat by default.

### 2. Dark minimal first

Dark mode is the primary design target. Light mode should later be made equivalent, but implementation should first refine the dark experience.

### 3. Comfortable by default, compact when needed

Single-pane chat should breathe. Narrow panes and long logs should automatically or explicitly use compact density.

### 4. Panels remain first-class

Orbit must still support multiple chats and terminal panes at once. The redesign must not reduce Orbit to a single-chat app.

### 5. Existing Orbit identity stays

The current SVG orbit mark remains the product icon. The redesign may refine color, spacing, and placement, but not replace the icon.

## Approved Layout

### Global shell

The app keeps a fixed left sidebar and a workspace area.

```
Sidebar
└── Workspace
    ├── single chat pane, or
    └── split pane tree
        ├── chat pane
        ├── terminal pane
        └── additional chat/tool panes
```

The current right `MetaPanel` should no longer be visible by default. It should become an inspector surface invoked on demand.

### Sidebar

Sidebar follows the Quiet Console mockup:

- Current Orbit icon, then `orbit` wordmark.
- Small rounded `new` button.
- Search / jump input.
- Session groups such as `Today`, `Pinned`, `Workspace`, and `Other sessions`.
- Session rows show only essential information:
  - display name
  - status dot
  - short provider/model
  - branch or pane assignment
  - compact context percentage only when useful
- Footer shows low-priority workspace hints, e.g. `drag sessions into panes • ⌘\ split • ⌘I inspect`.

The sidebar should feel quiet and stable. Avoid dense token rows, multiple badges, or bright accent blocks.

### Pane headers / topbar

When the workspace has one chat, the topbar can read like a global session header. When the workspace is split, each pane owns a compact header.

Pane header includes:

- small colored status dot
- session / terminal title
- short path or branch
- status pill
- provider/model pill when relevant
- context percentage pill when relevant
- overflow menu
- close button

Avoid duplicating a global topbar plus pane headers in split mode. Split mode should make pane headers the primary chrome.

### Chat feed

Chat uses the Quiet Journal timeline:

- Vertical event rail.
- Small nodes for user, agent, tool, and system events.
- Event metadata in subdued mono text.
- Agent text as readable prose, not boxed unless needed.
- User prompts can remain softly boxed in comfortable mode.
- Tool calls are inline timeline cards, not dominant panels.
- Tool cards show:
  - tool/action title
  - state (`done`, `working`, `queued`, `failed`)
  - concise output preview
  - expandable details later if needed

The feed should be optimized for scanning an agent's reasoning/progress without turning into raw logs.

### Composer

Composer remains at the bottom of each chat pane.

It should use a rounded quiet surface with:

- placeholder text
- `@ file`
- `/ command`
- model/effort controls where relevant
- compact toggle
- send button

In compact panes, composer height and button labels may shrink.

### Terminals

Terminals are first-class panes.

Terminal pane uses the same pane header system as chat panes:

- title: `Terminal`
- shell/path subtitle
- shell pill, e.g. `pwsh`
- new terminal action
- close action

Terminal content stays terminal-like: mono text, dark surface, prompt coloring. It should look integrated, but not fake-chat-like.

### Multipanel / split behavior

The split pane system remains central.

Rules:

- Multiple chat panes can be open simultaneously.
- Terminal panes can be split alongside chats.
- Splitters stay subtle, thin, and draggable.
- Focused pane gets a subtle outline or glow, not a loud border.
- Narrow chat panes enter compact timeline mode.
- Wide chat panes use comfortable spacing.

Compact mode changes:

- reduce timeline padding
- reduce event gaps
- reduce avatar/node size
- optionally remove user-message bubble chrome
- make tool cards smaller
- shorten pane header metadata

### Inspector / MetaPanel replacement

The current right MetaPanel should not be default visible chrome.

It should become an inspector opened by action or shortcut (`⌘I` / equivalent):

- Drawer or pane surface is acceptable in implementation planning.
- Inspector should contain current stats/tasks/agents functionality.
- Inspector should be session-aware for the focused pane.
- Inspector should be available without permanently reducing chat readability.

Default behavior for first iteration: hidden inspector, opened on demand.

## Component Impact

Likely affected components:

- `ui/components/Sidebar.svelte`
- `ui/components/CentralPanel.svelte`
- `ui/components/Feed.svelte`
- `ui/components/ToolCallEntry.svelte`
- `ui/components/InputBar.svelte`
- `ui/components/MetaPanel.svelte`
- `ui/components/TerminalPanel.svelte`
- `ui/components/workspace/PanelHeader.svelte`
- `ui/components/workspace/PaneContainer.svelte`
- `ui/components/workspace/SplitContainer.svelte`
- `ui/app.css`
- `ui/themes.css`

The implementation should preserve existing data flow and stores where possible. This is primarily a shell/feed/chrome redesign, not a backend or store rewrite.

## Data Flow

No major data-flow changes are required.

Existing stores remain sources of truth:

- `sessions` for session state
- `workspace` for split pane layout
- `journal` for chat entries
- `rawJournal` for raw output
- UI preference stores for sidebar/meta visibility and density

New or revised UI state may be needed for:

- inspector visibility
- compact density mode per pane or globally
- terminal pane type if not already represented clearly in workspace state

## Accessibility Requirements

- Preserve keyboard navigation for sidebar, pane headers, composer, and inspector.
- Pane header actions must have labels/titles.
- Status dots must not be the only status indicator; text status remains available.
- Timeline should remain semantic enough for screen readers: event type, actor, timestamp, content.
- Compact mode must not reduce font sizes below readable thresholds.

## Testing Requirements

Update/add component tests around:

- Sidebar simplified rendering and session rows.
- Feed timeline rendering for user, assistant, tool call, progress, and system entries.
- Tool call compact card rendering.
- Pane header rendering in chat and terminal contexts.
- Compact mode behavior.
- Inspector hidden-by-default behavior.
- Existing tests should be updated rather than discarded.

E2E smoke should continue to verify:

- app loads
- session can be created
- first user message appears
- chat feed renders populated content

## Non-goals

- Do not replace the Orbit icon.
- Do not remove split panes.
- Do not remove terminal support.
- Do not redesign backend/session orchestration.
- Do not make light mode the primary implementation target in this pass.
- Do not make raw logs the default chat view.

## Open Implementation Choice

The main remaining choice is how the inspector is implemented:

1. Drawer over the right side.
2. Temporary right pane attached to focused pane.
3. Reuse existing MetaPanel behind hidden-by-default visibility.

Recommendation for planning: start with option 3 for lower risk, then refine into drawer/pane behavior if needed.
