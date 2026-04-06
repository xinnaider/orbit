import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Session, TokenUsage, MiniLogEntry } from './stores/sessions';
import type { JournalEntry, SlashCommand, TaskItem } from './types';

// ── Session IPC ────────────────────────────────────────────────

export interface CreateSessionOptions {
  projectPath: string;
  prompt: string;
  model?: string;
  permissionMode?: 'ignore' | 'approve';
  sessionName?: string;
}

export async function createSession(opts: CreateSessionOptions): Promise<Session> {
  return await invoke('create_session', {
    projectPath: opts.projectPath,
    prompt: opts.prompt,
    model: opts.model ?? null,
    permissionMode: opts.permissionMode ?? 'ignore',
    sessionName: opts.sessionName ?? null,
  });
}

export async function listSessions(): Promise<Session[]> {
  return await invoke('list_sessions');
}

export async function stopSession(sessionId: number): Promise<void> {
  await invoke('stop_session', { sessionId });
}

export async function sendSessionMessage(sessionId: number, message: string): Promise<void> {
  await invoke('send_session_message', { sessionId, message });
}

export async function getSessionJournal(sessionId: number): Promise<JournalEntry[]> {
  return await invoke('get_session_journal', { sessionId });
}

// ── Project IPC ────────────────────────────────────────────────

export async function createProject(name: string, path: string) {
  return await invoke('create_project', { name, path });
}

export async function listProjects() {
  return await invoke('list_projects');
}

// ── Read-only commands (unchanged) ────────────────────────────

export async function getSubagentJournal(sessionId: string, subagentId: string): Promise<JournalEntry[]> {
  return await invoke('get_subagent_journal', { sessionId, subagentId });
}

export async function getSlashCommands(): Promise<SlashCommand[]> {
  return await invoke('get_slash_commands');
}

export async function listProjectFiles(cwd: string): Promise<string[]> {
  return await invoke('list_project_files', { cwd });
}

export async function getSessionTasks(sessionId: string): Promise<TaskItem[]> {
  return await invoke('get_tasks', { sessionId });
}

// ── Event listeners ────────────────────────────────────────────

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
}

export function onSessionCreated(cb: (session: Session) => void) {
  return listen<Session>('session:created', e => cb(e.payload));
}

export function onSessionOutput(cb: (payload: SessionOutputPayload) => void) {
  return listen<SessionOutputPayload>('session:output', e => cb(e.payload));
}

export function onSessionState(cb: (payload: SessionStatePayload) => void) {
  return listen<SessionStatePayload>('session:state', e => cb(e.payload));
}

export function onSessionStopped(cb: (sessionId: number) => void) {
  return listen<{ sessionId: number }>('session:stopped', e => cb(e.payload.sessionId));
}

export function onSessionRunning(cb: (sessionId: number, pid: number) => void) {
  return listen<{ sessionId: number; pid: number }>('session:running', e =>
    cb(e.payload.sessionId, e.payload.pid)
  );
}

export function onSessionError(cb: (sessionId: number, error: string) => void) {
  return listen<{ sessionId: number; error: string }>('session:error', e =>
    cb(e.payload.sessionId, e.payload.error)
  );
}
