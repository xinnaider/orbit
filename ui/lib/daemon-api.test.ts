import { describe, it, expect, vi } from 'vitest';
import {
  startDaemonRun,
  resumeDaemonRun,
  cancelDaemonRun,
  respondPermission,
  type DaemonRun,
  type FetchLike,
} from './daemon-api';

const run: DaemonRun = {
  id: 'run_1',
  createdAt: '2026-06-08T12:00:00.000Z',
  updatedAt: '2026-06-08T12:00:00.000Z',
  provider: 'claude',
  prompt: 'hi',
  status: 'queued',
  permissionMode: 'normal',
};

function okFetch(payload: unknown, status = 200): FetchLike {
  return vi.fn(async () => ({
    ok: status >= 200 && status < 300,
    status,
    json: async () => payload,
    text: async () => JSON.stringify(payload),
  })) as unknown as FetchLike;
}

describe('daemon-api', () => {
  it('startDaemonRun POSTs to /runs with the body and returns the run', async () => {
    const doFetch = okFetch(run, 201);
    const result = await startDaemonRun('http://d/', { provider: 'claude', prompt: 'hi' }, doFetch);

    expect(result).toEqual(run);
    expect(doFetch).toHaveBeenCalledWith(
      'http://d/runs',
      expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ provider: 'claude', prompt: 'hi' }),
      })
    );
  });

  it('trims a trailing slash from the base url exactly once', async () => {
    const doFetch = okFetch(run, 201);
    await startDaemonRun('http://d', { provider: 'codex', prompt: 'x' }, doFetch);
    expect(doFetch).toHaveBeenCalledWith('http://d/runs', expect.anything());
  });

  it('resumeDaemonRun hits /runs/:id/resume with the prompt', async () => {
    const doFetch = okFetch(run);
    await resumeDaemonRun('http://d', 'run_1', { prompt: 'more' }, doFetch);
    expect(doFetch).toHaveBeenCalledWith(
      'http://d/runs/run_1/resume',
      expect.objectContaining({ body: JSON.stringify({ prompt: 'more' }) })
    );
  });

  it('cancelDaemonRun POSTs /runs/:id/cancel with no body', async () => {
    const doFetch = okFetch(run);
    await cancelDaemonRun('http://d', 'run_1', doFetch);
    const call = (doFetch as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(call[0]).toBe('http://d/runs/run_1/cancel');
    expect(call[1]).not.toHaveProperty('body');
  });

  it('respondPermission POSTs the decision to the permission endpoint', async () => {
    const doFetch = okFetch({ ok: true });
    const res = await respondPermission(
      'http://d',
      'run_1',
      'perm_9',
      { decision: 'allow', scope: 'once' },
      doFetch
    );
    expect(res).toEqual({ ok: true });
    expect(doFetch).toHaveBeenCalledWith(
      'http://d/runs/run_1/permissions/perm_9',
      expect.objectContaining({ body: JSON.stringify({ decision: 'allow', scope: 'once' }) })
    );
  });

  it('throws with status and body on a non-ok response', async () => {
    const doFetch = okFetch({ error: 'bad' }, 400);
    await expect(
      startDaemonRun('http://d', { provider: 'claude', prompt: 'hi' }, doFetch)
    ).rejects.toThrow(/400/);
  });
});
