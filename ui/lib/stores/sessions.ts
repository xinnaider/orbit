import { writable, derived } from 'svelte/store';
import { workspace } from './workspace';
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
  provider: string;
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
  sshHost: string | null;
  sshUser: string | null;
  subagents?: SubagentInfo[];
  attention?: { requiresAttention: boolean; reason: string | null; since: string | null } | null;
  rateLimit?: import('../types').RateLimitInfo[];
  costUsd?: number;
  skipPermissions?: boolean;
  parentSessionId?: number | null;
  depth?: number;
  createdAt: string;
  updatedAt: string;
}

export const sessions = writable<Session[]>([]);

// Derive selected session ID from workspace focused pane
export const selectedSessionId = derived(workspace, ($ws) => {
  const focusedPane = $ws.focusedPaneId ? $ws.panes[$ws.focusedPaneId] : null;
  const activeTab =
    focusedPane?.tabs.find((tab) => tab.id === focusedPane.activeTabId) ?? focusedPane?.tabs[0];
  return activeTab?.target.kind === 'agent' ? activeTab.target.sessionId : null;
});

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
