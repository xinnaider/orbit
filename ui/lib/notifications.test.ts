import { describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import { notificationsEnabled } from './stores/preferences';
import { mutedSessions, toggleMute } from './stores/ui';

describe('notification preferences', () => {
  it('notificationsEnabled store accepts toggle', () => {
    notificationsEnabled.set(false);
    expect(get(notificationsEnabled)).toBe(false);
    notificationsEnabled.set(true);
    expect(get(notificationsEnabled)).toBe(true);
  });

  it('muted session is tracked per id', () => {
    toggleMute('7');
    expect(mutedSessions.isMuted(get(mutedSessions), '7')).toBe(true);
    toggleMute('7');
    expect(mutedSessions.isMuted(get(mutedSessions), '7')).toBe(false);
  });
});
