import type { Session, TokenUsage, MiniLogEntry } from '../stores/sessions';
import type { JournalEntry, SubagentInfo } from '../types';
import { listen } from './invoke';

export interface SessionOutputPayload {
  sessionId: number;
  entry: JournalEntry;
}

export interface SessionStatePayload {
  sessionId: number;
  status: string;
  tokens: TokenUsage;
  contextPercent: number;
  pendingApproval: string | null;
  miniLog: MiniLogEntry[];
  gitBranch: string | null;
  subagents: SubagentInfo[];
  model: string | null;
  contextWindow: number | null;
  attention: { requiresAttention: boolean; reason: string | null; since: string | null } | null;
  rateLimit: import('../types').RateLimitInfo[];
  costUsd: number | null;
}

export function onSessionCreated(cb: (session: Session) => void) {
  return listen<Session>('session:created', (e) => cb(e.payload));
}

export function onSessionOutput(cb: (payload: SessionOutputPayload) => void) {
  return listen<SessionOutputPayload>('session:output', (e) => cb(e.payload));
}

export function onSessionState(cb: (payload: SessionStatePayload) => void) {
  return listen<SessionStatePayload>('session:state', (e) => cb(e.payload));
}

export function onSessionStopped(cb: (sessionId: number) => void) {
  return listen<{ sessionId: number }>('session:stopped', (e) => cb(e.payload.sessionId));
}

export function onSessionRunning(cb: (sessionId: number, pid: number) => void) {
  return listen<{ sessionId: number; pid: number }>('session:running', (e) =>
    cb(e.payload.sessionId, e.payload.pid)
  );
}

export function onSessionError(cb: (sessionId: number, error: string) => void) {
  return listen<{ sessionId: number; error: string }>('session:error', (e) =>
    cb(e.payload.sessionId, e.payload.error)
  );
}

export function onSessionRateLimit(cb: (sessionId: number) => void) {
  return listen<{ sessionId: number }>('session:rate-limit', (e) => cb(e.payload.sessionId));
}

export function onSessionTaskUpdate(cb: (sessionId: number) => void) {
  return listen<{ sessionId: number }>('session:task-update', (e) => cb(e.payload.sessionId));
}
