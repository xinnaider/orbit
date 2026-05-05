import { writable, get } from 'svelte/store';

// ── Types ──────────────────────────────────────────────────────────────

export type SplitNode =
  | { type: 'leaf'; paneId: string }
  | {
      type: 'split';
      direction: 'horizontal' | 'vertical';
      ratio: number;
      children: [SplitNode, SplitNode];
    };

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

export interface WorkspaceState {
  root: SplitNode;
  panes: Record<string, PaneState>;
  focusedPaneId: string | null;
}

// ── Helpers ────────────────────────────────────────────────────────────

function newPaneId(): string {
  if (crypto.randomUUID) return crypto.randomUUID().slice(0, 8);
  return Array.from(crypto.getRandomValues(new Uint8Array(4)), (b) =>
    b.toString(16).padStart(2, '0')
  ).join('');
}

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

function defaultState(): WorkspaceState {
  const paneId = newPaneId();
  return {
    root: { type: 'leaf', paneId },
    panes: { [paneId]: createPaneState() },
    focusedPaneId: paneId,
  };
}

// ── Store ──────────────────────────────────────────────────────────────

export const workspace = writable<WorkspaceState>(defaultState());

// ── Pane actions ───────────────────────────────────────────────────────

/** Assign a session to a pane. If session is already open in another pane, move it here. */
export function assignSession(paneId: string, sessionId: number): void {
  addTab(paneId, { kind: 'agent', sessionId });
}

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
    if (!pane || fromIndex < 0 || fromIndex >= pane.tabs.length) return ws;
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

/** Clear all tabs from a pane. */
export function clearPane(paneId: string): void {
  workspace.update((ws) => {
    if (!ws.panes[paneId]) return ws;
    ws.panes[paneId] = createPaneState();
    compact(ws);
    return ws;
  });
}

export function focusPane(paneId: string): void {
  workspace.update((ws) => ({ ...ws, focusedPaneId: paneId }));
}

// ── Split actions ──────────────────────────────────────────────────────

/** Split a pane in a direction, putting a session in the new pane. */
export function splitPane(
  paneId: string,
  direction: 'horizontal' | 'vertical',
  sessionIdOrTab: number | Tab | null,
  insertBefore = false
): void {
  workspace.update((ws) => {
    const newPId = newPaneId();
    const tab =
      typeof sessionIdOrTab === 'number'
        ? createTab({ kind: 'agent', sessionId: sessionIdOrTab })
        : sessionIdOrTab;
    ws.panes[newPId] = createPaneState(tab ?? undefined);

    const children: [SplitNode, SplitNode] = insertBefore
      ? [
          { type: 'leaf', paneId: newPId },
          { type: 'leaf', paneId },
        ]
      : [
          { type: 'leaf', paneId },
          { type: 'leaf', paneId: newPId },
        ];

    ws.root = replaceLeaf(ws.root, paneId, {
      type: 'split',
      direction,
      ratio: 0.5,
      children,
    });
    ws.focusedPaneId = newPId;
    compact(ws);
    return ws;
  });
}

/** Update the ratio of a split node at a given path in the tree. */
export function resizeSplit(path: number[], ratio: number): void {
  const clamped = Math.min(0.85, Math.max(0.15, ratio));
  workspace.update((ws) => {
    ws.root = setRatioAtPath(ws.root, path, clamped);
    return ws;
  });
}

/** Close a pane and collapse its parent split. */
export function closePane(paneId: string): void {
  workspace.update((ws) => {
    // Don't close the last pane
    if (ws.root.type === 'leaf') return ws;

    delete ws.panes[paneId];
    ws.root = removeLeaf(ws.root, paneId);

    if (ws.focusedPaneId === paneId) {
      ws.focusedPaneId = Object.keys(ws.panes)[0] ?? null;
    }
    return ws;
  });
}

/** Move a session from one pane to another (for drag between panes). */
export function moveSession(fromPaneId: string, toPaneId: string): void {
  workspace.update((ws) => {
    const from = ws.panes[fromPaneId];
    const to = ws.panes[toPaneId];
    if (!from || !to || fromPaneId === toPaneId) return ws;

    const activeTabId = from.activeTabId;
    const index = from.tabs.findIndex(
      (tab) => tab.id === activeTabId && tab.target.kind === 'agent'
    );
    if (index === -1) return ws;

    const [tab] = from.tabs.splice(index, 1);
    to.tabs.push(tab);
    to.activeTabId = tab.id;
    ws.focusedPaneId = toPaneId;

    if (from.activeTabId === activeTabId) {
      from.activeTabId = from.tabs[Math.max(0, index - 1)]?.id ?? from.tabs[0]?.id ?? null;
    }

    compact(ws);
    return ws;
  });
}

// ── Tree helpers ───────────────────────────────────────────────────────

function collectLeafIds(node: SplitNode): Set<string> {
  if (node.type === 'leaf') return new Set([node.paneId]);
  const ids = new Set<string>();
  for (const child of node.children) {
    for (const id of collectLeafIds(child)) ids.add(id);
  }
  return ids;
}

function compactTree(node: SplitNode, panes: Record<string, PaneState>): SplitNode {
  if (node.type === 'leaf') return node;
  const left = compactTree(node.children[0], panes);
  const right = compactTree(node.children[1], panes);
  const leftEmpty = left.type === 'leaf' && (panes[left.paneId]?.tabs.length ?? 0) === 0;
  const rightEmpty = right.type === 'leaf' && (panes[right.paneId]?.tabs.length ?? 0) === 0;
  if (leftEmpty && rightEmpty) return left;
  if (leftEmpty) return right;
  if (rightEmpty) return left;
  if (left !== node.children[0] || right !== node.children[1]) {
    return { ...node, children: [left, right] };
  }
  return node;
}

function compact(ws: WorkspaceState): void {
  ws.root = compactTree(ws.root, ws.panes);
  const inTree = collectLeafIds(ws.root);
  for (const pid of Object.keys(ws.panes)) {
    if (!inTree.has(pid)) delete ws.panes[pid];
  }
  if (Object.keys(ws.panes).length === 0) {
    const paneId = newPaneId();
    ws.panes[paneId] = createPaneState();
    ws.root = { type: 'leaf', paneId };
    ws.focusedPaneId = paneId;
  }
  if (!ws.panes[ws.focusedPaneId ?? '']) {
    const ids = Object.keys(ws.panes);
    ws.focusedPaneId = ids.length > 0 ? ids[0] : null;
  }
}

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

// ── Session lookup ─────────────────────────────────────────────────────

/** Remove a deleted session from all panes that hold it. */
export function clearSession(sessionId: number): void {
  workspace.update((ws) => {
    for (const [, pane] of Object.entries(ws.panes)) {
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

/** Find which paneId holds a given session, or null. */
export function findPaneWithSession(sessionId: number): string | null {
  const ws = get(workspace);
  return findTabByTarget(ws, { kind: 'agent', sessionId })?.paneId ?? null;
}

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

// ── Persistence ────────────────────────────────────────────────────────

const STORAGE_KEY = 'orbit:workspace';
let saveTimer: ReturnType<typeof setTimeout>;

function saveWorkspace(): void {
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

    for (const pane of Object.values(state.panes)) {
      if (!Array.isArray(pane.tabs)) {
        pane.tabs = [];
        pane.activeTabId = null;
      }
      pane.tabs = pane.tabs.filter(
        (tab) => tab.target.kind !== 'agent' || validSessionIds.has(tab.target.sessionId)
      );
      if (pane.activeTabId && !pane.tabs.some((tab) => tab.id === pane.activeTabId)) {
        pane.activeTabId = pane.tabs[0]?.id ?? null;
      }
    }

    if (Object.keys(state.panes).length === 0) return;
    workspace.set(state);
  } catch {
    // Corrupted — use default
  }
}

// Auto-save on every change
workspace.subscribe(() => saveWorkspace());
