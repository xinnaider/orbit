import { get } from 'svelte/store';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { assignSession, restoreWorkspace, workspace } from './workspace';

describe('workspace tabs', () => {
  const storage = new Map<string, string>();

  beforeEach(() => {
    vi.stubGlobal('localStorage', {
      clear: () => storage.clear(),
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => storage.set(key, value),
      removeItem: (key: string) => storage.delete(key),
    });
    localStorage.clear();
    restoreWorkspace(new Set());
  });

  it('opens assigned sessions as active tabs in the focused pane', () => {
    const paneId = get(workspace).focusedPaneId;

    expect(paneId).not.toBeNull();
    assignSession(paneId!, 42);

    const pane = get(workspace).panes[paneId!];
    expect(pane.tabs).toHaveLength(1);
    expect(pane.tabs[0].target).toEqual({ kind: 'agent', sessionId: 42 });
    expect(pane.activeTabId).toBe(pane.tabs[0].id);
  });
});
