import { writable } from 'svelte/store';

function createMutedSessionsStore() {
  const stored =
    typeof localStorage !== 'undefined'
      ? new Set<string>(JSON.parse(localStorage.getItem('mutedSessions') ?? '[]'))
      : new Set<string>();

  const { subscribe, update } = writable<Set<string>>(stored);

  subscribe((val) => {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('mutedSessions', JSON.stringify([...val]));
    }
  });

  return {
    subscribe,
    toggle(sessionId: string) {
      update((s) => {
        const next = new Set(s);
        if (next.has(sessionId)) {
          next.delete(sessionId);
        } else {
          next.add(sessionId);
        }
        return next;
      });
    },
    isMuted(set: Set<string>, sessionId: string): boolean {
      return set.has(sessionId);
    },
  };
}

export const mutedSessions = createMutedSessionsStore();

export function toggleMute(sessionId: string) {
  mutedSessions.toggle(sessionId);
}
