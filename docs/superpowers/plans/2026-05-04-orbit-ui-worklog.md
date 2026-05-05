# Orbit UI Worklog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the workspace tabs, shared panel header, xterm terminal, and Git overview UI work described in the 2026-05-04 Orbit UI worklog.

**Architecture:** Keep the workspace as a split-tree store where each leaf pane owns ordered tabs and one active tab. Use a shared `PanelHeader` for panels that need consistent title/status/action chrome, while each panel owns its content-specific behavior. Add Git overview through the existing Tauri command boundary: frontend calls a new `ui/lib/tauri/git.ts` wrapper, mock mode implements the same command names, and Rust owns all real Git CLI execution.

**Tech Stack:** SvelteKit 2.9, Svelte 5, TypeScript 5.6, Vite 6, Tauri 2, Rust 1.85, xterm.js, Monaco Editor, lucide-svelte, Vitest, ESLint, svelte-check, cargo check/clippy.

---

## Scope And Commit Strategy

Implement this as four logical blocks, not step-by-step commits:

1. `feat: add tabbed workspace panels` after Tasks 1-3 pass verification.
2. `feat: add xterm terminal panels` after Task 4 passes verification.
3. `feat: add git overview panel` after Tasks 5-8 pass verification.
4. `chore: update ui worklog changelog` only if user-facing entries need a separate cleanup commit; otherwise include the changelog in the relevant feature commit.

Before each commit, run checks manually. Do not rely only on the pre-commit hook.

---

## File Structure

### Workspace Tabs

- Modify: `ui/lib/stores/workspace.ts`
- Responsibility: define `TabTarget`, `Tab`, tab-aware `PaneState`, split tree state, tab actions, move/reorder actions, persistence, and session cleanup.
- Modify: `ui/components/workspace/PaneContainer.svelte`
- Responsibility: render the `TabBar`, active tab content, drop zones, and pane focus/close behavior.
- Modify: `ui/components/workspace/TabBar.svelte`
- Responsibility: display ordered tabs, same-pane reorder drop targets, add menu positioning, and tab events.
- Modify: `ui/components/workspace/TabItem.svelte`
- Responsibility: render minimal tab chrome with lucide icons, labels, drag payloads, and always-visible close controls.
- Modify: `ui/components/workspace/TabAddMenu.svelte`
- Responsibility: show the `+` dropdown clamped inside the viewport.
- Modify: `ui/components/workspace/SplitDropZone.svelte`
- Responsibility: accept tab and session drag payloads and emit drop position plus raw payload.
- Modify if needed: `ui/components/workspace/WorkspaceContainer.svelte`, `ui/components/workspace/SplitContainer.svelte`
- Responsibility: preserve split-tree rendering and resizing after panes become tab-aware.

### Shared Header

- Create: `ui/components/workspace/PanelHeader.svelte`
- Responsibility: shared visual header with title, subtitle, status, meta slot, actions slot, close button, and optional drag payload.
- Modify: `ui/app.css`
- Responsibility: add global utility classes for compact header buttons and pills only if component-local styles become duplicated.

### Terminal

- Modify: `ui/components/TerminalPanel.svelte`
- Responsibility: render xterm, spawn/attach PTY, fit to container, accept `cwd`, `terminalId`, `paneId`, and close callback, and use `PanelHeader`.
- Modify: `ui/lib/tauri/terminal.ts`
- Responsibility: keep typed wrappers for PTY commands and events.
- Verify existing Rust PTY commands before editing. If command names do not exist, add them in the current terminal command module instead of inventing frontend-only behavior.

### Git Overview

- Create: `docs/specs/git-overview.md`
- Responsibility: feature spec required before implementing new Git overview behavior.
- Create: `ui/components/GitPanel.svelte`
- Responsibility: status summary, branch dropdown with local/remotes separated, commits/stashes sections, changed-file list, and diff modal orchestration.
- Create: `ui/components/MonacoDiffViewer.svelte`
- Responsibility: lazy Monaco diff editor wrapper that accepts `original`, `modified`, `language`, and `height`.
- Create: `ui/lib/tauri/git.ts`
- Responsibility: typed frontend wrapper for Git Tauri commands.
- Modify: `ui/lib/mock/tauri-mock.ts`
- Responsibility: mock the Git commands with realistic branch/status/commit/stash/diff data.
- Create or modify: `tauri/src/commands/git.rs`
- Responsibility: run Git commands safely in a supplied repo path, parse branch/status/commit/stash data, and return diff file pairs for Monaco.
- Modify: `tauri/src/commands/mod.rs`
- Responsibility: expose the Git command module.
- Modify: `tauri/src/lib.rs`
- Responsibility: register new Git Tauri commands with `invoke_handler`.
- Modify: `package.json`, `package-lock.json`
- Responsibility: add `monaco-editor`, `@xterm/xterm`, `@xterm/addon-fit`, `@xterm/addon-unicode11`, `@xterm/addon-webgl`, and `lucide-svelte` if not already installed.
- Modify: `CHANGELOG.md`
- Responsibility: user-facing entry for workspace tabs, terminal panel, and Git overview.

---

## Task 1: Tab-Aware Workspace Store

**Files:**

- Modify: `ui/lib/stores/workspace.ts`
- Test: add inline Vitest coverage only if a workspace store test file exists; otherwise verify through `npm run check` and manual mock app testing.

- [ ] **Step 1: Replace session-only pane state with tab-aware types**

Update the type section of `ui/lib/stores/workspace.ts` so it exposes tab primitives used by `TabBar.svelte` and `TabItem.svelte`:

```ts
export type TabTarget =
  | { kind: 'agent'; sessionId: number }
  | { kind: 'terminal'; terminalId: string; cwd: string }
  | { kind: 'git'; cwd: string };

export interface Tab {
  id: string;
  target: TabTarget;
  createdAt: number;
}

export interface PaneState {
  tabs: Tab[];
  activeTabId: string | null;
}
```

Keep `SplitNode` and `WorkspaceState` unchanged except that `panes` now maps to the tab-aware `PaneState`.

- [ ] **Step 2: Add tab creation helpers**

Add these helpers near `newPaneId()`:

```ts
function newTabId(): string {
  if (crypto.randomUUID) return crypto.randomUUID();
  return `tab-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export function createTab(target: TabTarget): Tab {
  return {
    id: newTabId(),
    target,
    createdAt: Date.now(),
  };
}

function createPaneState(tab?: Tab): PaneState {
  return {
    tabs: tab ? [tab] : [],
    activeTabId: tab?.id ?? null,
  };
}
```

Update `defaultState()` to create `{ [paneId]: createPaneState() }`.

- [ ] **Step 3: Implement tab actions**

Replace the session-only actions with these exported actions:

```ts
export function addTab(paneId: string, target: TabTarget): void {
  workspace.update((ws) => {
    const pane = ws.panes[paneId];
    if (!pane) return ws;

    const existing = findTabByTarget(ws, target);
    if (existing) {
      ws.focusedPaneId = existing.paneId;
      ws.panes[existing.paneId].activeTabId = existing.tab.id;
      return ws;
    }

    const tab = createTab(target);
    pane.tabs.push(tab);
    pane.activeTabId = tab.id;
    ws.focusedPaneId = paneId;
    return ws;
  });
}

export function closeTab(paneId: string, tabId: string): void {
  workspace.update((ws) => {
    const pane = ws.panes[paneId];
    if (!pane) return ws;

    const index = pane.tabs.findIndex((tab) => tab.id === tabId);
    if (index === -1) return ws;

    pane.tabs.splice(index, 1);
    if (pane.activeTabId === tabId) {
      pane.activeTabId = pane.tabs[Math.max(0, index - 1)]?.id ?? pane.tabs[0]?.id ?? null;
    }

    compact(ws);
    return ws;
  });
}

export function setActiveTab(paneId: string, tabId: string): void {
  workspace.update((ws) => {
    const pane = ws.panes[paneId];
    if (!pane || !pane.tabs.some((tab) => tab.id === tabId)) return ws;
    pane.activeTabId = tabId;
    ws.focusedPaneId = paneId;
    return ws;
  });
}

export function reorderTab(paneId: string, fromIndex: number, toIndex: number): void {
  workspace.update((ws) => {
    const pane = ws.panes[paneId];
    if (!pane) return ws;
    if (fromIndex < 0 || fromIndex >= pane.tabs.length) return ws;
    const boundedTo = Math.min(Math.max(toIndex, 0), pane.tabs.length - 1);
    const [tab] = pane.tabs.splice(fromIndex, 1);
    pane.tabs.splice(boundedTo, 0, tab);
    return ws;
  });
}

export function moveTab(fromPaneId: string, toPaneId: string, tabId: string): void {
  workspace.update((ws) => {
    if (fromPaneId === toPaneId) return ws;
    const fromPane = ws.panes[fromPaneId];
    const toPane = ws.panes[toPaneId];
    if (!fromPane || !toPane) return ws;

    const index = fromPane.tabs.findIndex((tab) => tab.id === tabId);
    if (index === -1) return ws;

    const [tab] = fromPane.tabs.splice(index, 1);
    toPane.tabs.push(tab);
    toPane.activeTabId = tab.id;
    ws.focusedPaneId = toPaneId;

    if (fromPane.activeTabId === tabId) {
      fromPane.activeTabId = fromPane.tabs[Math.max(0, index - 1)]?.id ?? fromPane.tabs[0]?.id ?? null;
    }

    compact(ws);
    return ws;
  });
}
```

- [ ] **Step 4: Update split and compact behavior**

Change `splitPane` to accept a `Tab`, not `sessionId`:

```ts
export function splitPane(
  paneId: string,
  direction: 'horizontal' | 'vertical',
  tab: Tab,
  insertBefore = false
): void {
  workspace.update((ws) => {
    if (!ws.panes[paneId]) return ws;

    const freshPaneId = newPaneId();
    ws.panes[freshPaneId] = createPaneState(tab);

    const children: [SplitNode, SplitNode] = insertBefore
      ? [
          { type: 'leaf', paneId: freshPaneId },
          { type: 'leaf', paneId },
        ]
      : [
          { type: 'leaf', paneId },
          { type: 'leaf', paneId: freshPaneId },
        ];

    ws.root = replaceLeaf(ws.root, paneId, {
      type: 'split',
      direction,
      ratio: 0.5,
      children,
    });
    ws.focusedPaneId = freshPaneId;
    compact(ws);
    return ws;
  });
}
```

Update `compactTree()` so a leaf is empty when `panes[paneId]?.tabs.length === 0`.

- [ ] **Step 5: Preserve compatibility helpers for session selection**

Keep existing callers working by replacing `assignSession`, `clearSession`, and `findPaneWithSession` with tab-aware implementations:

```ts
export function assignSession(paneId: string, sessionId: number): void {
  addTab(paneId, { kind: 'agent', sessionId });
}

export function clearSession(sessionId: number): void {
  workspace.update((ws) => {
    for (const pane of Object.values(ws.panes)) {
      pane.tabs = pane.tabs.filter(
        (tab) => !(tab.target.kind === 'agent' && tab.target.sessionId === sessionId)
      );
      if (pane.activeTabId && !pane.tabs.some((tab) => tab.id === pane.activeTabId)) {
        pane.activeTabId = pane.tabs[0]?.id ?? null;
      }
    }
    compact(ws);
    return ws;
  });
}

export function findPaneWithSession(sessionId: number): string | null {
  const ws = get(workspace);
  const found = findTabByTarget(ws, { kind: 'agent', sessionId });
  return found?.paneId ?? null;
}
```

Add the private helper used above:

```ts
function sameTarget(a: TabTarget, b: TabTarget): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === 'agent' && b.kind === 'agent') return a.sessionId === b.sessionId;
  if (a.kind === 'terminal' && b.kind === 'terminal') return a.terminalId === b.terminalId;
  if (a.kind === 'git' && b.kind === 'git') return a.cwd === b.cwd;
  return false;
}

function findTabByTarget(
  ws: WorkspaceState,
  target: TabTarget
): { paneId: string; tab: Tab } | null {
  for (const [paneId, pane] of Object.entries(ws.panes)) {
    const tab = pane.tabs.find((candidate) => sameTarget(candidate.target, target));
    if (tab) return { paneId, tab };
  }
  return null;
}
```

- [ ] **Step 6: Run typecheck for the store migration**

Run:

```bash
npm run check
```

Expected: this may fail because components still use old `pane.sessionId` shape. Continue to Task 2 and fix those component errors there.

---

## Task 2: Minimal Tab UI And Pane Rendering

**Files:**

- Modify: `ui/components/workspace/PaneContainer.svelte`
- Modify: `ui/components/workspace/TabBar.svelte`
- Modify: `ui/components/workspace/TabItem.svelte`
- Modify: `ui/components/workspace/TabAddMenu.svelte`
- Modify: `ui/components/workspace/SplitDropZone.svelte`

- [ ] **Step 1: Update `PaneContainer.svelte` imports and derived values**

Use tab-aware imports:

```ts
import {
  workspace,
  focusPane,
  addTab,
  splitPane,
  closePane,
  moveTab,
  createTab,
  type Tab,
} from '../../lib/stores/workspace';
import { sessions } from '../../lib/stores/sessions';
import SplitDropZone from './SplitDropZone.svelte';
import TabBar from './TabBar.svelte';
import CentralPanel from '../CentralPanel.svelte';
import TerminalPanel from '../TerminalPanel.svelte';
import GitPanel from '../GitPanel.svelte';
```

Set derived pane values like this:

```ts
$: pane = $workspace.panes[paneId];
$: activeTab = pane?.tabs.find((tab) => tab.id === pane.activeTabId) ?? pane?.tabs[0] ?? null;
$: session = activeTab?.target.kind === 'agent'
  ? ($sessions.find((s) => s.id === activeTab.target.sessionId) ?? null)
  : null;
$: isFocused = $workspace.focusedPaneId === paneId;
$: canClose = Object.keys($workspace.panes).length > 1;
```

- [ ] **Step 2: Update drop handling in `PaneContainer.svelte`**

Use a helper to parse session or tab drag payloads:

```ts
function tabFromDropData(data: string): { tab: Tab; sourcePaneId: string | null; sourceTabId: string | null } | null {
  try {
    const parsed = JSON.parse(data) as {
      tabId?: string;
      sourcePaneId?: string;
      sessionId?: number;
      target?: Tab['target'];
    };

    if (parsed.target) {
      return {
        tab: createTab(parsed.target),
        sourcePaneId: parsed.sourcePaneId ?? null,
        sourceTabId: parsed.tabId ?? null,
      };
    }

    if (typeof parsed.sessionId === 'number') {
      return {
        tab: createTab({ kind: 'agent', sessionId: parsed.sessionId }),
        sourcePaneId: parsed.sourcePaneId ?? null,
        sourceTabId: null,
      };
    }
  } catch {
    const sessionId = Number(data);
    if (Number.isFinite(sessionId)) {
      return { tab: createTab({ kind: 'agent', sessionId }), sourcePaneId: null, sourceTabId: null };
    }
  }

  return null;
}
```

Update `handleSplitDrop`:

```ts
function handleSplitDrop(
  e: CustomEvent<{
    position: 'center' | 'top' | 'bottom' | 'left' | 'right';
    data: string;
  }>
) {
  dragOver = false;
  dragEnterCount = 0;

  const parsed = tabFromDropData(e.detail.data);
  if (!parsed) return;

  if (e.detail.position === 'center') {
    if (parsed.sourcePaneId && parsed.sourceTabId) {
      moveTab(parsed.sourcePaneId, paneId, parsed.sourceTabId);
    } else {
      addTab(paneId, parsed.tab.target);
    }
    return;
  }

  const direction: 'horizontal' | 'vertical' =
    e.detail.position === 'left' || e.detail.position === 'right' ? 'horizontal' : 'vertical';
  const insertBefore = e.detail.position === 'left' || e.detail.position === 'top';
  splitPane(paneId, direction, parsed.tab, insertBefore);
}
```

- [ ] **Step 3: Render tab bar and active tab content**

Replace the old session-only markup in `PaneContainer.svelte` with:

```svelte
<div
  class="pane-container"
  class:focused={isFocused}
  role="region"
  on:click={() => focusPane(paneId)}
  on:keydown={() => focusPane(paneId)}
  on:dragenter={handleDragEnter}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
>
  {#if pane}
    <TabBar
      {paneId}
      tabs={pane.tabs}
      activeTabId={pane.activeTabId}
      on:addaction={(e) => handleAddAction(e.detail.action)}
    />
  {/if}

  <div class="pane-content">
    {#if activeTab?.target.kind === 'agent' && session}
      <CentralPanel {session} {paneId} onClose={canClose ? () => closePane(paneId) : null} />
    {:else if activeTab?.target.kind === 'terminal'}
      <TerminalPanel
        terminalId={activeTab.target.terminalId}
        cwd={activeTab.target.cwd}
        {paneId}
        onClose={() => closeTab(paneId, activeTab.id)}
      />
    {:else if activeTab?.target.kind === 'git'}
      <GitPanel cwd={activeTab.target.cwd} {paneId} />
    {:else}
      <div class="empty-state">
        <span class="icon">+</span>
        <span class="hint">click a session in the sidebar or use + to open a panel</span>
      </div>
    {/if}
  </div>

  <SplitDropZone visible={dragOver} on:drop={handleSplitDrop} />
</div>
```

Add this handler:

```ts
function handleAddAction(action: 'terminal' | 'session' | 'open' | 'git') {
  const cwd = session?.cwd ?? '.';
  if (action === 'terminal') {
    addTab(paneId, { kind: 'terminal', terminalId: crypto.randomUUID(), cwd });
  } else if (action === 'git') {
    addTab(paneId, { kind: 'git', cwd });
  }
}
```

For `session` and `open`, wire to the existing new/open session flow if one already exists in the app; otherwise leave the menu action disabled in Task 2 and enable it in a later feature.

- [ ] **Step 4: Add pane content CSS**

Add this to the `PaneContainer.svelte` style block:

```css
.pane-content {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
```

- [ ] **Step 5: Update `TabItem.svelte` to use lucide icons and target labels**

Replace the icon logic with lucide components:

```ts
import { Bot, Terminal, GitBranch } from 'lucide-svelte';
```

Use this label logic:

```ts
$: label = (() => {
  if (tab.target.kind === 'agent') {
    const s = $sessions.find((s) => s.id === tab.target.sessionId);
    return s?.name ?? s?.projectName ?? s?.cwd?.split(/[\\/]/).pop() ?? `#${tab.target.sessionId}`;
  }
  if (tab.target.kind === 'git') return 'Git';
  return 'Terminal';
})();
```

Render the icon block like this:

```svelte
<span class="tab-icon" aria-hidden="true">
  {#if tab.target.kind === 'agent'}
    <Bot size={13} />
  {:else if tab.target.kind === 'git'}
    <GitBranch size={13} />
  {:else}
    <Terminal size={13} />
  {/if}
</span>
```

Set drag data to include the target:

```ts
e.dataTransfer!.setData(
  'text/plain',
  JSON.stringify({ tabId: tab.id, sourcePaneId: paneId, target: tab.target })
);
```

- [ ] **Step 6: Ensure inactive tabs remain legible**

Update `TabItem.svelte` CSS so inactive tabs have subtle borders/backgrounds and close is always visible:

```css
.tab-item {
  border: 1px solid rgba(255, 255, 255, 0.04);
  border-bottom-color: transparent;
  background: color-mix(in srgb, var(--bg1) 88%, transparent);
}

.tab-item:not(.active) .tab-close {
  opacity: 0.55;
}

.tab-item.active {
  border-color: var(--bd1);
  border-bottom-color: var(--ac);
}
```

If `color-mix` conflicts with the app's browser target, use `background: var(--bg1); opacity` on borders instead.

- [ ] **Step 7: Clamp the add menu to the viewport**

In `TabAddMenu.svelte`, compute clamped coordinates:

```ts
$: menuLeft = Math.min(x, window.innerWidth - 176);
$: menuTop = Math.min(y, window.innerHeight - 132);
```

Use:

```svelte
<div class="menu" style="left: {menuLeft}px; top: {menuTop}px;" role="menu">
```

Add a Git menu item:

```svelte
<button class="menu-item" role="menuitem" on:click={() => select('git')}>
  <span class="item-icon">git</span>
  Git overview
</button>
```

Update dispatcher and `select` action types to `'terminal' | 'session' | 'open' | 'git'`.

- [ ] **Step 8: Verify the workspace tab block**

Run:

```bash
npm run check
```

Expected: no TypeScript/Svelte errors from workspace files. Errors about missing `GitPanel.svelte` are acceptable until Task 5 if `PaneContainer.svelte` already imports it; create a temporary stub only if needed to keep Task 2 independently checkable.

---

## Task 3: Shared Panel Header

**Files:**

- Create: `ui/components/workspace/PanelHeader.svelte`
- Modify: `ui/app.css` only if repeated utility styles are needed.
- Modify later users in Tasks 4 and 5.

- [ ] **Step 1: Create `PanelHeader.svelte`**

Create `ui/components/workspace/PanelHeader.svelte` with:

```svelte
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { X } from 'lucide-svelte';

  export let title: string;
  export let subtitle: string | null = null;
  export let status: string | null = null;
  export let dragPayload: string | null = null;
  export let closeLabel = 'Close panel';
  export let onClose: (() => void) | null = null;

  const dispatch = createEventDispatcher<{ dragstart: DragEvent }>();

  function handleDragStart(e: DragEvent) {
    if (!dragPayload) return;
    e.dataTransfer?.setData('text/plain', dragPayload);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
    dispatch('dragstart', e);
  }
</script>

<header class="panel-header" draggable={!!dragPayload} on:dragstart={handleDragStart}>
  <div class="panel-title-block">
    <div class="panel-title-row">
      <span class="panel-title">{title}</span>
      {#if status}
        <span class="panel-status">{status}</span>
      {/if}
    </div>
    {#if subtitle}
      <span class="panel-subtitle">{subtitle}</span>
    {/if}
  </div>

  <div class="panel-meta">
    <slot name="meta" />
  </div>

  <div class="panel-actions">
    <slot name="actions" />
    {#if onClose}
      <button class="panel-icon-button" type="button" aria-label={closeLabel} on:click={onClose}>
        <X size={14} />
      </button>
    {/if}
  </div>
</header>

<style>
  .panel-header {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    min-height: 38px;
    padding: 0 var(--sp-5);
    border-bottom: 1px solid var(--bd);
    background: var(--bg1);
    flex-shrink: 0;
    user-select: none;
  }

  .panel-header[draggable='true'] {
    cursor: grab;
  }

  .panel-title-block {
    min-width: 0;
    flex: 1;
  }

  .panel-title-row {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    min-width: 0;
  }

  .panel-title {
    overflow: hidden;
    color: var(--t0);
    font-size: var(--sm);
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .panel-subtitle {
    display: block;
    overflow: hidden;
    color: var(--t3);
    font-size: var(--xs);
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .panel-status {
    border: 1px solid var(--bd1);
    border-radius: 999px;
    padding: 0 var(--sp-2);
    color: var(--t2);
    font-size: var(--xs);
  }

  .panel-meta,
  .panel-actions {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    flex-shrink: 0;
  }

  .panel-icon-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--t2);
  }

  .panel-icon-button:hover {
    border-color: var(--bd1);
    background: var(--bg2);
    color: var(--t0);
  }
</style>
```

- [ ] **Step 2: Verify header typecheck**

Run:

```bash
npm run check
```

Expected: no errors from `PanelHeader.svelte`.

---

## Task 4: Xterm Terminal Panel

**Files:**

- Modify: `ui/components/TerminalPanel.svelte`
- Modify: `ui/lib/tauri/terminal.ts` only if command types are incomplete.

- [ ] **Step 1: Update terminal props**

Change props in `TerminalPanel.svelte` to:

```ts
let {
  cwd = '.',
  terminalId = '',
  paneId = '',
  onClose = null,
}: {
  cwd?: string;
  terminalId?: string;
  paneId?: string;
  onClose?: (() => void) | null;
} = $props();
```

Remove `sessionId` unless another caller still uses it. If another caller still uses it, keep `sessionId?: number` and prefer `terminalId` when provided.

- [ ] **Step 2: Use cwd in PTY spawn**

Replace the hardcoded `const cwd = '.';` in `spawnPty()` with the prop value:

```ts
await ptyCreate(numericId, shell, [], cwd, [], rows, cols);
```

Keep shell selection as:

```ts
const isWindows = navigator.platform.startsWith('Win');
const shell = isWindows ? 'powershell.exe' : '/bin/bash';
```

- [ ] **Step 3: Add `PanelHeader`**

Import:

```ts
import PanelHeader from './workspace/PanelHeader.svelte';
```

Wrap the terminal markup:

```svelte
<section class="terminal-shell">
  <PanelHeader
    title="Terminal"
    subtitle={cwd}
    status={loading ? 'starting' : error ? 'error' : 'ready'}
    dragPayload={JSON.stringify({
      sourcePaneId: paneId,
      target: { kind: 'terminal', terminalId, cwd },
    })}
    {onClose}
  />

  <div class="terminal-body">
    {#if loading}
      <div class="terminal-overlay">
        <span class="terminal-status">Starting shell...</span>
      </div>
    {:else if error}
      <div class="terminal-overlay">
        <span class="terminal-status error">{error}</span>
        <button class="retry-btn" onclick={() => { terminal?.dispose(); terminal = undefined; initTerminal(); }}>Retry</button>
      </div>
    {/if}

    <div class="terminal-panel" bind:this={container} class:hidden={!!error || loading}></div>
  </div>
</section>
```

Use ASCII `...` instead of the ellipsis character if the file does not already use non-ASCII.

- [ ] **Step 4: Adjust terminal layout CSS**

Add:

```css
.terminal-shell {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  background: #1a1a1a;
}

.terminal-body {
  position: relative;
  flex: 1;
  min-width: 0;
  min-height: 0;
}
```

Change `.terminal-panel` and `.terminal-overlay` to use `height: 100%` and remove `min-height: 200px` unless needed for standalone rendering.

- [ ] **Step 5: Verify terminal block**

Run:

```bash
npm run check
npm run build
```

Expected: both pass. Build may warn that Monaco chunks are large after Task 6; that warning is acceptable.

Manual verification:

```bash
npm run dev:mock
```

Expected: terminal tab renders without a real shell in browser mock. Real shell must be verified later with:

```bash
npm run tauri:dev
```

---

## Task 5: Git Overview Spec And Frontend Contract

**Files:**

- Create: `docs/specs/git-overview.md`
- Create: `ui/lib/tauri/git.ts`
- Modify: `ui/lib/mock/tauri-mock.ts`

- [ ] **Step 1: Create the Git overview spec**

Create `docs/specs/git-overview.md`:

```markdown
# Git Overview

## Goal

Add a workspace panel that summarizes the current Git repository, lets users inspect local and remote branches separately, and opens changed-file diffs in a large modal.

## Expected Behavior

- Users can open a Git overview tab from the workspace tab `+` menu.
- The panel shows current branch, ahead/behind counts, changed files, recent commits, and stashes.
- The branch dropdown groups local branches and remote branches.
- Clicking a changed file opens a nearly full-screen Monaco diff modal.
- The diff modal closes through its close button, outside click, or `Esc`.
- Mock mode provides deterministic Git data so the UI can be tested without Tauri.

## Edge Cases

- Non-Git directories show a clear empty/error state without crashing the panel.
- Untracked files diff from empty content to working-tree content.
- Deleted files diff from base content to empty content.
- Staged files diff from `HEAD:path` to index content.
- Unstaged files diff from index content to working-tree content.
- Large Monaco chunks are acceptable initially; lazy loading can be optimized later if needed.

## Acceptance Criteria

- `npm run check` passes with no errors or warnings.
- `npm run build` passes; Monaco chunk-size warnings are documented as acceptable.
- `cargo check --manifest-path tauri/Cargo.toml` passes.
- `git diff --check` passes.
- In mock mode, the Git panel shows status, grouped branches, commits, stashes, and an opening diff modal.
- In Tauri mode, a real Git repository returns branch/status data and changed-file diffs.
```

- [ ] **Step 2: Add typed Git wrapper**

Create `ui/lib/tauri/git.ts`:

```ts
import { invoke } from './invoke';

export interface GitBranchInfo {
  name: string;
  fullName: string;
  kind: 'local' | 'remote';
  current: boolean;
  upstream: string | null;
  ahead: number;
  behind: number;
}

export interface GitFileChange {
  path: string;
  status: string;
  staged: boolean;
  untracked: boolean;
}

export interface GitCommitInfo {
  hash: string;
  shortHash: string;
  subject: string;
  author: string;
  relativeTime: string;
}

export interface GitStashInfo {
  index: number;
  name: string;
  message: string;
}

export interface GitOverview {
  cwd: string;
  branch: string | null;
  upstream: string | null;
  ahead: number;
  behind: number;
  files: GitFileChange[];
  branches: GitBranchInfo[];
  commits: GitCommitInfo[];
  stashes: GitStashInfo[];
}

export interface GitDiffFile {
  path: string;
  language: string;
  original: string;
  modified: string;
}

export function gitOverview(cwd: string): Promise<GitOverview> {
  return invoke<GitOverview>('git_overview', { cwd });
}

export function gitDiffFile(cwd: string, path: string, staged: boolean): Promise<GitDiffFile> {
  return invoke<GitDiffFile>('git_diff_file', { cwd, path, staged });
}
```

- [ ] **Step 3: Add mock Git command responses**

In `ui/lib/mock/tauri-mock.ts`, add mock objects:

```ts
const MOCK_GIT_OVERVIEW = {
  cwd: 'C:\\Users\\dev\\dashboard',
  branch: 'feat/workspace-tabs',
  upstream: 'origin/feat/workspace-tabs',
  ahead: 3,
  behind: 1,
  files: [
    { path: 'ui/components/workspace/TabItem.svelte', status: 'modified', staged: false, untracked: false },
    { path: 'ui/components/GitPanel.svelte', status: 'added', staged: true, untracked: false },
    { path: 'notes/mock-only.md', status: 'untracked', staged: false, untracked: true },
  ],
  branches: [
    { name: 'main', fullName: 'refs/heads/main', kind: 'local', current: false, upstream: 'origin/main', ahead: 0, behind: 2 },
    { name: 'feat/workspace-tabs', fullName: 'refs/heads/feat/workspace-tabs', kind: 'local', current: true, upstream: 'origin/feat/workspace-tabs', ahead: 3, behind: 1 },
    { name: 'origin/main', fullName: 'refs/remotes/origin/main', kind: 'remote', current: false, upstream: null, ahead: 0, behind: 0 },
  ],
  commits: [
    { hash: '9f0e7d6c5b4a', shortHash: '9f0e7d6', subject: 'feat: add tabbed workspace panels', author: 'Fernando', relativeTime: '2 hours ago' },
    { hash: '8e7d6c5b4a3f', shortHash: '8e7d6c5', subject: 'fix: keep diff modal visible', author: 'Fernando', relativeTime: 'yesterday' },
  ],
  stashes: [
    { index: 0, name: 'stash@{0}', message: 'WIP on feat/workspace-tabs: mock data' },
  ],
};

const MOCK_GIT_DIFF = {
  path: 'ui/components/workspace/TabItem.svelte',
  language: 'svelte',
  original: '<script lang="ts">\n  export let active = false;\n</script>\n',
  modified: '<script lang="ts">\n  export let active = false;\n  export let paneId = "";\n</script>\n',
};
```

Then add cases in the mock invoke switch/function:

```ts
case 'git_overview':
  return MOCK_GIT_OVERVIEW;
case 'git_diff_file':
  return { ...MOCK_GIT_DIFF, path: args?.path ?? MOCK_GIT_DIFF.path };
```

Use the exact dispatch shape already present in `tauri-mock.ts`; do not introduce a second mock dispatcher.

- [ ] **Step 4: Verify frontend contract compiles**

Run:

```bash
npm run check
```

Expected: no errors from `ui/lib/tauri/git.ts` or mock additions. `GitPanel.svelte` can still be missing until Task 6.

---

## Task 6: Git Panel And Monaco Diff Modal

**Files:**

- Create: `ui/components/GitPanel.svelte`
- Create: `ui/components/MonacoDiffViewer.svelte`
- Modify: `ui/components/workspace/PaneContainer.svelte` if a temporary Git stub was used.

- [ ] **Step 1: Create `MonacoDiffViewer.svelte`**

Create `ui/components/MonacoDiffViewer.svelte`:

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  export let original: string;
  export let modified: string;
  export let language = 'plaintext';
  export let height = '520px';

  let host: HTMLDivElement;
  let editor: import('monaco-editor').editor.IStandaloneDiffEditor | null = null;
  let monaco: typeof import('monaco-editor') | null = null;

  async function mountEditor() {
    monaco = await import('monaco-editor');
    if (!host || !monaco) return;

    editor = monaco.editor.createDiffEditor(host, {
      automaticLayout: true,
      readOnly: true,
      minimap: { enabled: false },
      renderSideBySide: true,
      scrollBeyondLastLine: false,
    });

    editor.setModel({
      original: monaco.editor.createModel(original, language),
      modified: monaco.editor.createModel(modified, language),
    });
  }

  $: if (editor && monaco) {
    const model = editor.getModel();
    model?.original.dispose();
    model?.modified.dispose();
    editor.setModel({
      original: monaco.editor.createModel(original, language),
      modified: monaco.editor.createModel(modified, language),
    });
  }

  onMount(() => {
    mountEditor();
  });

  onDestroy(() => {
    const model = editor?.getModel();
    model?.original.dispose();
    model?.modified.dispose();
    editor?.dispose();
  });
</script>

<div class="monaco-host" style:height bind:this={host}></div>

<style>
  .monaco-host {
    width: 100%;
    min-height: 0;
  }
</style>
```

If Svelte rejects `style:height`, use `style="height: {height};"`.

- [ ] **Step 2: Create `GitPanel.svelte` script**

Create `ui/components/GitPanel.svelte` with the script:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { GitBranch, RefreshCw, X } from 'lucide-svelte';
  import PanelHeader from './workspace/PanelHeader.svelte';
  import MonacoDiffViewer from './MonacoDiffViewer.svelte';
  import { gitDiffFile, gitOverview, type GitDiffFile, type GitOverview } from '../lib/tauri/git';

  export let cwd: string;
  export let paneId = '';

  let overview: GitOverview | null = null;
  let loading = true;
  let error = '';
  let selectedBranch = '';
  let diff: GitDiffFile | null = null;
  let diffLoading = false;
  let diffError = '';

  $: localBranches = overview?.branches.filter((branch) => branch.kind === 'local') ?? [];
  $: remoteBranches = overview?.branches.filter((branch) => branch.kind === 'remote') ?? [];

  async function loadOverview() {
    loading = true;
    error = '';
    try {
      overview = await gitOverview(cwd);
      selectedBranch = overview.branch ?? '';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function openDiff(path: string, staged: boolean) {
    diffLoading = true;
    diffError = '';
    try {
      diff = await gitDiffFile(cwd, path, staged);
    } catch (e) {
      diffError = e instanceof Error ? e.message : String(e);
    } finally {
      diffLoading = false;
    }
  }

  function closeDiff() {
    diff = null;
    diffError = '';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') closeDiff();
  }

  onMount(() => {
    loadOverview();
  });
</script>
```

- [ ] **Step 3: Add `GitPanel.svelte` markup**

Add markup:

```svelte
<section class="git-panel">
  <PanelHeader
    title="Git"
    subtitle={cwd}
    status={overview?.branch ?? 'repository'}
    dragPayload={JSON.stringify({ sourcePaneId: paneId, target: { kind: 'git', cwd } })}
  >
    <div slot="actions" class="header-actions">
      <button class="icon-button" type="button" aria-label="Refresh Git overview" on:click={loadOverview}>
        <RefreshCw size={14} />
      </button>
    </div>
  </PanelHeader>

  {#if loading}
    <div class="state">Loading Git status...</div>
  {:else if error}
    <div class="state error">{error}</div>
  {:else if overview}
    <div class="git-content">
      <section class="summary-card">
        <div class="branch-title"><GitBranch size={15} /> {overview.branch ?? 'Detached HEAD'}</div>
        <div class="branch-meta">
          <span>{overview.ahead} ahead</span>
          <span>{overview.behind} behind</span>
          {#if overview.upstream}<span>{overview.upstream}</span>{/if}
        </div>
      </section>

      <label class="field-label" for="branch-select">Branches</label>
      <select id="branch-select" bind:value={selectedBranch}>
        <optgroup label="Local branches">
          {#each localBranches as branch}
            <option value={branch.name}>{branch.current ? '* ' : ''}{branch.name}</option>
          {/each}
        </optgroup>
        <optgroup label="Remote branches">
          {#each remoteBranches as branch}
            <option value={branch.name}>{branch.name}</option>
          {/each}
        </optgroup>
      </select>

      <section class="section-block">
        <h3>Changed files</h3>
        {#each overview.files as file}
          <button class="file-row" type="button" on:click={() => openDiff(file.path, file.staged)}>
            <span class="file-status">{file.status}</span>
            <span class="file-path">{file.path}</span>
            {#if file.staged}<span class="pill">staged</span>{/if}
          </button>
        {:else}
          <p class="muted">Working tree clean.</p>
        {/each}
      </section>

      <section class="section-block">
        <h3>Recent commits</h3>
        {#each overview.commits as commit}
          <div class="commit-row">
            <span class="hash">{commit.shortHash}</span>
            <span class="subject">{commit.subject}</span>
            <span class="time">{commit.relativeTime}</span>
          </div>
        {/each}
      </section>

      <section class="section-block">
        <h3>Stashes</h3>
        {#each overview.stashes as stash}
          <div class="stash-row"><span>{stash.name}</span><span>{stash.message}</span></div>
        {:else}
          <p class="muted">No stashes.</p>
        {/each}
      </section>
    </div>
  {/if}
</section>

{#if diff || diffLoading || diffError}
  <div class="diff-backdrop" role="presentation" on:click={closeDiff} on:keydown={handleKeydown} tabindex="-1">
    <div class="diff-modal" role="dialog" aria-modal="true" aria-label="File diff" on:click|stopPropagation>
      <header class="diff-header">
        <span>{diff?.path ?? 'Loading diff...'}</span>
        <button class="icon-button" type="button" aria-label="Close diff" on:click={closeDiff}><X size={15} /></button>
      </header>
      {#if diffLoading}
        <div class="state">Loading diff...</div>
      {:else if diffError}
        <div class="state error">{diffError}</div>
      {:else if diff}
        <MonacoDiffViewer original={diff.original} modified={diff.modified} language={diff.language} height="100%" />
      {/if}
    </div>
  </div>
{/if}
```

- [ ] **Step 4: Add `GitPanel.svelte` styles**

Add styles:

```css
.git-panel {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  background: var(--bg);
}

.git-content {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
  gap: var(--sp-5);
  overflow: auto;
  padding: var(--sp-6);
}

.summary-card,
.section-block {
  border: 1px solid var(--bd);
  border-radius: var(--radius-md);
  background: var(--bg1);
  padding: var(--sp-5);
}

.branch-title,
.branch-meta,
.header-actions,
.commit-row,
.stash-row,
.file-row {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.branch-meta,
.muted,
.time {
  color: var(--t3);
  font-size: var(--xs);
}

.field-label,
  color: var(--t2);
  font-size: var(--xs);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

select {
  border: 1px solid var(--bd);
  border-radius: var(--radius-sm);
  background: var(--bg1);
  color: var(--t0);
  padding: var(--sp-3) var(--sp-4);
}

.file-row {
  width: 100%;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--t1);
  padding: var(--sp-3);
  text-align: left;
}

.file-row:hover,
.icon-button:hover {
  background: var(--bg2);
}

.file-path,
.subject {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-status,
.hash,
.pill {
  color: var(--t3);
  font-size: var(--xs);
}

.icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: 1px solid var(--bd);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--t2);
}

.state {
  display: grid;
  flex: 1;
  place-items: center;
  color: var(--t3);
}

.state.error {
  color: var(--s-error);
}

.diff-backdrop {
  position: fixed;
  inset: 0;
  z-index: 2000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.58);
}

.diff-modal {
  display: flex;
  width: min(1400px, calc(100vw - 48px));
  height: calc(100vh - 48px);
  min-height: 0;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--bd1);
  border-radius: var(--radius-lg);
  background: var(--bg);
}

.diff-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--bd);
  padding: var(--sp-4) var(--sp-5);
  color: var(--t1);
}
```

- [ ] **Step 5: Ensure Escape closes the diff**

If `on:keydown` on the backdrop does not fire reliably, add a `window` listener while the modal is open:

```svelte
<svelte:window on:keydown={handleKeydown} />
```

Guard the handler so it only closes when `diff || diffLoading || diffError` is active.

- [ ] **Step 6: Verify Git UI in mock mode**

Run:

```bash
npm run check
npm run build
npm run dev:mock
```

Expected:

- `npm run check` passes with 0 errors.
- `npm run build` passes. Monaco chunk-size warning is acceptable.
- Mock app opens at `http://127.0.0.1:1420/` or the next available Vite port.
- Opening `+ -> Git overview` shows mock Git data.
- Clicking a changed file opens the Monaco diff modal.
- Close button, outside click, and `Esc` close the modal.

---

## Task 7: Rust Git Commands

**Files:**

- Create: `tauri/src/commands/git.rs`
- Modify: `tauri/src/commands/mod.rs`
- Modify: `tauri/src/lib.rs`

- [ ] **Step 1: Add Git response structs**

Create `tauri/src/commands/git.rs` with structs:

```rust
use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchInfo {
    name: String,
    full_name: String,
    kind: String,
    current: bool,
    upstream: Option<String>,
    ahead: u32,
    behind: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileChange {
    path: String,
    status: String,
    staged: bool,
    untracked: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitInfo {
    hash: String,
    short_hash: String,
    subject: String,
    author: String,
    relative_time: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStashInfo {
    index: u32,
    name: String,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitOverview {
    cwd: String,
    branch: Option<String>,
    upstream: Option<String>,
    ahead: u32,
    behind: u32,
    files: Vec<GitFileChange>,
    branches: Vec<GitBranchInfo>,
    commits: Vec<GitCommitInfo>,
    stashes: Vec<GitStashInfo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiffFile {
    path: String,
    language: String,
    original: String,
    modified: String,
}
```

- [ ] **Step 2: Add a safe Git runner helper**

Add:

```rust
fn run_git(cwd: &str, args: &[&str]) -> Result<String, String> {
    if !Path::new(cwd).exists() {
        return Err(format!("Directory does not exist: {cwd}"));
    }

    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("failed to run git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "git command failed".to_string()
        } else {
            stderr
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

- [ ] **Step 3: Parse current branch, ahead, and behind**

Add:

```rust
fn parse_ahead_behind(line: &str) -> (u32, u32) {
    let mut ahead = 0;
    let mut behind = 0;

    let Some(start) = line.find('[') else {
        return (ahead, behind);
    };
    let Some(end) = line[start..].find(']') else {
        return (ahead, behind);
    };

    let bracket = &line[start + 1..start + end];
    for part in bracket.split(',').map(str::trim) {
        if let Some(value) = part.strip_prefix("ahead ") {
            ahead = value.parse().unwrap_or(0);
        } else if let Some(value) = part.strip_prefix("behind ") {
            behind = value.parse().unwrap_or(0);
        }
    }

    (ahead, behind)
}

fn current_branch(cwd: &str) -> Result<(Option<String>, Option<String>, u32, u32), String> {
    let branch = run_git(cwd, &["branch", "--show-current"])?;
    let branch = branch.trim();
    let branch = if branch.is_empty() { None } else { Some(branch.to_string()) };

    let upstream = run_git(cwd, &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let status = run_git(cwd, &["status", "--short", "--branch"])?;
    let first_line = status.lines().next().unwrap_or_default();
    let (ahead, behind) = parse_ahead_behind(first_line);

    Ok((branch, upstream, ahead, behind))
}
```

- [ ] **Step 4: Parse changed files**

Add:

```rust
fn changed_files(cwd: &str) -> Result<Vec<GitFileChange>, String> {
    let output = run_git(cwd, &["status", "--porcelain=v1"])?;
    Ok(output
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }

            let staged_code = line.chars().next().unwrap_or(' ');
            let worktree_code = line.chars().nth(1).unwrap_or(' ');
            let path = line[3..].to_string();
            let untracked = staged_code == '?' && worktree_code == '?';
            let staged = staged_code != ' ' && staged_code != '?';
            let code = if staged { staged_code } else { worktree_code };
            let status = match code {
                'A' => "added",
                'D' => "deleted",
                'R' => "renamed",
                'C' => "copied",
                '?' => "untracked",
                _ => "modified",
            };

            Some(GitFileChange {
                path,
                status: status.to_string(),
                staged,
                untracked,
            })
        })
        .collect())
}
```

- [ ] **Step 5: Parse branches using `for-each-ref`**

Add:

```rust
fn branches(cwd: &str) -> Result<Vec<GitBranchInfo>, String> {
    let output = run_git(
        cwd,
        &[
            "for-each-ref",
            "--format=%(refname)%00%(refname:short)%00%(HEAD)%00%(upstream:short)%00%(ahead-behind:HEAD)",
            "refs/heads",
            "refs/remotes",
        ],
    )?;

    Ok(output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\0').collect();
            if parts.len() < 5 {
                return None;
            }

            let full_name = parts[0].to_string();
            let name = parts[1].to_string();
            let kind = if full_name.starts_with("refs/remotes/") {
                "remote"
            } else {
                "local"
            };
            let upstream = if parts[3].is_empty() { None } else { Some(parts[3].to_string()) };
            let mut counts = parts[4].split_whitespace();
            let ahead = counts.next().and_then(|value| value.parse().ok()).unwrap_or(0);
            let behind = counts.next().and_then(|value| value.parse().ok()).unwrap_or(0);

            Some(GitBranchInfo {
                name,
                full_name,
                kind: kind.to_string(),
                current: parts[2] == "*",
                upstream,
                ahead,
                behind,
            })
        })
        .collect())
}
```

- [ ] **Step 6: Parse commits and stashes**

Add:

```rust
fn commits(cwd: &str) -> Result<Vec<GitCommitInfo>, String> {
    let output = run_git(cwd, &["log", "-n", "20", "--format=%H%x00%h%x00%s%x00%an%x00%cr"])?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\0').collect();
            if parts.len() < 5 {
                return None;
            }
            Some(GitCommitInfo {
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                subject: parts[2].to_string(),
                author: parts[3].to_string(),
                relative_time: parts[4].to_string(),
            })
        })
        .collect())
}

fn stashes(cwd: &str) -> Result<Vec<GitStashInfo>, String> {
    let output = run_git(cwd, &["stash", "list", "--format=%gd%x00%gs"])?;
    Ok(output
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let parts: Vec<&str> = line.split('\0').collect();
            if parts.len() < 2 {
                return None;
            }
            Some(GitStashInfo {
                index: index as u32,
                name: parts[0].to_string(),
                message: parts[1].to_string(),
            })
        })
        .collect())
}
```

- [ ] **Step 7: Implement `git_overview` command**

Add:

```rust
#[tauri::command]
pub fn git_overview(cwd: String) -> Result<GitOverview, String> {
    run_git(&cwd, &["rev-parse", "--is-inside-work-tree"])?;
    let (branch, upstream, ahead, behind) = current_branch(&cwd)?;

    Ok(GitOverview {
        cwd,
        branch,
        upstream,
        ahead,
        behind,
        files: changed_files(&cwd)?,
        branches: branches(&cwd)?,
        commits: commits(&cwd)?,
        stashes: stashes(&cwd)?,
    })
}
```

During implementation, avoid moving `cwd` before later borrows. If Rust reports a move/borrow error, clone `cwd` for the struct field.

- [ ] **Step 8: Implement `git_diff_file` command**

Add helpers:

```rust
fn read_git_object(cwd: &str, spec: &str) -> String {
    run_git(cwd, &["show", spec]).unwrap_or_default()
}

fn read_worktree_file(cwd: &str, path: &str) -> String {
    std::fs::read_to_string(Path::new(cwd).join(path)).unwrap_or_default()
}

fn language_for(path: &str) -> String {
    match Path::new(path).extension().and_then(|ext| ext.to_str()).unwrap_or_default() {
        "svelte" => "svelte",
        "ts" => "typescript",
        "js" => "javascript",
        "rs" => "rust",
        "json" => "json",
        "md" => "markdown",
        "css" => "css",
        _ => "plaintext",
    }
    .to_string()
}
```

Add command:

```rust
#[tauri::command]
pub fn git_diff_file(cwd: String, path: String, staged: bool) -> Result<GitDiffFile, String> {
    run_git(&cwd, &["rev-parse", "--is-inside-work-tree"])?;

    let status = run_git(&cwd, &["status", "--porcelain=v1", "--", &path])?;
    let untracked = status.starts_with("??");
    let deleted = status.chars().nth(1) == Some('D') || status.chars().next() == Some('D');

    let (original, modified) = if untracked {
        (String::new(), read_worktree_file(&cwd, &path))
    } else if deleted {
        (read_git_object(&cwd, &format!("HEAD:{path}")), String::new())
    } else if staged {
        (
            read_git_object(&cwd, &format!("HEAD:{path}")),
            read_git_object(&cwd, &format!(":{path}")),
        )
    } else {
        (
            read_git_object(&cwd, &format!(":{path}")),
            read_worktree_file(&cwd, &path),
        )
    };

    Ok(GitDiffFile {
        language: language_for(&path),
        path,
        original,
        modified,
    })
}
```

This implements the required pairs:

- staged: `HEAD:path` vs index
- unstaged: index vs working tree
- untracked: empty vs working tree
- deleted: base vs empty

- [ ] **Step 9: Register the Git commands**

In `tauri/src/commands/mod.rs`, add:

```rust
pub mod git;
```

In `tauri/src/lib.rs`, add the commands inside the existing `tauri::generate_handler![...]` list:

```rust
commands::git::git_overview,
commands::git::git_diff_file,
```

- [ ] **Step 10: Verify Rust Git commands**

Run:

```bash
cargo fmt --manifest-path tauri/Cargo.toml
cargo check --manifest-path tauri/Cargo.toml
cargo clippy --manifest-path tauri/Cargo.toml -- -D warnings
```

Expected: all pass. If Clippy flags `unwrap_or_default()` on command output as hiding errors, keep it only in best-effort diff fallback helpers and propagate errors elsewhere.

---

## Task 8: Dependencies, Changelog, And Final Verification

**Files:**

- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Install dependencies if missing**

Check `package.json`. If any required package is missing, run:

```bash
npm install monaco-editor @xterm/xterm @xterm/addon-fit @xterm/addon-unicode11 @xterm/addon-webgl lucide-svelte
```

Expected: `package.json` and `package-lock.json` update. In the current checkout, xterm packages and `lucide-svelte` are already present; `monaco-editor` may still need to be added.

- [ ] **Step 2: Add user-facing changelog entry**

Add an entry under `## May 2026` in `CHANGELOG.md`:

```markdown
### 05/04 · New — Workspace tabs, terminal panels, and Git overview
Workspace panels now support tabbed agent, terminal, and Git views with a cleaner tab bar and draggable panel headers. The terminal uses a real xterm-style interface, and Git changes can be reviewed in a large diff modal with local and remote branches grouped separately.
```

If `## May 2026` does not exist, create it near the top of the changelog.

- [ ] **Step 3: Run full frontend checks**

Run:

```bash
npm run check
npm run build
npm run lint:ui
npm run format:check:ui
```

Expected:

- `npm run check` passes with 0 errors and 0 warnings.
- `npm run build` passes. Monaco chunk-size warning is acceptable and should be noted in the final report.
- `npm run lint:ui` passes.
- `npm run format:check:ui` passes.

- [ ] **Step 4: Run Rust checks**

Run:

```bash
cargo fmt --manifest-path tauri/Cargo.toml --check
cargo check --manifest-path tauri/Cargo.toml
cargo clippy --manifest-path tauri/Cargo.toml -- -D warnings
```

Expected: all pass.

- [ ] **Step 5: Run whitespace diff check**

Run:

```bash
git diff --check
```

Expected: no trailing whitespace or conflict marker output.

- [ ] **Step 6: Manual mock verification**

Run:

```bash
npm run dev:mock
```

Expected manual results:

- Server responds at `http://127.0.0.1:1420/` or the next Vite port.
- Tabs show lucide icons and always-visible close buttons.
- Inactive tabs remain legible without hover.
- The `+` menu stays inside the viewport near the right edge.
- Dragging a panel/tab header can move content to another pane or create a split.
- Terminal tab renders xterm shell chrome; PTY can be no-op in mock mode.
- Git overview shows mock status, grouped branches, commits, stashes, and diff modal.
- Diff modal closes by close button, outside click, and `Esc`.

- [ ] **Step 7: Manual Tauri verification**

Run:

```bash
npm run tauri:dev
```

Expected manual results:

- Terminal spawns a real shell in the provided `cwd`.
- Git overview works against a real Git repository.
- `git_diff_file` returns correct content pairs for staged, unstaged, untracked, and deleted files.

If Tauri verification cannot be completed in the current environment, document the blocker in the final report and do not claim real terminal/Git behavior was verified.

- [ ] **Step 8: Commit by block**

Before committing, inspect changes:

```bash
git status --short
git diff --stat
git diff --check
```

Create logical commits only after the relevant verification passes:

```bash
git add ui/lib/stores/workspace.ts ui/components/workspace/TabItem.svelte ui/components/workspace/TabBar.svelte ui/components/workspace/TabAddMenu.svelte ui/components/workspace/PaneContainer.svelte ui/components/workspace/SplitDropZone.svelte ui/components/workspace/PanelHeader.svelte ui/app.css
git commit -m "feat: add tabbed workspace panels"

git add ui/components/TerminalPanel.svelte ui/lib/tauri/terminal.ts package.json package-lock.json CHANGELOG.md
git commit -m "feat: add xterm terminal panels"

git add docs/specs/git-overview.md ui/components/GitPanel.svelte ui/components/MonacoDiffViewer.svelte ui/lib/tauri/git.ts ui/lib/mock/tauri-mock.ts tauri/src/commands/git.rs tauri/src/commands/mod.rs tauri/src/lib.rs package.json package-lock.json CHANGELOG.md
git commit -m "feat: add git overview panel"
```

Adjust `git add` paths to match the actual files changed. Do not use `--no-verify`.

---

## Self-Review Notes

- Spec coverage: workspace tabs and split behavior are covered by Tasks 1-2; shared panel header by Task 3; xterm terminal by Task 4; Git overview, branch grouping, diff modal, mock data, and Rust diff pairs by Tasks 5-7; verification and commit grouping by Task 8.
- Placeholder scan: no open-ended implementation-only steps remain. The only conditional instructions are environment-dependent verification and existing-app wiring for the New/Open session menu actions.
- Type consistency: `TabTarget`, `Tab`, `PaneState`, `GitOverview`, `GitDiffFile`, and Tauri command names are consistent across store, components, mock wrapper, and Rust command plan.
