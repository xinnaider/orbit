import { writable } from 'svelte/store';
import type { AgentState } from '../types';

export const agents = writable<AgentState[]>([]);
export const selectedAgentId = writable<string | null>(null);

export function selectedAgent(agentList: AgentState[], id: string | null): AgentState | null {
  if (!id) return null;
  return agentList.find((a) => a.sessionId === id) ?? null;
}
