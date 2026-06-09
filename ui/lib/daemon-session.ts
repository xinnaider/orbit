/**
 * Session orchestrators — the glue between an Orbit session and a daemon run.
 *
 * These compose the typed API client (daemon-api.ts) with the run registry
 * (daemon-runs.ts) so callers work in terms of Orbit session ids. The SSE bridge
 * then routes that run's events back into the session's feed via the same
 * registry. Pure orchestration — no UI, no globals beyond the injected fetch.
 */

import {
  startDaemonRun,
  resumeDaemonRun,
  cancelDaemonRun,
  type DaemonRun,
  type StartRunBody,
  type FetchLike,
} from './daemon-api';
import { registerDaemonRun, resolveRunForSession, clearDaemonSession } from './daemon-runs';

/** Start a daemon run for a session and register the mapping for event routing. */
export async function openDaemonSession(
  baseUrl: string,
  sessionId: number,
  body: StartRunBody,
  doFetch: FetchLike = fetch
): Promise<DaemonRun> {
  const run = await startDaemonRun(baseUrl, body, doFetch);
  registerDaemonRun(run.id, sessionId);
  return run;
}

/** Send a follow-up prompt to the session's active daemon run. */
export async function sendDaemonMessage(
  baseUrl: string,
  sessionId: number,
  prompt: string,
  doFetch: FetchLike = fetch
): Promise<DaemonRun> {
  const runId = resolveRunForSession(sessionId);
  if (!runId) throw new Error(`No daemon run registered for session ${sessionId}`);
  return resumeDaemonRun(baseUrl, runId, { prompt }, doFetch);
}

/** Cancel the session's run and forget the mapping. */
export async function cancelDaemonSession(
  baseUrl: string,
  sessionId: number,
  doFetch: FetchLike = fetch
): Promise<DaemonRun | null> {
  const runId = resolveRunForSession(sessionId);
  if (!runId) return null;
  const run = await cancelDaemonRun(baseUrl, runId, doFetch);
  clearDaemonSession(sessionId);
  return run;
}
