import { writable } from 'svelte/store';
import type { Skill } from '../../types';
import { meshListSkills } from '../../tauri/mesh';

export const availableSkills = writable<Skill[]>([]);

let loaded = false;

export async function ensureSkillsLoaded(): Promise<void> {
  if (loaded) return;
  try {
    const list = await meshListSkills();
    availableSkills.set(list);
    loaded = true;
  } catch (e) {
    console.warn('[mesh] failed to list skills', e);
  }
}

export async function refreshSkills(): Promise<void> {
  loaded = false;
  await ensureSkillsLoaded();
}

/** Test-only exports. Use `ensureSkillsLoaded`/`refreshSkills` from production code. */
export const _internal = {
  resetLoaded(): void {
    loaded = false;
  },
};
