import { describe, it, expect, beforeEach, vi } from 'vitest';
import { messageHistory } from './history';

beforeEach(() => {
  const storage = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, val: string) => storage.set(key, val),
    removeItem: (key: string) => storage.delete(key),
    clear: () => storage.clear(),
  });
  localStorage.clear();
});

describe('messageHistory', () => {
  // Each test uses a unique session ID to avoid module-level state pollution

  it('starts with empty history', () => {
    expect(messageHistory.up('hist-empty', '')).toBeNull();
  });

  it('push adds message', () => {
    messageHistory.push('hist-push', 'msg');
    expect(messageHistory.up('hist-push', 'new')).toBe('msg');
  });

  it('up returns messages in reverse order', () => {
    messageHistory.push('hist-up-order', 'msg1');
    messageHistory.push('hist-up-order', 'msg2');
    expect(messageHistory.up('hist-up-order', 'new')).toBe('msg2');
    expect(messageHistory.up('hist-up-order', 'new')).toBe('msg1');
  });

  it('down goes back through history', () => {
    messageHistory.push('hist-down', 'msg1');
    messageHistory.push('hist-down', 'msg2');
    messageHistory.up('hist-down', '');
    messageHistory.up('hist-down', '');
    // after going up to msg2 then msg1, down goes back to msg2
    expect(messageHistory.down('hist-down')).toBe('msg2');
  });

  it('down returns saved text when cursor at 0', () => {
    messageHistory.push('hist-saved', 'msg');
    messageHistory.up('hist-saved', 'draft');
    expect(messageHistory.down('hist-saved')).toBe('draft');
  });

  it('resetCursor resets navigation', () => {
    messageHistory.push('hist-reset', 'msg');
    expect(messageHistory.up('hist-reset', '')).toBe('msg');
    messageHistory.resetCursor('hist-reset');
    expect(messageHistory.up('hist-reset', '')).toBe('msg');
  });

  it('no duplicate on push', () => {
    messageHistory.push('hist-dedup', 'msg');
    messageHistory.push('hist-dedup', 'msg');
    expect(messageHistory.up('hist-dedup', 'new')).toBe('msg');
    expect(messageHistory.up('hist-dedup', 'new')).toBeNull();
  });

  it('session isolation', () => {
    messageHistory.push('hist-iso-a', 'msg1');
    expect(messageHistory.up('hist-iso-b', '')).toBeNull();
  });
});
