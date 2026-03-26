import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AgentState, JournalEntry, SlashCommand, TaskItem } from './types';

export async function sendKeystroke(sessionId: string, key: string): Promise<void> {
  await invoke('send_keystroke', { sessionId, key });
}

export async function sendMessage(sessionId: string, message: string): Promise<void> {
  await invoke('send_message', { sessionId, message });
}

export async function getJournal(sessionId: string): Promise<JournalEntry[]> {
  return await invoke('get_journal', { sessionId });
}

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

export function onAgentsUpdate(callback: (agents: AgentState[]) => void) {
  return listen<AgentState[]>('agents-update', (event) => {
    callback(event.payload);
  });
}
