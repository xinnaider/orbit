import { invoke as _invoke } from '@tauri-apps/api/core';
import { listen as _listen } from '@tauri-apps/api/event';
import { getVersion as _getVersion } from '@tauri-apps/api/app';
import type { Session, TokenUsage, MiniLogEntry } from './stores/sessions';
import type { JournalEntry, SlashCommand, TaskItem, UpdateInfo, SubagentInfo } from './types';
import { mockInvoke, mockListen } from './mock/tauri-mock';

const IS_MOCK =
  import.meta.env.VITE_MOCK === 'true' ||
  !(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (IS_MOCK) return mockInvoke(cmd, args) as Promise<T>;
  return _invoke<T>(cmd, args);
}

function listen<T>(event: string, cb: (e: { payload: T }) => void): Promise<() => void> {
  if (IS_MOCK) {
    const unlisten = mockListen(event, (payload) => cb({ payload: payload as T }));
    return Promise.resolve(unlisten);
  }
  return _listen<T>(event, cb);
}

export interface CreateSessionOptions {
  projectPath: string;
  prompt: string;
  model?: string;
  permissionMode?: 'ignore' | 'approve';
  sessionName?: string;
  sshHost?: string;
  sshUser?: string;
  useWorktree?: boolean;
}

export async function createSession(opts: CreateSessionOptions): Promise<Session> {
  return await invoke('create_session', {
    projectPath: opts.projectPath,
    prompt: opts.prompt,
    model: opts.model ?? null,
    permissionMode: opts.permissionMode ?? 'ignore',
    sessionName: opts.sessionName ?? null,
    sshHost: opts.sshHost ?? null,
    sshUser: opts.sshUser ?? null,
    useWorktree: opts.useWorktree ?? false,
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

export async function createProject(name: string, path: string) {
  return await invoke('create_project', { name, path });
}

export async function listProjects() {
  return await invoke('list_projects');
}

export async function getSubagentJournal(
  sessionId: number,
  subagentId: string
): Promise<JournalEntry[]> {
  return await invoke('get_subagent_journal', { sessionId, subagentId });
}

export async function getSubagents(sessionId: number): Promise<SubagentInfo[]> {
  return await invoke('get_subagents', { sessionId });
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

export interface ClaudeUsageStats {
  weeklyTokens: number;
  todayTokens: number;
  weeklyMessages: number;
  todayMessages: number;
}

export async function getClaudeUsageStats(): Promise<ClaudeUsageStats> {
  return await invoke('get_claude_usage_stats');
}

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

export interface ClaudeCheck {
  found: boolean;
  path: string | null;
  searchedPath?: string;
  hint?: string;
}

export async function checkClaude(): Promise<ClaudeCheck> {
  return await invoke('check_claude');
}

export async function renameSession(sessionId: number, name: string): Promise<void> {
  await invoke('rename_session', { sessionId, name });
}

export async function deleteSession(sessionId: number): Promise<void> {
  await invoke('delete_session', { sessionId });
}

export interface SpawnDiagnostic {
  claudeFound: boolean;
  claudePath: string | null;
  whereOutput: string;
  versionOutput: string;
  augmentedPath: string;
  processPath: string;
}

export async function diagnoseSpawn(): Promise<SpawnDiagnostic> {
  return await invoke('diagnose_spawn');
}

export function onSessionRateLimit(cb: (sessionId: number) => void) {
  return listen<{ sessionId: number }>('session:rate-limit', (e) => cb(e.payload.sessionId));
}

export async function getAppVersion(): Promise<string> {
  if (IS_MOCK) return '0.0.0';
  return _getVersion();
}

export async function checkUpdate(): Promise<UpdateInfo | null> {
  return await invoke<UpdateInfo | null>('check_update');
}

export async function installUpdate(): Promise<void> {
  await invoke('install_update');
}

export async function getChangelog(): Promise<string> {
  return await invoke<string>('get_changelog');
}
