import { describe, expect, it, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';

vi.mock('../../tauri/mesh', () => ({
  meshCreateTemplate: vi.fn(),
  meshDeleteTemplate: vi.fn(),
  meshListTemplates: vi.fn(),
  meshUpdateTemplate: vi.fn(),
}));

import * as tauriMod from '../../tauri/mesh';
import {
  addTemplate,
  editTemplate,
  getAgentPrompt,
  getBrowserUrl,
  getSkillSlug,
  isAgentTemplate,
  isBrowserTemplate,
  isSkillTemplate,
  loadTemplates,
  removeTemplate,
  templatesStore,
} from './templates';

type Fn = ReturnType<typeof vi.fn>;

function template(id: number, overrides: Partial<{ provider: string; name: string }> = {}) {
  return {
    id,
    floorId: 1,
    name: overrides.name ?? `T${id}`,
    prePrompt: 'p',
    model: null,
    provider: overrides.provider ?? 'claude-code',
    useWorktree: true,
    createdAt: '',
    updatedAt: '',
  };
}

describe('templates store', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    templatesStore.set([]);
  });

  it('loads templates for a floor', async () => {
    (tauriMod.meshListTemplates as unknown as Fn).mockResolvedValueOnce([template(1), template(2)]);
    await loadTemplates(7);
    expect(get(templatesStore)).toHaveLength(2);
  });

  it('addTemplate appends optimistically and defaults provider to claude-code', async () => {
    (tauriMod.meshCreateTemplate as unknown as Fn).mockResolvedValueOnce(template(5));
    const t = await addTemplate(1, 'Reader', 'p', null, true);
    expect(t.id).toBe(5);
    expect(get(templatesStore)).toHaveLength(1);
    const call = (tauriMod.meshCreateTemplate as unknown as Fn).mock.calls[0];
    expect(call[5]).toBe('claude-code');
  });

  it('addTemplate forwards a custom provider (e.g. browser/skill)', async () => {
    (tauriMod.meshCreateTemplate as unknown as Fn).mockResolvedValueOnce(
      template(5, { provider: 'browser' })
    );
    await addTemplate(1, 'Docs', 'https://x', null, false, 'browser');
    expect((tauriMod.meshCreateTemplate as unknown as Fn).mock.calls[0][5]).toBe('browser');
  });

  it('editTemplate updates the matching entry in place', async () => {
    templatesStore.set([template(1, { name: 'Old' })]);
    (tauriMod.meshUpdateTemplate as unknown as Fn).mockResolvedValueOnce(undefined);
    await editTemplate(1, 'New', 'p2', 'm', false);
    const t = get(templatesStore)[0];
    expect(t.name).toBe('New');
    expect(t.prePrompt).toBe('p2');
    expect(t.useWorktree).toBe(false);
  });

  it('removeTemplate drops it from the store after delete', async () => {
    templatesStore.set([template(1), template(2)]);
    (tauriMod.meshDeleteTemplate as unknown as Fn).mockResolvedValueOnce(undefined);
    await removeTemplate(1);
    const ts = get(templatesStore);
    expect(ts).toHaveLength(1);
    expect(ts[0].id).toBe(2);
  });
});

describe('templates kind helpers', () => {
  const agent = { ...template(1, { provider: 'claude-code' }), prePrompt: 'role text' };
  const browser = { ...template(2, { provider: 'browser' }), prePrompt: 'https://docs.x' };
  const skill = { ...template(3, { provider: 'skill' }), prePrompt: 'check-bugs' };

  it('classifies templates by provider', () => {
    expect(isAgentTemplate(agent)).toBe(true);
    expect(isAgentTemplate(browser)).toBe(false);
    expect(isAgentTemplate(skill)).toBe(false);
    expect(isBrowserTemplate(browser)).toBe(true);
    expect(isBrowserTemplate(agent)).toBe(false);
    expect(isSkillTemplate(skill)).toBe(true);
    expect(isSkillTemplate(agent)).toBe(false);
  });

  it('reads the right interpretation of prePrompt per kind', () => {
    expect(getAgentPrompt(agent)).toBe('role text');
    expect(getBrowserUrl(browser)).toBe('https://docs.x');
    expect(getSkillSlug(skill)).toBe('check-bugs');
  });

  it('throws when accessor is called on the wrong kind', () => {
    expect(() => getAgentPrompt(browser)).toThrow(/not an agent/);
    expect(() => getBrowserUrl(skill)).toThrow(/not a browser/);
    expect(() => getSkillSlug(agent)).toThrow(/not a skill/);
  });
});
