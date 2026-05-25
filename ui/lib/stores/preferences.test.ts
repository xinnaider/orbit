import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';

describe('preferences stores', () => {
  beforeEach(() => {
    vi.resetModules();
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
    document.documentElement.removeAttribute('data-window-opacity');
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

  it('persists window opacity and updates document attribute', async () => {
    const { windowOpacity } = await import('./preferences');
    expect(get(windowOpacity)).toBe(100);
    windowOpacity.set(70);
    expect(get(windowOpacity)).toBe(70);
    expect(localStorage.getItem('windowOpacity')).toBe('70');
    expect(document.documentElement.getAttribute('data-window-opacity')).toBe('70');
    windowOpacity.set(100);
    expect(document.documentElement.hasAttribute('data-window-opacity')).toBe(false);
  });
});
