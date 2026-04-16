import { writable } from 'svelte/store';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

export function newPaneId(): string {
  return crypto.randomUUID().slice(0, 8);
}

export function newTabId(): string {
  return crypto.randomUUID();
}

export function createEmptyPane(): PaneState {
  return { tabs: [], activeTabId: null };
}

export function defaultState(): WorkspaceState {
  const paneId = newPaneId();
  return {
    root: { type: 'leaf', paneId },
    panes: { [paneId]: createEmptyPane() },
    focusedPaneId: paneId,
  };
}

export function tabTargetsEqual(a: TabTarget, b: TabTarget): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === 'agent' && b.kind === 'agent') return a.sessionId === b.sessionId;
  if (a.kind === 'terminal' && b.kind === 'terminal') return a.terminalId === b.terminalId;
  return false;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const workspace = writable<WorkspaceState>(defaultState());

// ---------------------------------------------------------------------------
// Tree helpers (internal)
// ---------------------------------------------------------------------------

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
  if (node.type === 'leaf') return node;
  if (path.length === 0) {
    return { ...node, ratio };
  }
  const [head, ...rest] = path;
  const children: [SplitNode, SplitNode] = [
    head === 0 ? setRatioAtPath(node.children[0], rest, ratio) : node.children[0],
    head === 1 ? setRatioAtPath(node.children[1], rest, ratio) : node.children[1],
  ];
  return { ...node, children };
}

/**
 * Removes a leaf from the tree and returns the sibling node that should
 * replace the parent split, or null if the leaf was not found.
 */
function removeLeaf(node: SplitNode, paneId: string): SplitNode | null {
  if (node.type === 'leaf') {
    // Return null to signal "this node should be removed"
    return node.paneId === paneId ? null : node;
  }

  const left = removeLeaf(node.children[0], paneId);
  const right = removeLeaf(node.children[1], paneId);

  if (left === null) {
    // Left child was the removed leaf — promote right child
    return right;
  }
  if (right === null) {
    // Right child was the removed leaf — promote left child
    return left;
  }

  // Neither child was removed; reconstruct with potentially updated subtrees
  return { ...node, children: [left, right] };
}

function collapseEmptyPane(ws: WorkspaceState, paneId: string): WorkspaceState {
  const newRoot = removeLeaf(ws.root, paneId);
  if (newRoot === null) {
    // Removed the only pane — reset to default (shouldn't normally happen)
    return defaultState();
  }

  const newPanes = { ...ws.panes };
  delete newPanes[paneId];

  const focusedPaneId =
    ws.focusedPaneId === paneId
      ? (Object.keys(newPanes)[0] ?? null)
      : ws.focusedPaneId;

  return { root: newRoot, panes: newPanes, focusedPaneId };
}

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

const STORAGE_KEY = 'orbit:workspace';

let saveTimer: ReturnType<typeof setTimeout> | null = null;

export function saveWorkspace(): void {
  if (saveTimer !== null) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    try {
      let snapshot: WorkspaceState | undefined;
      workspace.subscribe((ws) => {
        snapshot = ws;
      })(); // immediately unsubscribe — just grab current value
      if (snapshot !== undefined) {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(snapshot));
      }
    } catch {
      // localStorage may be unavailable (SSR, private browsing quota exceeded)
    }
    saveTimer = null;
  }, 500);
}

export function pruneTree(node: SplitNode, panes: Record<string, PaneState>): SplitNode | null {
  if (node.type === 'leaf') {
    return node.paneId in panes ? node : null;
  }

  const left = pruneTree(node.children[0], panes);
  const right = pruneTree(node.children[1], panes);

  if (left === null && right === null) return null;
  if (left === null) return right;
  if (right === null) return left;

  return { ...node, children: [left, right] };
}

export function restoreWorkspace(validSessionIds: Set<number>): void {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return;

    const parsed = JSON.parse(raw) as WorkspaceState;

    // Validate basic shape
    if (!parsed.root || !parsed.panes || typeof parsed.panes !== 'object') return;

    // Filter panes: remove terminal tabs, validate agent tabs
    const panes: Record<string, PaneState> = {};
    for (const [paneId, pane] of Object.entries(parsed.panes)) {
      const validTabs = pane.tabs.filter((tab) => {
        if (tab.target.kind === 'terminal') return false; // always remove on restore
        if (tab.target.kind === 'agent') return validSessionIds.has(tab.target.sessionId);
        return false;
      });

      if (validTabs.length === 0) continue; // skip panes with no valid tabs

      const activeTabId =
        pane.activeTabId !== null && validTabs.some((t) => t.id === pane.activeTabId)
          ? pane.activeTabId
          : (validTabs[0]?.id ?? null);

      panes[paneId] = { tabs: validTabs, activeTabId };
    }

    // Prune dead branches from tree
    const newRoot = pruneTree(parsed.root, panes);
    if (newRoot === null) return; // nothing survived — keep default

    const focusedPaneId =
      parsed.focusedPaneId !== null && parsed.focusedPaneId in panes
        ? parsed.focusedPaneId
        : (Object.keys(panes)[0] ?? null);

    workspace.set({ root: newRoot, panes, focusedPaneId });
  } catch {
    // Malformed JSON or other error — keep default state
  }
}

// Auto-save on every change (debounced 500ms inside saveWorkspace)
workspace.subscribe(() => saveWorkspace());

// ---------------------------------------------------------------------------
// Tab management actions
// ---------------------------------------------------------------------------

export function addTab(paneId: string, target: TabTarget): void {
  workspace.update((ws) => {
    // Check if any pane already has this target open
    for (const [existingPaneId, pane] of Object.entries(ws.panes)) {
      const existing = pane.tabs.find((t) => tabTargetsEqual(t.target, target));
      if (existing) {
        // Focus the existing tab in its pane
        return {
          ...ws,
          panes: {
            ...ws.panes,
            [existingPaneId]: { ...pane, activeTabId: existing.id },
          },
          focusedPaneId: existingPaneId,
        };
      }
    }

    const pane = ws.panes[paneId];
    if (!pane) return ws;

    const tab: Tab = { id: newTabId(), target, createdAt: Date.now() };
    const newTabs = [...pane.tabs, tab];

    return {
      ...ws,
      panes: {
        ...ws.panes,
        [paneId]: { tabs: newTabs, activeTabId: tab.id },
      },
      focusedPaneId: paneId,
    };
  });
}

export function closeTab(paneId: string, tabId: string): void {
  workspace.update((ws) => {
    const pane = ws.panes[paneId];
    if (!pane) return ws;

    const idx = pane.tabs.findIndex((t) => t.id === tabId);
    if (idx === -1) return ws;

    const newTabs = pane.tabs.filter((t) => t.id !== tabId);

    if (newTabs.length === 0) {
      // Pane is now empty — collapse it (unless it's the last pane)
      const paneCount = Object.keys(ws.panes).length;
      if (paneCount <= 1) {
        // Keep the pane but clear it
        return {
          ...ws,
          panes: { ...ws.panes, [paneId]: { tabs: [], activeTabId: null } },
        };
      }
      return collapseEmptyPane(ws, paneId);
    }

    // Activate previous tab (or next if closing first tab)
    const newActiveIdx = Math.max(0, idx - 1);
    const activeTabId = newTabs[newActiveIdx]?.id ?? null;

    return {
      ...ws,
      panes: { ...ws.panes, [paneId]: { tabs: newTabs, activeTabId } },
    };
  });
}

export function setActiveTab(paneId: string, tabId: string): void {
  workspace.update((ws) => {
    const pane = ws.panes[paneId];
    if (!pane) return ws;
    return {
      ...ws,
      panes: { ...ws.panes, [paneId]: { ...pane, activeTabId: tabId } },
      focusedPaneId: paneId,
    };
  });
}

export function focusPane(paneId: string): void {
  workspace.update((ws) => ({ ...ws, focusedPaneId: paneId }));
}

// ---------------------------------------------------------------------------
// Drag/drop and split actions
// ---------------------------------------------------------------------------

export function moveTab(fromPaneId: string, toPaneId: string, tabId: string): void {
  workspace.update((ws) => {
    if (fromPaneId === toPaneId) return ws;

    const fromPane = ws.panes[fromPaneId];
    const toPane = ws.panes[toPaneId];
    if (!fromPane || !toPane) return ws;

    const tab = fromPane.tabs.find((t) => t.id === tabId);
    if (!tab) return ws;

    const fromTabs = fromPane.tabs.filter((t) => t.id !== tabId);
    const toTabs = [...toPane.tabs, tab];

    let newWs: WorkspaceState = {
      ...ws,
      panes: {
        ...ws.panes,
        [fromPaneId]: {
          tabs: fromTabs,
          activeTabId:
            fromPane.activeTabId === tabId
              ? (fromTabs[Math.max(0, fromTabs.length - 1)]?.id ?? null)
              : fromPane.activeTabId,
        },
        [toPaneId]: { tabs: toTabs, activeTabId: tab.id },
      },
      focusedPaneId: toPaneId,
    };

    // Collapse source pane if it became empty (and there are other panes)
    if (fromTabs.length === 0 && Object.keys(newWs.panes).length > 1) {
      newWs = collapseEmptyPane(newWs, fromPaneId);
    }

    return newWs;
  });
}

export function reorderTab(paneId: string, fromIndex: number, toIndex: number): void {
  workspace.update((ws) => {
    const pane = ws.panes[paneId];
    if (!pane) return ws;
    if (fromIndex === toIndex) return ws;

    const tabs = [...pane.tabs];
    const [moved] = tabs.splice(fromIndex, 1);
    tabs.splice(toIndex, 0, moved);

    return {
      ...ws,
      panes: { ...ws.panes, [paneId]: { ...pane, tabs } },
    };
  });
}

export function splitPane(
  paneId: string,
  direction: 'horizontal' | 'vertical',
  target: TabTarget
): void {
  workspace.update((ws) => {
    const sourcePaneExists = paneId in ws.panes;
    if (!sourcePaneExists) return ws;

    const newPaneIdValue = newPaneId();
    const newTab: Tab = { id: newTabId(), target, createdAt: Date.now() };
    const newPane: PaneState = { tabs: [newTab], activeTabId: newTab.id };

    const splitNode: SplitNode = {
      type: 'split',
      direction,
      ratio: 0.5,
      children: [{ type: 'leaf', paneId }, { type: 'leaf', paneId: newPaneIdValue }],
    };

    const newRoot = replaceLeaf(ws.root, paneId, splitNode);

    return {
      root: newRoot,
      panes: { ...ws.panes, [newPaneIdValue]: newPane },
      focusedPaneId: newPaneIdValue,
    };
  });
}

export function resizeSplit(path: number[], ratio: number): void {
  const clamped = Math.min(0.85, Math.max(0.15, ratio));
  workspace.update((ws) => ({
    ...ws,
    root: setRatioAtPath(ws.root, path, clamped),
  }));
}
