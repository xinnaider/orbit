import type { JournalEntry, SlashCommand, TaskItem, SubagentInfo } from '../types';
import { invoke } from './invoke';

export async function createProject(name: string, path: string) {
  return await invoke('create_project', { name, path });
}

export async function listProjects() {
  return await invoke('list_projects');
}

export async function listProjectFiles(cwd: string): Promise<string[]> {
  return await invoke('list_project_files', { cwd });
}

export async function getSubagents(sessionId: number): Promise<SubagentInfo[]> {
  return await invoke('get_subagents', { sessionId });
}

export async function getSubagentJournal(
  sessionId: number,
  subagentId: string
): Promise<JournalEntry[]> {
  return await invoke('get_subagent_journal', { sessionId, subagentId });
}

export async function getSlashCommands(provider?: string): Promise<SlashCommand[]> {
  return await invoke('get_slash_commands', { provider: provider ?? null });
}

export async function getSessionTasks(sessionId: string): Promise<TaskItem[]> {
  return await invoke('get_tasks', { sessionId });
}
