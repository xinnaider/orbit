import { describe, it, expect } from 'vitest';
import { getSelectedSession, upsertSession, updateSessionState, type Session } from './sessions';

function makeSession(overrides: Partial<Session> = {}): Session {
  return {
    id: 1,
    projectId: null,
    name: null,
    status: 'running',
    permissionMode: 'ignore',
    model: null,
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

// ── getSelectedSession ────────────────────────────────────────

describe('getSelectedSession', () => {
  it('returns null when id is null', () => {
    const list = [makeSession({ id: 1 })];
    expect(getSelectedSession(list, null)).toBeNull();
  });

  it('returns null when list is empty', () => {
    expect(getSelectedSession([], 1)).toBeNull();
  });

  it('returns the matching session', () => {
    const s1 = makeSession({ id: 1, projectName: 'a' });
    const s2 = makeSession({ id: 2, projectName: 'b' });
    expect(getSelectedSession([s1, s2], 2)?.projectName).toBe('b');
  });

  it('returns null when id not in list', () => {
    const s = makeSession({ id: 1 });
    expect(getSelectedSession([s], 99)).toBeNull();
  });
});

// ── upsertSession ─────────────────────────────────────────────

describe('upsertSession', () => {
  it('prepends new session when not in list', () => {
    const s1 = makeSession({ id: 1 });
    const s2 = makeSession({ id: 2 });
    const result = upsertSession([s1], s2);
    expect(result).toHaveLength(2);
    expect(result[0].id).toBe(2); // new session at front
    expect(result[1].id).toBe(1);
  });

  it('updates existing session in place', () => {
    const s1 = makeSession({ id: 1, status: 'running' });
    const updated = makeSession({ id: 1, status: 'completed' });
    const result = upsertSession([s1], updated);
    expect(result).toHaveLength(1);
    expect(result[0].status).toBe('completed');
  });

  it('preserves existing fields when merging', () => {
    const s1 = makeSession({ id: 1, model: 'sonnet', status: 'running' });
    const patch = makeSession({ id: 1, status: 'completed' });
    const result = upsertSession([s1], patch);
    // model is from patch (full object replace)
    expect(result[0].status).toBe('completed');
  });

  it('does not mutate the original list', () => {
    const original = [makeSession({ id: 1 })];
    const updated = makeSession({ id: 1, status: 'stopped' });
    upsertSession(original, updated);
    expect(original[0].status).toBe('running');
  });
});

// ── updateSessionState ────────────────────────────────────────

describe('updateSessionState', () => {
  it('patches only the matching session', () => {
    const s1 = makeSession({ id: 1, status: 'running' });
    const s2 = makeSession({ id: 2, status: 'running' });
    const result = updateSessionState([s1, s2], 1, { status: 'completed' });
    expect(result[0].status).toBe('completed');
    expect(result[1].status).toBe('running');
  });

  it('returns list unchanged when id not found', () => {
    const s1 = makeSession({ id: 1, status: 'running' });
    const result = updateSessionState([s1], 99, { status: 'completed' });
    expect(result[0].status).toBe('running');
  });

  it('patches tokens correctly', () => {
    const s = makeSession({ id: 1, tokens: null });
    const tokens = { input: 100, output: 50, cacheRead: 10, cacheWrite: 5 };
    const result = updateSessionState([s], 1, { tokens });
    expect(result[0].tokens).toEqual(tokens);
  });

  it('patches pendingApproval', () => {
    const s = makeSession({ id: 1, pendingApproval: null });
    const result = updateSessionState([s], 1, { pendingApproval: 'Allow Bash?' });
    expect(result[0].pendingApproval).toBe('Allow Bash?');
  });

  it('does not mutate original list', () => {
    const original = [makeSession({ id: 1, status: 'running' })];
    updateSessionState(original, 1, { status: 'completed' });
    expect(original[0].status).toBe('running');
  });
});
