import type { Session } from '../stores/sessions';
import type { JournalEntry } from '../types';
import { invoke } from './invoke';

export interface CreateSessionOptions {
  projectPath: string;
  prompt: string;
  model?: string;
  permissionMode?: 'ignore' | 'approve';
  sessionName?: string;
  useWorktree?: boolean;
  provider?: string;
  /** Provider API key. Set before spawn to avoid race condition. */
  apiKey?: string;
  sshHost?: string;
  sshUser?: string;
  /** SSH password. Never persisted — held in backend memory for session lifetime. */
  sshPassword?: string;
}

export async function createSession(opts: CreateSessionOptions): Promise<Session> {
  return await invoke('create_session', {
    projectPath: opts.projectPath,
    prompt: opts.prompt,
    model: opts.model ?? null,
    permissionMode: opts.permissionMode ?? 'ignore',
    sessionName: opts.sessionName ?? null,
    useWorktree: opts.useWorktree ?? false,
    provider: opts.provider ?? 'claude-code',
    apiKey: opts.apiKey ?? null,
    sshHost: opts.sshHost ?? null,
    sshUser: opts.sshUser ?? null,
    sshPassword: opts.sshPassword ?? null,
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

export async function updateSessionModel(sessionId: number, model: string): Promise<void> {
  await invoke('update_session_model', { sessionId, model });
}

export async function updateSessionEffort(sessionId: number, effort: string): Promise<void> {
  await invoke('update_session_effort', { sessionId, effort });
}

export async function setSessionApiKey(sessionId: number, apiKey: string): Promise<void> {
  await invoke('set_session_api_key', { sessionId, apiKey });
}

export async function renameSession(sessionId: number, name: string): Promise<void> {
  await invoke('rename_session', { sessionId, name });
}

export async function deleteSession(sessionId: number): Promise<void> {
  await invoke('delete_session', { sessionId });
}

export async function getSessionJournal(sessionId: number): Promise<JournalEntry[]> {
  return await invoke('get_session_journal', { sessionId });
}
