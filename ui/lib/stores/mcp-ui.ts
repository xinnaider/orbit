import { writable } from 'svelte/store';

/** Parent session ids expanded in the sidebar MCP tree. */
export const expandedParentSessions = writable<Set<number>>(new Set());

export function expandParentSession(parentId: number): void {
  expandedParentSessions.update((set) => {
    if (set.has(parentId)) return set;
    return new Set([...set, parentId]);
  });
}
