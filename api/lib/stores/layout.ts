import { writable } from 'svelte/store';

export type PaneId = 'tl' | 'tr' | 'bl' | 'br';

export interface SplitLayout {
  panes: Record<PaneId, number | null>;
  visible: PaneId[];
  focused: PaneId;
}

const defaultLayout: SplitLayout = {
  panes: { tl: null, tr: null, bl: null, br: null },
  visible: ['tl'],
  focused: 'tl',
};

export const splitLayout = writable<SplitLayout>(defaultLayout);

/** Put a session in a pane that is already visible; focus it. */
export function assignSession(paneId: PaneId, sessionId: number): void {
  splitLayout.update((l) => ({
    ...l,
    panes: { ...l.panes, [paneId]: sessionId },
    focused: paneId,
  }));
}

/** Open a pane (add to visible if needed), assign a session, focus it. */
export function openPane(paneId: PaneId, sessionId: number): void {
  splitLayout.update((l) => {
    const alreadyVisible = l.visible.includes(paneId);
    return {
      ...l,
      panes: { ...l.panes, [paneId]: sessionId },
      visible: alreadyVisible ? l.visible : [...l.visible, paneId],
      focused: paneId,
    };
  });
}

/** Close a pane. Never closes the last remaining pane. Clears its session.
 *  If all remaining panes are empty after closing, collapses to a single pane. */
export function closePane(paneId: PaneId): void {
  splitLayout.update((l) => {
    const visible = l.visible.filter((p) => p !== paneId);
    if (visible.length === 0) return l; // guard: never close last
    const focused = l.focused === paneId ? visible[0] : l.focused;
    const panes = { ...l.panes, [paneId]: null };
    // If every remaining pane is empty, collapse to a single full-screen pane
    const allEmpty = visible.every((p) => panes[p] === null);
    if (allEmpty) {
      return { ...l, panes, visible: [focused], focused };
    }
    return { ...l, panes, visible, focused };
  });
}

/** Set which pane has keyboard/MetaPanel focus. */
export function focusPane(paneId: PaneId): void {
  splitLayout.update((l) => {
    if (!l.visible.includes(paneId)) return l;
    return { ...l, focused: paneId };
  });
}

/** Open the next available adjacent pane (empty). Focuses the new pane. No-op at 4 panes. */
export function splitPane(fromPaneId: PaneId): void {
  const priority: Record<PaneId, PaneId[]> = {
    tl: ['tr', 'bl'],
    tr: ['br', 'tl'],
    bl: ['br', 'tl'],
    br: ['tr', 'bl'],
  };
  splitLayout.update((l) => {
    if (l.visible.length >= 4) return l;
    const target = priority[fromPaneId].find((p) => !l.visible.includes(p));
    if (!target) return l;
    return { ...l, visible: [...l.visible, target], focused: target };
  });
}

/** Remove a deleted/hidden session from every pane that held it. */
export function clearSession(sessionId: number): void {
  splitLayout.update((l) => ({
    ...l,
    panes: Object.fromEntries(
      Object.entries(l.panes).map(([k, v]) => [k, v === sessionId ? null : v])
    ) as Record<PaneId, number | null>,
  }));
}
