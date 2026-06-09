import { describe, it, expect, vi, beforeEach } from 'vitest';
import { openDaemonSession, sendDaemonMessage, cancelDaemonSession } from './daemon-session';
import { resolveSessionForRun, resolveRunForSession, resetDaemonRuns } from './daemon-runs';
import type { DaemonRun, FetchLike } from './daemon-api';

const run: DaemonRun = {
  id: 'run_1',
  createdAt: '2026-06-08T12:00:00.000Z',
  updatedAt: '2026-06-08T12:00:00.000Z',
  provider: 'claude',
  prompt: 'hi',
  status: 'queued',
  permissionMode: 'normal',
};

function fetchReturning(payload: unknown): FetchLike {
  return vi.fn(async () => ({
    ok: true,
    status: 200,
    json: async () => payload,
    text: async () => JSON.stringify(payload),
  })) as unknown as FetchLike;
}

describe('daemon session orchestrators', () => {
  beforeEach(() => resetDaemonRuns());

  it('openDaemonSession starts a run and registers it to the session', async () => {
    const doFetch = fetchReturning(run);
    const result = await openDaemonSession(
      'http://d',
      88,
      { provider: 'claude', prompt: 'hi' },
      doFetch
    );

    expect(result).toEqual(run);
    expect(resolveSessionForRun('run_1')).toBe(88);
    expect(resolveRunForSession(88)).toBe('run_1');
  });

  it('sendDaemonMessage resumes the registered run', async () => {
    await openDaemonSession(
      'http://d',
      5,
      { provider: 'codex', prompt: 'first' },
      fetchReturning(run)
    );

    const doFetch = fetchReturning({ ...run, prompt: 'second' });
    await sendDaemonMessage('http://d', 5, 'second', doFetch);

    expect(doFetch).toHaveBeenCalledWith(
      'http://d/runs/run_1/resume',
      expect.objectContaining({ body: JSON.stringify({ prompt: 'second' }) })
    );
  });

  it('sendDaemonMessage throws when the session has no run', async () => {
    await expect(sendDaemonMessage('http://d', 404, 'x', fetchReturning(run))).rejects.toThrow(
      /No daemon run/
    );
  });

  it('cancelDaemonSession cancels the run and forgets the mapping', async () => {
    await openDaemonSession(
      'http://d',
      9,
      { provider: 'claude', prompt: 'hi' },
      fetchReturning(run)
    );

    const doFetch = fetchReturning({ ...run, status: 'cancelled' });
    const result = await cancelDaemonSession('http://d', 9, doFetch);

    expect(result?.status).toBe('cancelled');
    expect(resolveRunForSession(9)).toBeNull();
    expect(resolveSessionForRun('run_1')).toBeNull();
  });

  it('cancelDaemonSession is a no-op (null) for an unknown session', async () => {
    const doFetch = vi.fn() as unknown as FetchLike;
    const result = await cancelDaemonSession('http://d', 1234, doFetch);
    expect(result).toBeNull();
    expect(doFetch).not.toHaveBeenCalled();
  });
});
