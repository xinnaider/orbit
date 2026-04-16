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

export interface PaneState {
  sessionId: number | null;
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

function defaultState(): WorkspaceState {
  const paneId = newPaneId();
  return {
    root: { type: 'leaf', paneId },
    panes: { [paneId]: { sessionId: null } },
    focusedPaneId: paneId,
  };
}

// ── Store ──────────────────────────────────────────────────────────────

export const workspace = writable<WorkspaceState>(defaultState());

// ── Pane actions ───────────────────────────────────────────────────────

/** Assign a session to a pane. If session is already open in another pane, focus that one. */
export function assignSession(paneId: string, sessionId: number): void {
  workspace.update((ws) => {
    // Check if already open somewhere
    for (const [pid, pane] of Object.entries(ws.panes)) {
      if (pane.sessionId === sessionId) {
        return { ...ws, focusedPaneId: pid };
      }
    }
    if (!ws.panes[paneId]) return ws;
    ws.panes[paneId] = { sessionId };
    ws.focusedPaneId = paneId;
    return ws;
  });
}

/** Clear a session from a pane (set to null). */
export function clearPane(paneId: string): void {
  workspace.update((ws) => {
    if (!ws.panes[paneId]) return ws;
    ws.panes[paneId] = { sessionId: null };
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
  sessionId: number | null
): void {
  workspace.update((ws) => {
    if (sessionId !== null) {
      const existing = Object.entries(ws.panes).find(
        ([pid, p]) => pid !== paneId && p.sessionId === sessionId
      );
      if (existing) {
        ws.focusedPaneId = existing[0];
        return ws;
      }
    }

    const newPId = newPaneId();
    ws.panes[newPId] = { sessionId };
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
    if (!from || !to || !from.sessionId) return ws;
    if (fromPaneId === toPaneId) return ws;

    const existingHolder = Object.entries(ws.panes).find(
      ([pid, p]) => pid !== fromPaneId && pid !== toPaneId && p.sessionId === from.sessionId
    );
    if (existingHolder) {
      ws.panes[fromPaneId] = { sessionId: null };
      ws.focusedPaneId = existingHolder[0];
      return ws;
    }

    ws.panes[toPaneId] = { sessionId: from.sessionId };
    ws.panes[fromPaneId] = { sessionId: null };
    ws.focusedPaneId = toPaneId;
    return ws;
  });
}

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
      if (pane.sessionId === sessionId) {
        pane.sessionId = null;
      }
    }
    return ws;
  });
}

/** Find which paneId holds a given session, or null. */
export function findPaneWithSession(sessionId: number): string | null {
  const ws = get(workspace);
  for (const [paneId, pane] of Object.entries(ws.panes)) {
    if (pane.sessionId === sessionId) return paneId;
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

    // Validate sessions
    for (const pane of Object.values(state.panes)) {
      if (pane.sessionId !== null && !validSessionIds.has(pane.sessionId)) {
        pane.sessionId = null;
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
