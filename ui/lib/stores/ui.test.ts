import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { mutedSessions, sessionEffort } from './ui';

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

describe('mutedSessions', () => {
  // Tests run sequentially; state persists from one test to the next.

  it('starts empty', () => {
    expect(get(mutedSessions)).toEqual(new Set());
  });

  it('toggle adds session', () => {
    mutedSessions.toggle('1');
    expect(get(mutedSessions).has('1')).toBe(true);
  });

  it('toggle again removes session', () => {
    // store has { '1' } from previous test
    mutedSessions.toggle('1');
    expect(get(mutedSessions)).toEqual(new Set());
  });

  it('isMuted checks membership', () => {
    mutedSessions.toggle('1');
    const set = get(mutedSessions);
    expect(mutedSessions.isMuted(set, '1')).toBe(true);
    expect(mutedSessions.isMuted(set, '2')).toBe(false);
  });

  it('multiple sessions independent', () => {
    mutedSessions.toggle('2');
    const set = get(mutedSessions);
    expect(set.has('1')).toBe(true);
    expect(set.has('2')).toBe(true);
  });
});

describe('sessionEffort', () => {
  it('starts empty', () => {
    expect(get(sessionEffort)).toEqual({});
  });

  it('set stores value', () => {
    sessionEffort.set('1', 'max');
    expect(get(sessionEffort)['1']).toBe('max');
  });

  it('get returns default', () => {
    expect(sessionEffort.get({}, 'unknown')).toBe('high');
  });

  it('set overwrites', () => {
    sessionEffort.set('1', 'low');
    sessionEffort.set('1', 'high');
    expect(get(sessionEffort)['1']).toBe('high');
  });

  it('multiple sessions', () => {
    sessionEffort.set('1', 'low');
    sessionEffort.set('2', 'max');
    const map = get(sessionEffort);
    expect(map['1']).toBe('low');
    expect(map['2']).toBe('max');
  });
});
