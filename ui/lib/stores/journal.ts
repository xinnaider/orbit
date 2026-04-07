import { writable } from 'svelte/store';
import type { JournalEntry } from '../types';

export const journal = writable<Map<number, JournalEntry[]>>(new Map());

export interface PendingMessage {
  id: number;
  text: string;
  timestamp: string;
}

let nextId = 0;

function createPendingStore() {
  const { subscribe, update } = writable<PendingMessage[]>([]);

  return {
    subscribe,
    add(text: string) {
      const msg: PendingMessage = {
        id: nextId++,
        text,
        timestamp: new Date().toISOString(),
      };
      update((msgs) => [...msgs, msg]);
    },
    clear() {
      update(() => []);
    },
    remove(id: number) {
      update((msgs) => msgs.filter((m) => m.id !== id));
    },
  };
}

export const pendingMessages = createPendingStore();
