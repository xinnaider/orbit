import { writable } from 'svelte/store';
import type { AgentTemplate } from '../../types';
import {
  meshCreateTemplate,
  meshDeleteTemplate,
  meshListTemplates,
  meshUpdateTemplate,
} from '../../tauri/mesh';
import { MESH_DEFAULT_PROVIDER } from './constants';

export const templatesStore = writable<AgentTemplate[]>([]);

export async function loadTemplates(floorId: number): Promise<void> {
  const list = await meshListTemplates(floorId);
  templatesStore.set(list);
}

export async function addTemplate(
  floorId: number,
  name: string,
  prePrompt: string,
  model: string | null,
  useWorktree: boolean,
  provider: string = MESH_DEFAULT_PROVIDER
): Promise<AgentTemplate> {
  const t = await meshCreateTemplate(floorId, name, prePrompt, model, useWorktree, provider);
  templatesStore.update((prev) => [...prev, t]);
  return t;
}

export async function editTemplate(
  id: number,
  name: string,
  prePrompt: string,
  model: string | null,
  useWorktree: boolean
): Promise<void> {
  await meshUpdateTemplate(id, name, prePrompt, model, useWorktree);
  templatesStore.update((prev) =>
    prev.map((t) => (t.id === id ? { ...t, name, prePrompt, model, useWorktree } : t))
  );
}

export async function removeTemplate(id: number): Promise<void> {
  await meshDeleteTemplate(id);
  templatesStore.update((prev) => prev.filter((t) => t.id !== id));
}

// ── Typed accessors ──────────────────────────────────────────────────────
// `prePrompt` carries different semantics per `provider`: an agent's role
// prompt, a browser URL, or a Claude Skill slug. These helpers read the
// right interpretation and throw if called against the wrong kind.

export function isAgentTemplate(t: AgentTemplate): boolean {
  return t.provider !== 'browser' && t.provider !== 'skill';
}

export function isBrowserTemplate(t: AgentTemplate): boolean {
  return t.provider === 'browser';
}

export function isSkillTemplate(t: AgentTemplate): boolean {
  return t.provider === 'skill';
}

export function getAgentPrompt(t: AgentTemplate): string {
  if (!isAgentTemplate(t)) {
    throw new Error(`template ${t.id} is not an agent (provider=${t.provider})`);
  }
  return t.prePrompt;
}

export function getBrowserUrl(t: AgentTemplate): string {
  if (!isBrowserTemplate(t)) {
    throw new Error(`template ${t.id} is not a browser (provider=${t.provider})`);
  }
  return t.prePrompt;
}

export function getSkillSlug(t: AgentTemplate): string {
  if (!isSkillTemplate(t)) {
    throw new Error(`template ${t.id} is not a skill (provider=${t.provider})`);
  }
  return t.prePrompt;
}
