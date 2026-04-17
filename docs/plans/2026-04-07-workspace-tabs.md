# Workspace Tab System — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the fixed 4-pane grid with a flexible workspace where each pane holds draggable tabs (agent, terminal), supports dynamic splits via drop zones, and persists layout across restarts.

**Architecture:** Recursive `SplitNode` tree rendered by `WorkspaceContainer`. Each leaf is a `PaneContainer` with a `TabBar`. Native browser drag API for tab movement and split creation. Store in `workspace.ts` replaces `layout.ts`, persisted to `localStorage`.

**Tech Stack:** Svelte 5 (existing), native Drag and Drop API, `localStorage` for persistence, `crypto.randomUUID()` for tab IDs.

---

## File Map

### New files
| File | Responsibility |
|------|---------------|
| `ui/lib/stores/workspace.ts` | Workspace store: types, state, all actions (addTab, closeTab, moveTab, splitPane, etc.) |
| `ui/components/workspace/WorkspaceContainer.svelte` | Walks `SplitNode` tree, renders `SplitContainer` or `PaneContainer` |
| `ui/components/workspace/SplitContainer.svelte` | Two children + resize drag handle |
| `ui/components/workspace/PaneContainer.svelte` | TabBar + active tab content + SplitDropZone |
| `ui/components/workspace/TabBar.svelte` | Horizontal tab list, drag reorder, `+` button with menu |
| `ui/components/workspace/TabItem.svelte` | Single draggable tab: icon, label, close |
| `ui/components/workspace/SplitDropZone.svelte` | Overlay with 5 drop zones shown during drag |
| `ui/components/workspace/TabAddMenu.svelte` | Dropdown from `+` button: new terminal, new session, open session |

### Modified files
| File | Changes |
|------|---------|
| `ui/App.svelte` | Replace `PaneGrid` with `WorkspaceContainer`, update session event handlers to use workspace store |
| `ui/components/CentralPanel.svelte` | Remove tab bar, terminal activation, PTY code. Only feed + input. |
| `ui/components/Sidebar.svelte` | Click/double-click/drag use workspace store instead of layout store |
| `ui/components/MetaPanel.svelte` | Accept `TabTarget` to show info for terminal tabs |

### Removed files
| File | Reason |
|------|--------|
| `ui/lib/stores/layout.ts` | Replaced by `workspace.ts` |
| `ui/components/PaneGrid.svelte` | Replaced by `WorkspaceContainer` |
| `ui/components/Pane.svelte` | Replaced by `PaneContainer` |

---

## Task 1: Create workspace store with types and core actions

**Files:**
- Create: `ui/lib/stores/workspace.ts`

- [ ] **Step 1: Create the store with types and default state**

Create `ui/lib/stores/workspace.ts`:
```typescript
import { writable, get } from 'svelte/store';

// ── Types ──────────────────────────────────────────────────────────────

export type TabTarget =
  | { kind: 'agent'; sessionId: number }
  | { kind: 'terminal'; terminalId: string };

export interface Tab {
  id: string;
  target: TabTarget;
  createdAt: number;
}

export type SplitNode =
  | { type: 'leaf'; paneId: string }
  | {
      type: 'split';
      direction: 'horizontal' | 'vertical';
      ratio: number;
      children: [SplitNode, SplitNode];
    };

export interface PaneState {
  tabs: Tab[];
  activeTabId: string | null;
}

export interface WorkspaceState {
  root: SplitNode;
  panes: Record<string, PaneState>;
  focusedPaneId: string | null;
}

// ── Helpers ────────────────────────────────────────────────────────────

function newPaneId(): string {
  return crypto.randomUUID().slice(0, 8);
}

function newTabId(): string {
  return crypto.randomUUID();
}

function createEmptyPane(): PaneState {
  return { tabs: [], activeTabId: null };
}

function defaultState(): WorkspaceState {
  const paneId = newPaneId();
  return {
    root: { type: 'leaf', paneId },
    panes: { [paneId]: createEmptyPane() },
    focusedPaneId: paneId,
  };
}

// ── Store ──────────────────────────────────────────────────────────────

export const workspace = writable<WorkspaceState>(defaultState());
```

- [ ] **Step 2: Add tab management actions**

Append to `ui/lib/stores/workspace.ts`:
```typescript
export function addTab(paneId: string, target: TabTarget): void {
  workspace.update((ws) => {
    const pane = ws.panes[paneId];
    if (!pane) return ws;

    // Check if this target already exists in any pane
    for (const [pid, p] of Object.entries(ws.panes)) {
      const existing = p.tabs.find((t) => tabTargetsEqual(t.target, target));
      if (existing) {
        // Focus existing tab instead of creating duplicate
        ws.panes[pid] = { ...p, activeTabId: existing.id };
        ws.focusedPaneId = pid;
        return ws;
      }
    }

    const tab: Tab = { id: newTabId(), target, createdAt: Date.now() };
    ws.panes[paneId] = {
      tabs: [...pane.tabs, tab],
      activeTabId: tab.id,
    };
    ws.focusedPaneId = paneId;
    return ws;
  });
}

export function closeTab(paneId: string, tabId: string): void {
  workspace.update((ws) => {
    const pane = ws.panes[paneId];
    if (!pane) return ws;

    const tabs = pane.tabs.filter((t) => t.id !== tabId);
    const activeTabId =
      pane.activeTabId === tabId
        ? (tabs[tabs.length - 1]?.id ?? null)
        : pane.activeTabId;

    ws.panes[paneId] = { tabs, activeTabId };

    // Collapse pane if empty
    if (tabs.length === 0) {
      return collapseEmptyPane(ws, paneId);
    }
    return ws;
  });
}

export function setActiveTab(paneId: string, tabId: string): void {
  workspace.update((ws) => {
    const pane = ws.panes[paneId];
    if (!pane) return ws;
    ws.panes[paneId] = { ...pane, activeTabId: tabId };
    ws.focusedPaneId = paneId;
    return ws;
  });
}

export function focusPane(paneId: string): void {
  workspace.update((ws) => ({ ...ws, focusedPaneId: paneId }));
}

function tabTargetsEqual(a: TabTarget, b: TabTarget): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === 'agent' && b.kind === 'agent') return a.sessionId === b.sessionId;
  if (a.kind === 'terminal' && b.kind === 'terminal') return a.terminalId === b.terminalId;
  return false;
}
```

- [ ] **Step 3: Add drag/drop and split actions**

Append to `ui/lib/stores/workspace.ts`:
```typescript
export function moveTab(fromPaneId: string, toPaneId: string, tabId: string): void {
  workspace.update((ws) => {
    const from = ws.panes[fromPaneId];
    const to = ws.panes[toPaneId];
    if (!from || !to) return ws;

    const tab = from.tabs.find((t) => t.id === tabId);
    if (!tab) return ws;

    // Remove from source
    const fromTabs = from.tabs.filter((t) => t.id !== tabId);
    const fromActive =
      from.activeTabId === tabId
        ? (fromTabs[fromTabs.length - 1]?.id ?? null)
        : from.activeTabId;
    ws.panes[fromPaneId] = { tabs: fromTabs, activeTabId: fromActive };

    // Add to target
    ws.panes[toPaneId] = { tabs: [...to.tabs, tab], activeTabId: tab.id };
    ws.focusedPaneId = toPaneId;

    // Collapse source if empty
    if (fromTabs.length === 0) {
      return collapseEmptyPane(ws, fromPaneId);
    }
    return ws;
  });
}

export function reorderTab(paneId: string, fromIndex: number, toIndex: number): void {
  workspace.update((ws) => {
    const pane = ws.panes[paneId];
    if (!pane) return ws;
    const tabs = [...pane.tabs];
    const [moved] = tabs.splice(fromIndex, 1);
    tabs.splice(toIndex, 0, moved);
    ws.panes[paneId] = { ...pane, tabs };
    return ws;
  });
}

export function splitPane(
  paneId: string,
  direction: 'horizontal' | 'vertical',
  target: TabTarget,
): void {
  workspace.update((ws) => {
    const newPId = newPaneId();
    const tab: Tab = { id: newTabId(), target, createdAt: Date.now() };
    ws.panes[newPId] = { tabs: [tab], activeTabId: tab.id };

    // Replace the leaf node with a split containing old pane + new pane
    ws.root = replaceLeaf(ws.root, paneId, {
      type: 'split',
      direction,
      ratio: 0.5,
      children: [
        { type: 'leaf', paneId },
        { type: 'leaf', paneId: newPId },
      ],
    });

    ws.focusedPaneId = newPId;
    return ws;
  });
}

export function resizeSplit(path: number[], ratio: number): void {
  const clamped = Math.min(0.85, Math.max(0.15, ratio));
  workspace.update((ws) => {
    ws.root = setRatioAtPath(ws.root, path, clamped);
    return ws;
  });
}
```

- [ ] **Step 4: Add tree manipulation helpers**

Append to `ui/lib/stores/workspace.ts`:
```typescript
// ── Tree helpers ───────────────────────────────────────────────────────

function replaceLeaf(node: SplitNode, paneId: string, replacement: SplitNode): SplitNode {
  if (node.type === 'leaf') {
    return node.paneId === paneId ? replacement : node;
  }
  return {
    ...node,
    children: [
      replaceLeaf(node.children[0], paneId, replacement),
      replaceLeaf(node.children[1], paneId, replacement),
    ],
  };
}

function setRatioAtPath(node: SplitNode, path: number[], ratio: number): SplitNode {
  if (path.length === 0 && node.type === 'split') {
    return { ...node, ratio };
  }
  if (node.type === 'split' && path.length > 0) {
    const [head, ...rest] = path;
    const children: [SplitNode, SplitNode] = [...node.children];
    children[head] = setRatioAtPath(children[head], rest, ratio);
    return { ...node, children };
  }
  return node;
}

function collapseEmptyPane(ws: WorkspaceState, paneId: string): WorkspaceState {
  delete ws.panes[paneId];

  // If this is the root leaf, recreate default
  if (ws.root.type === 'leaf' && ws.root.paneId === paneId) {
    const newPId = newPaneId();
    ws.root = { type: 'leaf', paneId: newPId };
    ws.panes[newPId] = createEmptyPane();
    ws.focusedPaneId = newPId;
    return ws;
  }

  // Remove the leaf and promote sibling
  ws.root = removeLeaf(ws.root, paneId);

  // Update focus if needed
  if (ws.focusedPaneId === paneId) {
    const allPaneIds = Object.keys(ws.panes);
    ws.focusedPaneId = allPaneIds[0] ?? null;
  }
  return ws;
}

function removeLeaf(node: SplitNode, paneId: string): SplitNode {
  if (node.type === 'leaf') return node;
  const [left, right] = node.children;

  if (left.type === 'leaf' && left.paneId === paneId) return right;
  if (right.type === 'leaf' && right.paneId === paneId) return left;

  return {
    ...node,
    children: [removeLeaf(left, paneId), removeLeaf(right, paneId)],
  };
}
```

- [ ] **Step 5: Add persistence**

Append to `ui/lib/stores/workspace.ts`:
```typescript
// ── Persistence ────────────────────────────────────────────────────────

const STORAGE_KEY = 'orbit:workspace';
let saveTimer: ReturnType<typeof setTimeout>;

export function saveWorkspace(): void {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    const state = get(workspace);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  }, 500);
}

export function restoreWorkspace(validSessionIds: Set<number>): void {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return;
    const state: WorkspaceState = JSON.parse(raw);

    // Remove terminal tabs and validate agent tabs
    for (const [paneId, pane] of Object.entries(state.panes)) {
      pane.tabs = pane.tabs.filter((t) => {
        if (t.target.kind === 'terminal') return false;
        if (t.target.kind === 'agent') return validSessionIds.has(t.target.sessionId);
        return true;
      });
      if (pane.tabs.length === 0) {
        delete state.panes[paneId];
      } else if (pane.activeTabId && !pane.tabs.some((t) => t.id === pane.activeTabId)) {
        pane.activeTabId = pane.tabs[0]?.id ?? null;
      }
    }

    // Prune tree of empty panes
    state.root = pruneTree(state.root, state.panes);

    // Fallback if nothing survived
    if (Object.keys(state.panes).length === 0) return; // will use default

    workspace.set(state);
  } catch {
    // Corrupted data — use default
  }
}

function pruneTree(node: SplitNode, panes: Record<string, PaneState>): SplitNode {
  if (node.type === 'leaf') return node;
  const left = pruneTree(node.children[0], panes);
  const right = pruneTree(node.children[1], panes);

  const leftDead = left.type === 'leaf' && !panes[left.paneId];
  const rightDead = right.type === 'leaf' && !panes[right.paneId];

  if (leftDead && rightDead) return left; // both dead, return arbitrary
  if (leftDead) return right;
  if (rightDead) return left;
  return { ...node, children: [left, right] };
}

// Auto-save on every change
workspace.subscribe(() => saveWorkspace());
```

- [ ] **Step 6: Verify lint passes**

Run: `npx svelte-check --threshold error`
Expected: 0 errors

- [ ] **Step 7: Commit**

```
feat: add workspace store with types, actions, and persistence
```

---

## Task 2: Create WorkspaceContainer and SplitContainer

**Files:**
- Create: `ui/components/workspace/WorkspaceContainer.svelte`
- Create: `ui/components/workspace/SplitContainer.svelte`

- [ ] **Step 1: Create WorkspaceContainer**

Create `ui/components/workspace/WorkspaceContainer.svelte`:
```svelte
<script lang="ts">
  import { workspace } from '../../lib/stores/workspace';
  import type { SplitNode } from '../../lib/stores/workspace';
  import SplitContainer from './SplitContainer.svelte';
  import PaneContainer from './PaneContainer.svelte';

  function renderNode(node: SplitNode): { component: any; props: any } {
    if (node.type === 'leaf') {
      return { component: PaneContainer, props: { paneId: node.paneId } };
    }
    return { component: SplitContainer, props: { node } };
  }

  $: node = $workspace.root;
</script>

<div class="workspace">
  {#if node.type === 'leaf'}
    <PaneContainer paneId={node.paneId} />
  {:else}
    <SplitContainer {node} path={[]} />
  {/if}
</div>

<style>
  .workspace {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
</style>
```

- [ ] **Step 2: Create SplitContainer with resize handle**

Create `ui/components/workspace/SplitContainer.svelte`:
```svelte
<script lang="ts">
  import type { SplitNode } from '../../lib/stores/workspace';
  import { resizeSplit } from '../../lib/stores/workspace';
  import PaneContainer from './PaneContainer.svelte';

  export let node: SplitNode & { type: 'split' };
  export let path: number[];

  let container: HTMLDivElement;
  let dragging = false;

  function onHandleMousedown(e: MouseEvent) {
    e.preventDefault();
    dragging = true;

    const onMove = (ev: MouseEvent) => {
      if (!container) return;
      const rect = container.getBoundingClientRect();
      let ratio: number;
      if (node.direction === 'horizontal') {
        ratio = (ev.clientX - rect.left) / rect.width;
      } else {
        ratio = (ev.clientY - rect.top) / rect.height;
      }
      resizeSplit(path, ratio);
    };

    const onUp = () => {
      dragging = false;
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
    };

    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  }

  function onHandleDblclick() {
    resizeSplit(path, 0.5);
  }

  $: isHorizontal = node.direction === 'horizontal';
  $: firstSize = `${node.ratio * 100}%`;
  $: secondSize = `${(1 - node.ratio) * 100}%`;
</script>

<div
  class="split-container"
  class:horizontal={isHorizontal}
  class:vertical={!isHorizontal}
  bind:this={container}
>
  <div class="split-child" style={isHorizontal ? `width:${firstSize}` : `height:${firstSize}`}>
    {#if node.children[0].type === 'leaf'}
      <PaneContainer paneId={node.children[0].paneId} />
    {:else}
      <svelte:self node={node.children[0]} path={[...path, 0]} />
    {/if}
  </div>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="resize-handle"
    class:dragging
    class:handle-h={isHorizontal}
    class:handle-v={!isHorizontal}
    on:mousedown={onHandleMousedown}
    on:dblclick={onHandleDblclick}
  ></div>

  <div class="split-child" style={isHorizontal ? `width:${secondSize}` : `height:${secondSize}`}>
    {#if node.children[1].type === 'leaf'}
      <PaneContainer paneId={node.children[1].paneId} />
    {:else}
      <svelte:self node={node.children[1]} path={[...path, 1]} />
    {/if}
  </div>
</div>

<style>
  .split-container {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  .horizontal { flex-direction: row; }
  .vertical { flex-direction: column; }

  .split-child {
    display: flex;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .resize-handle {
    flex-shrink: 0;
    background: var(--bd);
    z-index: 2;
    transition: background 0.15s;
  }
  .resize-handle:hover, .resize-handle.dragging {
    background: var(--ac);
  }
  .handle-h {
    width: 4px;
    cursor: col-resize;
  }
  .handle-v {
    height: 4px;
    cursor: row-resize;
  }
</style>
```

- [ ] **Step 3: Verify compilation**

Run: `npx svelte-check --threshold error`
Expected: 0 errors (PaneContainer not yet created — may need a stub)

- [ ] **Step 4: Commit**

```
feat: add WorkspaceContainer and SplitContainer with resize handles
```

---

## Task 3: Create TabItem, TabBar, and TabAddMenu

**Files:**
- Create: `ui/components/workspace/TabItem.svelte`
- Create: `ui/components/workspace/TabBar.svelte`
- Create: `ui/components/workspace/TabAddMenu.svelte`

- [ ] **Step 1: Create TabItem**

Create `ui/components/workspace/TabItem.svelte`:
```svelte
<script lang="ts">
  import type { Tab } from '../../lib/stores/workspace';
  import { createEventDispatcher } from 'svelte';

  export let tab: Tab;
  export let active: boolean;
  export let paneId: string;

  const dispatch = createEventDispatcher<{ close: { tabId: string } }>();

  $: label = (() => {
    switch (tab.target.kind) {
      case 'agent': return `Session #${tab.target.sessionId}`;
      case 'terminal': return 'Terminal';
      default: return 'Tab';
    }
  })();

  $: icon = (() => {
    switch (tab.target.kind) {
      case 'agent': return '●';
      case 'terminal': return '>';
      default: return '○';
    }
  })();

  function onDragStart(e: DragEvent) {
    if (!e.dataTransfer) return;
    e.dataTransfer.setData('text/plain', JSON.stringify({ tabId: tab.id, sourcePaneId: paneId }));
    e.dataTransfer.effectAllowed = 'move';
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="tab-item"
  class:active
  draggable="true"
  on:dragstart={onDragStart}
>
  <span class="tab-icon">{icon}</span>
  <span class="tab-label">{label}</span>
  <button class="tab-close" on:click|stopPropagation={() => dispatch('close', { tabId: tab.id })}>
    ×
  </button>
</div>

<style>
  .tab-item {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: var(--sp-2) var(--sp-4);
    font-size: var(--xs);
    color: var(--t2);
    cursor: pointer;
    border-right: 1px solid var(--bd);
    white-space: nowrap;
    user-select: none;
    transition: color 0.1s, background 0.1s;
    flex-shrink: 0;
  }
  .tab-item:hover { color: var(--t0); background: var(--bg2); }
  .tab-item.active {
    color: var(--t0);
    background: var(--bg);
    border-bottom: 2px solid var(--ac);
  }
  .tab-icon { font-size: 8px; }
  .tab-label { max-width: 120px; overflow: hidden; text-overflow: ellipsis; }
  .tab-close {
    background: none; border: none; color: var(--t3); font-size: 11px;
    cursor: pointer; padding: 0 2px; line-height: 1; border-radius: 2px;
  }
  .tab-close:hover { color: var(--s-error); background: rgba(255,0,0,0.1); }
</style>
```

- [ ] **Step 2: Create TabAddMenu**

Create `ui/components/workspace/TabAddMenu.svelte`:
```svelte
<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';

  export let x: number;
  export let y: number;

  const dispatch = createEventDispatcher<{
    select: { action: 'terminal' | 'session' | 'open' };
    close: void;
  }>();

  function handleClick(action: 'terminal' | 'session' | 'open') {
    dispatch('select', { action });
  }

  onMount(() => {
    const onClick = () => dispatch('close');
    setTimeout(() => document.addEventListener('click', onClick), 0);
    return () => document.removeEventListener('click', onClick);
  });
</script>

<div class="menu" style="left:{x}px;top:{y}px">
  <button class="menu-item" on:click={() => handleClick('terminal')}>New terminal</button>
  <button class="menu-item" on:click={() => handleClick('session')}>New session</button>
  <button class="menu-item" on:click={() => handleClick('open')}>Open session...</button>
</div>

<style>
  .menu {
    position: fixed;
    z-index: 100;
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: var(--sp-2) 0;
    min-width: 140px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
  }
  .menu-item {
    display: block; width: 100%; text-align: left;
    background: none; border: none; color: var(--t1);
    font-size: var(--xs); padding: var(--sp-3) var(--sp-5);
    cursor: pointer; font-family: var(--mono);
  }
  .menu-item:hover { background: var(--bg3); color: var(--t0); }
</style>
```

- [ ] **Step 3: Create TabBar**

Create `ui/components/workspace/TabBar.svelte`:
```svelte
<script lang="ts">
  import type { Tab } from '../../lib/stores/workspace';
  import { setActiveTab, closeTab, reorderTab } from '../../lib/stores/workspace';
  import TabItem from './TabItem.svelte';
  import TabAddMenu from './TabAddMenu.svelte';
  import { createEventDispatcher } from 'svelte';

  export let paneId: string;
  export let tabs: Tab[];
  export let activeTabId: string | null;

  const dispatch = createEventDispatcher<{
    addaction: { action: 'terminal' | 'session' | 'open' };
  }>();

  let menuOpen: { x: number; y: number } | null = null;

  function onTabClick(tabId: string) {
    setActiveTab(paneId, tabId);
  }

  function onTabClose(e: CustomEvent<{ tabId: string }>) {
    closeTab(paneId, e.detail.tabId);
  }

  function onAddClick(e: MouseEvent) {
    const rect = (e.target as HTMLElement).getBoundingClientRect();
    menuOpen = { x: rect.left, y: rect.bottom + 4 };
  }

  function onMenuSelect(e: CustomEvent<{ action: 'terminal' | 'session' | 'open' }>) {
    menuOpen = null;
    dispatch('addaction', e.detail);
  }

  // Drag reorder within same tab bar
  let dragOverIndex = -1;

  function onDragOver(e: DragEvent, index: number) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    dragOverIndex = index;
  }

  function onDragLeave() {
    dragOverIndex = -1;
  }

  function onDrop(e: DragEvent, toIndex: number) {
    e.preventDefault();
    dragOverIndex = -1;
    if (!e.dataTransfer) return;
    try {
      const data = JSON.parse(e.dataTransfer.getData('text/plain'));
      if (data.sourcePaneId === paneId) {
        const fromIndex = tabs.findIndex((t) => t.id === data.tabId);
        if (fromIndex >= 0 && fromIndex !== toIndex) {
          reorderTab(paneId, fromIndex, toIndex);
        }
      }
    } catch { /* not a tab drag */ }
  }
</script>

<div class="tab-bar">
  <div class="tab-list">
    {#each tabs as tab, i (tab.id)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="tab-drop-zone"
        class:drag-over={dragOverIndex === i}
        on:click={() => onTabClick(tab.id)}
        on:keydown={(e) => e.key === 'Enter' && onTabClick(tab.id)}
        on:dragover={(e) => onDragOver(e, i)}
        on:dragleave={onDragLeave}
        on:drop={(e) => onDrop(e, i)}
      >
        <TabItem {tab} active={tab.id === activeTabId} {paneId} on:close={onTabClose} />
      </div>
    {/each}
  </div>
  <button class="tab-add" on:click={onAddClick} title="Add tab">+</button>
</div>

{#if menuOpen}
  <TabAddMenu x={menuOpen.x} y={menuOpen.y} on:select={onMenuSelect} on:close={() => (menuOpen = null)} />
{/if}

<style>
  .tab-bar {
    display: flex;
    align-items: stretch;
    background: var(--bg1);
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
    min-height: 28px;
  }
  .tab-list {
    display: flex;
    overflow-x: auto;
    flex: 1;
    min-width: 0;
  }
  .tab-list::-webkit-scrollbar { height: 0; }
  .tab-drop-zone { display: flex; }
  .tab-drop-zone.drag-over { border-left: 2px solid var(--ac); }
  .tab-add {
    background: none; border: none; color: var(--t2);
    font-size: 14px; padding: 0 var(--sp-4); cursor: pointer;
    flex-shrink: 0; transition: color 0.15s;
  }
  .tab-add:hover { color: var(--ac); }
</style>
```

- [ ] **Step 4: Verify compilation**

Run: `npx svelte-check --threshold error`
Expected: 0 errors

- [ ] **Step 5: Commit**

```
feat: add TabItem, TabBar, and TabAddMenu components
```

---

## Task 4: Create SplitDropZone and PaneContainer

**Files:**
- Create: `ui/components/workspace/SplitDropZone.svelte`
- Create: `ui/components/workspace/PaneContainer.svelte`

- [ ] **Step 1: Create SplitDropZone**

Create `ui/components/workspace/SplitDropZone.svelte`:
```svelte
<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let visible: boolean = false;

  type DropPosition = 'center' | 'top' | 'bottom' | 'left' | 'right';
  let hovered: DropPosition | null = null;

  const dispatch = createEventDispatcher<{
    drop: { position: DropPosition; data: string };
  }>();

  function getZone(e: DragEvent, rect: DOMRect): DropPosition {
    const x = (e.clientX - rect.left) / rect.width;
    const y = (e.clientY - rect.top) / rect.height;
    const margin = 0.25;
    if (y < margin) return 'top';
    if (y > 1 - margin) return 'bottom';
    if (x < margin) return 'left';
    if (x > 1 - margin) return 'right';
    return 'center';
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    hovered = getZone(e, rect);
  }

  function onDragLeave() {
    hovered = null;
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    if (!hovered || !e.dataTransfer) return;
    const data = e.dataTransfer.getData('text/plain');
    dispatch('drop', { position: hovered, data });
    hovered = null;
  }
</script>

{#if visible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="drop-overlay"
    on:dragover={onDragOver}
    on:dragleave={onDragLeave}
    on:drop={onDrop}
  >
    {#if hovered}
      <div class="preview" class:preview-top={hovered === 'top'}
        class:preview-bottom={hovered === 'bottom'}
        class:preview-left={hovered === 'left'}
        class:preview-right={hovered === 'right'}
        class:preview-center={hovered === 'center'}
      ></div>
    {/if}
  </div>
{/if}

<style>
  .drop-overlay {
    position: absolute;
    inset: 0;
    z-index: 10;
  }
  .preview {
    position: absolute;
    background: rgba(0, 212, 126, 0.12);
    border: 2px solid var(--ac);
    border-radius: var(--radius-sm);
    pointer-events: none;
    transition: all 0.1s;
  }
  .preview-center { inset: 4px; }
  .preview-top { left: 4px; right: 4px; top: 4px; height: 48%; }
  .preview-bottom { left: 4px; right: 4px; bottom: 4px; height: 48%; }
  .preview-left { top: 4px; bottom: 4px; left: 4px; width: 48%; }
  .preview-right { top: 4px; bottom: 4px; right: 4px; width: 48%; }
</style>
```

- [ ] **Step 2: Create PaneContainer**

Create `ui/components/workspace/PaneContainer.svelte`:
```svelte
<script lang="ts">
  import { workspace, focusPane, moveTab, splitPane, addTab, closeTab } from '../../lib/stores/workspace';
  import type { TabTarget } from '../../lib/stores/workspace';
  import { sessions } from '../../lib/stores/sessions';
  import TabBar from './TabBar.svelte';
  import SplitDropZone from './SplitDropZone.svelte';
  import CentralPanel from '../CentralPanel.svelte';
  import TerminalPanel from '../TerminalPanel.svelte';
  import { ptyCreate, ptyKill } from '../../lib/tauri/terminal';

  export let paneId: string;

  let showDropZone = false;

  $: pane = $workspace.panes[paneId];
  $: activeTab = pane?.tabs.find((t) => t.id === pane.activeTabId) ?? null;
  $: isFocused = $workspace.focusedPaneId === paneId;

  // Session lookup for agent tabs
  $: activeSession = (() => {
    if (!activeTab || activeTab.target.kind !== 'agent') return null;
    return $sessions.find((s) => s.id === activeTab.target.sessionId) ?? null;
  })();

  // Track drag state globally
  function onDragEnter(e: DragEvent) {
    e.preventDefault();
    showDropZone = true;
  }
  function onDragLeave(e: DragEvent) {
    const related = e.relatedTarget as HTMLElement | null;
    if (related && (e.currentTarget as HTMLElement).contains(related)) return;
    showDropZone = false;
  }

  function onDropZoneDrop(e: CustomEvent<{ position: string; data: string }>) {
    showDropZone = false;
    try {
      const dragData = JSON.parse(e.detail.data);
      const { tabId, sourcePaneId, sessionId } = dragData;

      if (e.detail.position === 'center') {
        if (tabId && sourcePaneId) {
          moveTab(sourcePaneId, paneId, tabId);
        } else if (sessionId) {
          addTab(paneId, { kind: 'agent', sessionId });
        }
      } else {
        const direction = (e.detail.position === 'left' || e.detail.position === 'right')
          ? 'horizontal' : 'vertical';
        const target: TabTarget = sessionId
          ? { kind: 'agent', sessionId }
          : (() => {
              // Find the tab target from source pane
              const src = $workspace.panes[sourcePaneId];
              const tab = src?.tabs.find((t) => t.id === tabId);
              return tab?.target ?? { kind: 'agent', sessionId: 0 };
            })();

        splitPane(paneId, direction, target);

        // Remove from source if it was a tab move
        if (tabId && sourcePaneId) {
          closeTab(sourcePaneId, tabId);
        }
      }
    } catch { /* invalid drag data */ }
  }

  async function handleAddAction(e: CustomEvent<{ action: string }>) {
    if (e.detail.action === 'terminal') {
      const terminalId = crypto.randomUUID().slice(0, 8);
      addTab(paneId, { kind: 'terminal', terminalId });
    } else if (e.detail.action === 'session') {
      // Dispatch event to open NewSessionModal — handled by App.svelte
      window.dispatchEvent(new CustomEvent('orbit:new-session'));
    }
    // 'open' action would show a session picker — future enhancement
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="pane-container"
  class:focused={isFocused}
  on:click={() => focusPane(paneId)}
  on:keydown={(e) => e.key === 'Enter' && focusPane(paneId)}
  on:dragenter={onDragEnter}
  on:dragleave={onDragLeave}
>
  {#if pane && pane.tabs.length > 0}
    <TabBar {paneId} tabs={pane.tabs} activeTabId={pane.activeTabId} on:addaction={handleAddAction} />

    <div class="pane-content">
      {#if activeTab?.target.kind === 'agent' && activeSession}
        <CentralPanel session={activeSession} />
      {:else if activeTab?.target.kind === 'terminal'}
        <TerminalPanel sessionId={0} terminalId={activeTab.target.terminalId} />
      {:else}
        <div class="pane-empty">
          <span class="hint">session not found</span>
        </div>
      {/if}
    </div>
  {:else}
    <div class="pane-empty">
      <span class="plus">+</span>
      <span class="hint">drag a session here or click + to add a tab</span>
      <TabBar {paneId} tabs={[]} activeTabId={null} on:addaction={handleAddAction} />
    </div>
  {/if}

  <SplitDropZone visible={showDropZone} on:drop={onDropZoneDrop} />
</div>

<style>
  .pane-container {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--bg);
  }
  .pane-container.focused::before {
    content: '';
    position: absolute;
    top: 0; left: 0; right: 0;
    height: 2px;
    background: var(--ac);
    z-index: 5;
    pointer-events: none;
  }
  .pane-content {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }
  .pane-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sp-3);
  }
  .plus { font-size: 28px; color: var(--t3); }
  .hint { font-size: var(--xs); color: var(--t3); text-align: center; max-width: 200px; }
</style>
```

- [ ] **Step 3: Verify compilation**

Run: `npx svelte-check --threshold error`
Expected: 0 errors

- [ ] **Step 4: Commit**

```
feat: add PaneContainer with drop zones and SplitDropZone overlay
```

---

## Task 5: Wire WorkspaceContainer into App.svelte and update Sidebar

**Files:**
- Modify: `ui/App.svelte`
- Modify: `ui/components/Sidebar.svelte`
- Modify: `ui/components/CentralPanel.svelte`

- [ ] **Step 1: Update App.svelte to use WorkspaceContainer**

In `ui/App.svelte`:
- Replace `import PaneGrid from './components/PaneGrid.svelte'` with `import WorkspaceContainer from './components/workspace/WorkspaceContainer.svelte'`
- Replace `import { assignSession } from './lib/stores/layout'` with `import { addTab, restoreWorkspace, workspace } from './lib/stores/workspace'`
- In `onMount`, replace `assignSession('tl', existing[0].id)` with `addTab($workspace.focusedPaneId!, { kind: 'agent', sessionId: existing[0].id })`
- Replace `<PaneGrid />` with `<WorkspaceContainer />`
- In `onSessionCreated`, replace `assignSession` with `addTab`
- Call `restoreWorkspace(new Set(existing.map(s => s.id)))` after loading sessions
- Update `selectedSessionId` derived store usage to read from workspace's focused pane's active agent tab

- [ ] **Step 2: Update Sidebar to use workspace store**

In `ui/components/Sidebar.svelte`:
- Replace `import { splitLayout, assignSession, clearSession } from '../lib/stores/layout'` with `import { workspace, addTab, splitPane } from '../lib/stores/workspace'`
- Click handler: `addTab($workspace.focusedPaneId!, { kind: 'agent', sessionId: s.id })`
- Double-click handler: `splitPane($workspace.focusedPaneId!, 'horizontal', { kind: 'agent', sessionId: s.id })`
- Add `draggable="true"` to session items with `dragstart` setting `JSON.stringify({ sessionId: s.id })`
- Remove `clearSession` calls (workspace handles this via tab close)
- Active state: check if session has a tab in any pane (not just grid position)

- [ ] **Step 3: Simplify CentralPanel**

In `ui/components/CentralPanel.svelte`:
- Remove `type ViewTab`, `activeTab`, `ptyActive`, `ptyLoading`, `ptyError` state
- Remove `activateTerminal()`, `switchTab()` functions
- Remove the tab bar (Feed/Terminal toggle) from the template
- Remove terminal-related CSS
- Remove `import TerminalPanel` and `import { ptyCreate, ptyKill }`
- Keep: header, branch strip, approval banner, feed, scroll button, input bar
- Remove `onSplit` and `onClose` props (handled by workspace now)

- [ ] **Step 4: Update MetaPanel to accept tab target**

In `ui/components/MetaPanel.svelte`:
- Add optional `tabKind` prop
- When `tabKind === 'terminal'`, show minimal info (shell, cwd)
- When `tabKind === 'agent'` (or not set), show existing session info

- [ ] **Step 5: Delete old layout files**

Delete:
- `ui/lib/stores/layout.ts`
- `ui/components/PaneGrid.svelte`
- `ui/components/Pane.svelte`

- [ ] **Step 6: Verify everything compiles**

Run: `npx svelte-check --threshold error`
Run: `npx eslint ui/ --max-warnings 0`
Expected: 0 errors

- [ ] **Step 7: Commit**

```
feat: wire workspace container into App, update Sidebar and CentralPanel
```

---

## Task 6: Update TerminalPanel for standalone use

**Files:**
- Modify: `ui/components/TerminalPanel.svelte`

- [ ] **Step 1: Add terminalId prop and auto-spawn**

Update `ui/components/TerminalPanel.svelte`:
- Add `terminalId` prop (string) alongside existing `sessionId`
- On mount, auto-spawn a shell PTY (the component manages its own PTY lifecycle)
- Use `terminalId` as the key for PTY operations instead of `sessionId` when in standalone mode
- Show error state if PTY spawn fails
- On destroy, kill the PTY

- [ ] **Step 2: Verify compilation**

Run: `npx svelte-check --threshold error`
Expected: 0 errors

- [ ] **Step 3: Commit**

```
feat: update TerminalPanel for standalone workspace tab use
```

---

## Task 7: Final integration and cleanup

**Files:**
- Modify: `ui/App.svelte` (final wiring)
- Modify: `ui/components/workspace/PaneContainer.svelte` (session name in tab label)

- [ ] **Step 1: Enrich tab labels with session names**

In `TabItem.svelte`, look up the session name from the sessions store for agent tabs:
```typescript
import { sessions } from '../../lib/stores/sessions';
// In the label reactive:
case 'agent': {
  const s = $sessions.find(s => s.id === tab.target.sessionId);
  return s?.name ?? s?.projectName ?? `#${tab.target.sessionId}`;
}
```

- [ ] **Step 2: Handle session:created event**

In `App.svelte`, when a new session is created, auto-open it as a tab in the focused pane:
```typescript
const u2 = onSessionCreated((s) => {
  sessions.update((l) => upsertSession(l, s));
  const ws = get(workspace);
  if (ws.focusedPaneId) {
    addTab(ws.focusedPaneId, { kind: 'agent', sessionId: s.id });
  }
});
```

- [ ] **Step 3: Handle new-session event from PaneContainer**

Listen for `orbit:new-session` custom event in App.svelte to open NewSessionModal:
```typescript
window.addEventListener('orbit:new-session', () => { showNewSession = true; });
```

- [ ] **Step 4: Full verification**

Run: `npx svelte-check --threshold error`
Run: `npx eslint ui/ --max-warnings 0`
Run: `cargo test --manifest-path tauri/Cargo.toml --lib`
Expected: All pass

- [ ] **Step 5: Commit**

```
feat: complete workspace tab integration with session labels and events
```

---

## Execution Order

Tasks are ordered by dependency:

1. **Task 1**: Workspace store (foundation — all components depend on this)
2. **Task 2**: WorkspaceContainer + SplitContainer (renders the tree)
3. **Task 3**: TabItem + TabBar + TabAddMenu (tab UI within panes)
4. **Task 4**: SplitDropZone + PaneContainer (ties tabs + content + drop zones)
5. **Task 5**: Wire into App + Sidebar + CentralPanel (replace old system)
6. **Task 6**: TerminalPanel standalone mode
7. **Task 7**: Final integration, labels, events, cleanup
