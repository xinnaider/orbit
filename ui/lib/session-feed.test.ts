import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { journal } from './stores/journal';
import { workspace } from './stores/workspace';
import { appendSessionFeedMessage, isSessionOpenInWorkspace } from './session-feed';

describe('session-feed', () => {
  beforeEach(() => {
    journal.set(new Map());
    workspace.set({
      root: { type: 'leaf', paneId: 'p1' },
      panes: {
        p1: {
          tabs: [{ id: 't1', createdAt: 0, target: { kind: 'agent', sessionId: 1 } }],
          activeTabId: 't1',
        },
      },
      focusedPaneId: 'p1',
    });
  });

  it('appends system message to journal', () => {
    appendSessionFeedMessage(1, 'hello');
    const entries = get(journal).get(1) ?? [];
    expect(entries).toHaveLength(1);
    expect(entries[0].entryType).toBe('system');
    expect(entries[0].text).toBe('hello');
  });

  it('marks error feed lines', () => {
    appendSessionFeedMessage(1, 'boom', { error: true });
    const entry = get(journal).get(1)?.[0];
    expect(entry?.feedError).toBe(true);
  });

  it('detects session open in workspace', () => {
    expect(isSessionOpenInWorkspace(1)).toBe(true);
    expect(isSessionOpenInWorkspace(99)).toBe(false);
  });
});
