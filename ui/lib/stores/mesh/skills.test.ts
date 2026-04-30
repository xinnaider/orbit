import { describe, expect, it, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';

vi.mock('../../tauri/mesh', () => ({
  meshListSkills: vi.fn(),
}));

import * as tauriMod from '../../tauri/mesh';
import { _internal, availableSkills, ensureSkillsLoaded, refreshSkills } from './skills';

type Fn = ReturnType<typeof vi.fn>;

function skill(slug: string) {
  return {
    slug,
    name: slug,
    description: '',
    path: `/x/${slug}/SKILL.md`,
    content: '',
  };
}

describe('skills store', () => {
  beforeEach(() => {
    _internal.resetLoaded();
    vi.clearAllMocks();
    availableSkills.set([]);
  });

  it('ensureSkillsLoaded fetches once and populates the store', async () => {
    (tauriMod.meshListSkills as unknown as Fn).mockResolvedValueOnce([skill('a'), skill('b')]);
    await ensureSkillsLoaded();
    expect(get(availableSkills)).toHaveLength(2);
  });

  it('ensureSkillsLoaded is a no-op on the second call', async () => {
    (tauriMod.meshListSkills as unknown as Fn).mockResolvedValueOnce([skill('a')]);
    await ensureSkillsLoaded();
    await ensureSkillsLoaded();
    expect(tauriMod.meshListSkills).toHaveBeenCalledTimes(1);
  });

  it('refreshSkills forces a re-fetch', async () => {
    (tauriMod.meshListSkills as unknown as Fn)
      .mockResolvedValueOnce([skill('a')])
      .mockResolvedValueOnce([skill('a'), skill('b')]);
    await ensureSkillsLoaded();
    await refreshSkills();
    expect(tauriMod.meshListSkills).toHaveBeenCalledTimes(2);
    expect(get(availableSkills)).toHaveLength(2);
  });
});
