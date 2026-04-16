# Orbit — Workspace Tab System

> Design spec for draggable tabs, dynamic split panes, and persistent workspace layout.
> Inspired by Paseo's split-container and VS Code's editor groups.

---

## Goal

Replace the current fixed 4-pane grid with a flexible workspace where each pane holds multiple tabs. Tabs represent independent content (agent feed, terminal, file). Users can drag tabs between panes, create splits by dropping on edges, and resize panes freely. Layout persists across app restarts.

---

## Data Model

### Tab types

```typescript
type TabTarget =
  | { kind: 'agent'; sessionId: number }
  | { kind: 'terminal'; terminalId: string }
  | { kind: 'file'; path: string }  // future

type Tab = {
  id: string            // crypto.randomUUID()
  target: TabTarget
  createdAt: number
}
```

### Split tree

```typescript
type SplitNode =
  | { type: 'leaf'; paneId: string }
  | {
      type: 'split'
      direction: 'horizontal' | 'vertical'
      ratio: number               // 0.0–1.0, size of first child
      children: [SplitNode, SplitNode]
    }
```

- `horizontal` splits left/right, `vertical` splits top/bottom.
- `ratio` is the fraction allocated to the first child.
- The tree can nest arbitrarily: a horizontal split inside a vertical split, etc.

### Pane state

```typescript
type PaneState = {
  tabs: Tab[]
  activeTabId: string | null
  tabOrder: string[]    // ordered tab IDs for display
}
```

### Workspace store

```typescript
interface WorkspaceStore {
  root: SplitNode
  panes: Record<string, PaneState>
  focusedPaneId: string | null
}
```

Default: single leaf pane, no tabs.

---

## Component Hierarchy

```
WorkspaceContainer            receives WorkspaceStore, walks the tree
  ├─ SplitContainer           split node: two children + resize handle
  │   ├─ SplitContainer       recursive nesting
  │   └─ PaneContainer        leaf node: tab bar + content
  └─ PaneContainer
       ├─ TabBar               draggable tabs, + button, horizontal scroll
       │   └─ TabItem          icon + label + close button
       ├─ SplitDropZone        overlay with 5 drop zones (during drag only)
       └─ PaneContent          renders based on active tab's TabTarget
            ├─ CentralPanel    kind: 'agent' — feed + input, no terminal
            ├─ TerminalPanel   kind: 'terminal' — shell
            └─ (FilePanel)     kind: 'file' — future
```

### New components (6)

| Component | Responsibility |
|-----------|---------------|
| `WorkspaceContainer.svelte` | Walk `SplitNode` tree, render `SplitContainer` or `PaneContainer` for each node |
| `SplitContainer.svelte` | Render two children side by side (or stacked) with a drag handle between them |
| `PaneContainer.svelte` | Render `TabBar` + active tab's content. Show `SplitDropZone` during drag. |
| `TabBar.svelte` | Horizontal list of `TabItem` components. `+` button at the end. Scrolls if overflowing. |
| `TabItem.svelte` | Single tab: icon (based on kind), label, close button. Draggable via native drag API. |
| `SplitDropZone.svelte` | Overlay shown during drag. 5 zones: top, bottom, left, right (borders), center. Highlighted zone shows a preview of where the new pane would appear. |

### Modified components

| Component | Changes |
|-----------|---------|
| `CentralPanel.svelte` | Remove tab bar (Feed/Terminal toggle), remove PTY activation code. Only renders feed + input. |
| `Sidebar.svelte` | Click = `addTab` in focused pane. Double-click = new split. Drag sessions into workspace. |
| `App.svelte` | Replace `PaneGrid` with `WorkspaceContainer`. Update event listeners. |
| `MetaPanel.svelte` | Show info for active tab of focused pane. Minimal view for terminal tabs. |

### Removed components

| Component | Replaced by |
|-----------|------------|
| `PaneGrid.svelte` | `WorkspaceContainer` |
| `Pane.svelte` | `PaneContainer` |

---

## Drag & Drop

### Technology

Native browser Drag and Drop API (`dragstart`, `dragover`, `drop`). No external library needed. Svelte actions for drag source and drop target.

### Tab drag flow

1. User mousedown + hold on `TabItem` → `dragstart` fires
2. Drag data: `{ tabId, sourcePaneId }` stored in `dataTransfer`
3. As the tab is dragged over panes, `SplitDropZone` activates:
   - **Center zone** (inner 50%): drop here adds tab to this pane
   - **Top/Bottom/Left/Right zones** (outer 25% each): drop here creates a split
4. Active zone shows a **semi-transparent preview overlay** of where the new pane would appear
5. On `drop`:
   - **Center**: remove tab from source pane, add to target pane's tab list
   - **Border**: replace the target leaf node with a new split node containing the original pane + a new pane with the dropped tab
6. If source pane has no remaining tabs → collapse it (remove leaf, simplify parent split)

### Tab reorder (same pane)

- Dragging a tab within its own `TabBar` reorders via position swap
- Visual indicator: vertical insertion line between tabs
- No split preview shown for same-pane reorder

### Session drag from sidebar

- Sessions in the sidebar are also drag sources
- Dragging a session into the workspace uses the same drop zone logic
- Drop creates a new `{ kind: 'agent', sessionId }` tab

---

## Split Resize

### Drag handle

- 4px wide/tall handle between the two children of a `SplitContainer`
- Cursor: `col-resize` (horizontal split) or `row-resize` (vertical split)
- Dragging changes the parent `SplitNode`'s `ratio`
- Ratio clamped to `[0.15, 0.85]` to prevent invisible panes

### Reset

- Double-click the handle → reset ratio to `0.5`

### Implementation

- `mousedown` on handle starts tracking
- `mousemove` on `document` updates ratio (relative to container bounds)
- `mouseup` commits the new ratio to the store

---

## Store Actions

```typescript
// Tab management
addTab(paneId: string, target: TabTarget): void
closeTab(paneId: string, tabId: string): void
setActiveTab(paneId: string, tabId: string): void

// Drag & drop
moveTab(fromPaneId: string, toPaneId: string, tabId: string): void
reorderTab(paneId: string, fromIndex: number, toIndex: number): void

// Split management
splitPane(paneId: string, direction: 'horizontal' | 'vertical', tab: Tab): void
resizeSplit(parentPath: number[], ratio: number): void
collapsePane(paneId: string): void  // removes empty pane, simplifies tree

// Focus
focusPane(paneId: string): void
```

### `collapsePane` algorithm

When a pane has no tabs:
1. Find the parent split node
2. Replace the parent split with the other child (the sibling)
3. If the sibling is also a split, it stays as-is
4. If the sibling is a leaf, the tree becomes shallower
5. Repeat upward if needed (parent of parent also became unnecessary)

---

## Sidebar Interaction

| Action | Result |
|--------|--------|
| Click session | Open as tab in focused pane. If already open in any pane, focus that tab instead of creating duplicate. |
| Double-click session | Open in new split (horizontal) to the right of focused pane |
| Drag session to workspace | Drop in center = new tab. Drop on border = new split. |
| `+` button in tab bar | Dropdown: "New terminal", "New session", "Open session..." |
| Close last tab in pane | Pane is removed, parent split collapses |

### Tab bar `+` menu

```
+ ─┬─ New terminal     → spawns shell in project cwd
   ├─ New session       → opens NewSessionModal
   └─ Open session...   → shows list of existing sessions to pick from
```

---

## Tab Close Behavior

| Tab kind | On close |
|----------|----------|
| `agent` | Remove tab only. Session keeps running in background. Can reopen from sidebar. |
| `terminal` | Kill PTY process, remove tab. |
| `file` | Remove tab. (future) |

---

## MetaPanel Integration

The right-side MetaPanel shows info about the **active tab of the focused pane**:

| Active tab kind | MetaPanel shows |
|-----------------|-----------------|
| `agent` | Tokens, cost, context %, tools, subagents (same as today) |
| `terminal` | Shell name, cwd, PID |
| `file` | File path, size (future) |

---

## Persistence

### What is saved

- Full `SplitNode` tree structure (directions, ratios)
- Pane states with `agent` tabs (sessionId references)
- Focused pane ID
- Saved to `localStorage` key `orbit:workspace`

### What is NOT saved

- Terminal tabs (process died on app close — removed on restore)
- File tabs (can be reopened — removed on restore for MVP)

### Save strategy

- Debounce 500ms after any store mutation
- `JSON.stringify(workspaceStore)` → `localStorage`

### Restore strategy

1. Read from `localStorage`
2. Remove tabs where `kind === 'terminal'`
3. Validate `agent` tabs: remove if sessionId no longer exists in DB
4. If pane has no remaining tabs after cleanup → collapse it
5. If nothing left → fall back to default layout (1 empty pane)

---

## Migration from Current Layout

The current `layout.ts` store (`PaneId`, `SplitLayout`, grid-based) will be replaced entirely by the new `workspace.ts` store. No migration needed — old layout in localStorage is ignored, users start with default layout.

Components `PaneGrid.svelte` and `Pane.svelte` are deleted after the new system is wired in.

---

## Acceptance Criteria

1. User can open multiple tabs (agent, terminal) in a single pane
2. Tabs can be reordered by drag within the same tab bar
3. Tabs can be dragged to another pane's tab bar or center zone
4. Dropping a tab on a pane's edge creates a new split in that direction
5. Split resize via drag handle works with ratio clamped to [0.15, 0.85]
6. Double-click handle resets to 50/50
7. Closing the last tab in a pane removes the pane and collapses the parent split
8. Sidebar click opens/focuses agent tab. Double-click opens in new split.
9. `+` button offers "New terminal" and "New session"
10. Layout (splits + agent tabs) persists across app restart
11. Terminal tabs are not restored (process is gone)
12. MetaPanel adapts to the active tab type
