import { writable, derived } from 'svelte/store';
import { splitLayout } from './layout';
import type { SubagentInfo } from '../types';

export interface TokenUsage {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
}

export interface MiniLogEntry {
  tool: string;
  target: string;
  result: string | null;
  success: boolean | null;
}

export interface Session {
  id: number;
  projectId: number | null;
  name: string | null;
  status: 'initializing' | 'running' | 'waiting' | 'completed' | 'stopped' | 'error';
  permissionMode: string;
  model: string | null;
  pid: number | null;
  cwd: string | null;
  projectName: string | null;
  gitBranch: string | null;
  worktreePath: string | null;
  branchName: string | null;
  tokens: TokenUsage | null;
  contextPercent: number | null;
  contextWindow?: number | null;
  pendingApproval: string | null;
  miniLog: MiniLogEntry[] | null;
  subagents?: SubagentInfo[];
  createdAt: string;
  updatedAt: string;
}

export const sessions = writable<Session[]>([]);
export const selectedSessionId = derived(splitLayout, ($l) => $l.panes[$l.focused] ?? null);

export function getSelectedSession(list: Session[], id: number | null): Session | null {
  if (id === null) return null;
  return list.find((s) => s.id === id) ?? null;
}

export function upsertSession(list: Session[], updated: Session): Session[] {
  const idx = list.findIndex((s) => s.id === updated.id);
  if (idx === -1) return [updated, ...list];
  const next = [...list];
  next[idx] = { ...next[idx], ...updated };
  return next;
}

export function updateSessionState(
  list: Session[],
  sessionId: number,
  patch: Partial<Session>
): Session[] {
  return list.map((s) => (s.id === sessionId ? { ...s, ...patch } : s));
}
