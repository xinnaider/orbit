import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';

describe('preferences stores', () => {
  beforeEach(() => {
    vi.resetModules();
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
  });

  it('defaults inspector/meta panel to hidden for Quiet Journal', async () => {
    const { metaPanelVisible } = await import('./preferences');
    expect(get(metaPanelVisible)).toBe(false);
    metaPanelVisible.set(true);
    expect(localStorage.getItem('metaPanelVisible')).toBe('true');
  });

  it('persists compact density preference', async () => {
    const { compactDensity } = await import('./preferences');
    expect(get(compactDensity)).toBe(false);
    compactDensity.set(true);
    expect(get(compactDensity)).toBe(true);
    expect(localStorage.getItem('compactDensity')).toBe('true');
  });
});
