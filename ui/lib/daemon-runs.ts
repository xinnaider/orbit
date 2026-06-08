/**
 * Run registry — correlates the daemon's string `runId` with Orbit's numeric
 * `sessionId`.
 *
 * The daemon owns run ids (it generates them on POST /runs) and Orbit owns
 * session ids. Rather than leaking either side's identifier into the other, this
 * registry is the single place that ties a started run to the session whose feed
 * its events should flow into. The SSE bridge resolves events through it.
 */

const runToSession = new Map<string, number>();
const sessionToRun = new Map<number, string>();

/** Tie a daemon run to an Orbit session. Replaces any prior run for the session. */
export function registerDaemonRun(runId: string, sessionId: number): void {
  const prior = sessionToRun.get(sessionId);
  if (prior && prior !== runId) runToSession.delete(prior);
  runToSession.set(runId, sessionId);
  sessionToRun.set(sessionId, runId);
}

/** Session that should receive a run's events, or null if the run is unknown. */
export function resolveSessionForRun(runId: string): number | null {
  return runToSession.get(runId) ?? null;
}

/** Current run id for a session, or null. Used for resume/cancel by session. */
export function resolveRunForSession(sessionId: number): string | null {
  return sessionToRun.get(sessionId) ?? null;
}

/** Forget a session and its run (on session delete/stop). */
export function clearDaemonSession(sessionId: number): void {
  const runId = sessionToRun.get(sessionId);
  if (runId) runToSession.delete(runId);
  sessionToRun.delete(sessionId);
}

/** Drop all mappings. Test helper / global reset. */
export function resetDaemonRuns(): void {
  runToSession.clear();
  sessionToRun.clear();
}
