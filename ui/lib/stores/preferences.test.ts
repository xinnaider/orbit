import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';

describe('preferences stores', () => {
  beforeEach(() => {
    vi.resetModules();
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
    document.documentElement.removeAttribute('data-glass-chrome');
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

  it('defaults frosted chrome off and persists when toggled', async () => {
    const { glassChrome } = await import('./preferences');
    expect(get(glassChrome)).toBe(false);
    expect(document.documentElement.hasAttribute('data-glass-chrome')).toBe(false);
    glassChrome.set(true);
    expect(get(glassChrome)).toBe(true);
    expect(document.documentElement.getAttribute('data-glass-chrome')).toBe('true');
    expect(localStorage.getItem('glassChrome')).toBe('true');
  });

  it('enables frosted chrome when Glass theme is selected', async () => {
    const { theme, glassChrome } = await import('./preferences');
    theme.set('glass');
    expect(get(theme)).toBe('glass');
    expect(get(glassChrome)).toBe(true);
    expect(document.documentElement.getAttribute('data-theme')).toBe('glass');
  });
});
