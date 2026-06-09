/**
 * Typed client for the daemon REST API.
 *
 * Mirrors the daemon's HTTP routes (adapters/http/routes.ts). `fetch` is
 * injectable so the client is unit-testable without a network or globals.
 */

import type { ProviderId } from './daemon-feed';

export type PermissionMode = 'normal' | 'yolo';

export interface DaemonRun {
  id: string;
  createdAt: string;
  updatedAt: string;
  provider: string;
  prompt: string;
  status: 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';
  permissionMode: PermissionMode;
}

export interface StartRunBody {
  provider: ProviderId;
  prompt: string;
  cwd?: string;
  model?: string;
  permissionMode?: PermissionMode;
  /** Provider-native session id, for resuming an existing agent session. */
  sessionId?: string;
}

export interface PermissionReply {
  decision: 'allow' | 'deny';
  scope: 'once' | 'always' | 'until_reply';
}

export type FetchLike = typeof fetch;

function base(url: string): string {
  return url.replace(/\/$/, '');
}

async function request<T>(
  doFetch: FetchLike,
  url: string,
  method: 'POST' | 'GET',
  body?: unknown
): Promise<T> {
  const res = await doFetch(url, {
    method,
    headers: { 'Content-Type': 'application/json' },
    ...(body !== undefined ? { body: JSON.stringify(body) } : {}),
  });
  if (!res.ok) {
    throw new Error(`daemon ${method} ${url} failed: ${res.status} ${await res.text()}`);
  }
  return res.json() as Promise<T>;
}

/** Start a run. POST /runs → 201 with the created run (including its generated id). */
export function startDaemonRun(
  baseUrl: string,
  body: StartRunBody,
  doFetch: FetchLike = fetch
): Promise<DaemonRun> {
  return request<DaemonRun>(doFetch, `${base(baseUrl)}/runs`, 'POST', body);
}

/** Resume a run with a follow-up prompt. POST /runs/:id/resume. */
export function resumeDaemonRun(
  baseUrl: string,
  runId: string,
  input: { prompt?: string; sessionId?: string },
  doFetch: FetchLike = fetch
): Promise<DaemonRun> {
  return request<DaemonRun>(doFetch, `${base(baseUrl)}/runs/${runId}/resume`, 'POST', input);
}

/** Cancel a run. POST /runs/:id/cancel. */
export function cancelDaemonRun(
  baseUrl: string,
  runId: string,
  doFetch: FetchLike = fetch
): Promise<DaemonRun> {
  return request<DaemonRun>(doFetch, `${base(baseUrl)}/runs/${runId}/cancel`, 'POST');
}

/** Answer a permission prompt. POST /runs/:id/permissions/:permissionId. */
export function respondPermission(
  baseUrl: string,
  runId: string,
  permissionId: string,
  reply: PermissionReply,
  doFetch: FetchLike = fetch
): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>(
    doFetch,
    `${base(baseUrl)}/runs/${runId}/permissions/${permissionId}`,
    'POST',
    reply
  );
}
