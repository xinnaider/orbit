import { get } from 'svelte/store';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { selectedSessionId, sessions, type Session } from './sessions';
import { workspace } from './workspace';
import { upsertAndOpenSession, upsertSessionFromEvent } from './session-actions';

function makeSession(overrides: Partial<Session> = {}): Session {
  return {
    id: 1,
    projectId: null,
    name: null,
    status: 'running',
    permissionMode: 'ignore',
    model: null,
    provider: 'claude-code',
    pid: null,
    cwd: '/tmp/proj',
    projectName: 'proj',
    gitBranch: null,
    worktreePath: null,
    branchName: null,
    tokens: null,
    contextPercent: null,
    pendingApproval: null,
    miniLog: null,
    sshHost: null,
    sshUser: null,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

function resetWorkspace() {
  workspace.set({
    root: { type: 'leaf', paneId: 'pane-1' },
    panes: { 'pane-1': { tabs: [], activeTabId: null } },
    focusedPaneId: 'pane-1',
  });
}

describe('session open actions', () => {
  const storage = new Map<string, string>();

  beforeEach(() => {
    vi.stubGlobal('localStorage', {
      clear: () => storage.clear(),
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => storage.set(key, value),
      removeItem: (key: string) => storage.delete(key),
    });
    localStorage.clear();
    sessions.set([]);
    resetWorkspace();
  });

  it('opens manually-created sessions even when another session is active', () => {
    upsertAndOpenSession(makeSession({ id: 1, projectName: 'first' }));
    upsertAndOpenSession(makeSession({ id: 2, projectName: 'second' }));

    expect(get(selectedSessionId)).toBe(2);
    expect(get(sessions).map((session) => session.id)).toEqual([2, 1]);
  });

  it('does not steal focus for background session events', () => {
    upsertAndOpenSession(makeSession({ id: 1, projectName: 'active' }));
    upsertSessionFromEvent(makeSession({ id: 2, projectName: 'background' }));

    expect(get(selectedSessionId)).toBe(1);
    expect(get(sessions).map((session) => session.id)).toEqual([2, 1]);
  });

  it('opens background session events when no session is active', () => {
    upsertSessionFromEvent(makeSession({ id: 2, projectName: 'first' }));

    expect(get(selectedSessionId)).toBe(2);
  });
});
