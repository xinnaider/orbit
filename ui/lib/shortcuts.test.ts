import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  browseButtonLabel,
  detectPlatform,
  formatModChord,
  inspectorToggleHint,
  isMacOS,
  modKeyLabel,
  splitPaneHint,
  workspaceShortcutsFooter,
} from './shortcuts';

function mockNavigator(platform: string, userAgent = '') {
  vi.stubGlobal('navigator', {
    platform,
    userAgent,
  });
}

describe('shortcuts', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('detects macOS', () => {
    mockNavigator('MacIntel', 'Mozilla/5.0 (Macintosh; Intel Mac OS X)');
    expect(detectPlatform()).toBe('mac');
    expect(isMacOS()).toBe(true);
  });

  it('detects Windows', () => {
    mockNavigator('Win32', 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)');
    expect(detectPlatform()).toBe('windows');
    expect(isMacOS()).toBe(false);
  });

  it('formats inspector hint for macOS', () => {
    mockNavigator('MacIntel');
    expect(inspectorToggleHint()).toBe('⌘I');
    expect(modKeyLabel()).toBe('⌘');
  });

  it('formats inspector hint for Windows', () => {
    mockNavigator('Win32');
    expect(inspectorToggleHint()).toBe('Ctrl+I');
    expect(modKeyLabel()).toBe('Ctrl');
  });

  it('formats split pane hint per platform', () => {
    mockNavigator('MacIntel');
    expect(splitPaneHint()).toBe('⌘\\');

    mockNavigator('Win32');
    expect(splitPaneHint()).toBe('Ctrl+\\');
  });

  it('builds workspace footer with platform-specific chords', () => {
    mockNavigator('Win32');
    expect(workspaceShortcutsFooter()).toContain('Ctrl+\\');
    expect(workspaceShortcutsFooter()).toContain('Ctrl+I');
    expect(workspaceShortcutsFooter()).not.toContain('⌘');
  });

  it('browse button uses ellipsis off macOS', () => {
    mockNavigator('Win32');
    expect(browseButtonLabel()).toBe('…');

    mockNavigator('MacIntel');
    expect(browseButtonLabel()).toBe('⌘');
  });

  it('formatModChord supports custom separator', () => {
    mockNavigator('Win32');
    expect(formatModChord('S', '+')).toBe('Ctrl+S');
  });

  it('defaults to non-mac when navigator is missing', () => {
    vi.stubGlobal('navigator', undefined);
    expect(isMacOS()).toBe(false);
    expect(inspectorToggleHint()).toBe('Ctrl+I');
  });
});
