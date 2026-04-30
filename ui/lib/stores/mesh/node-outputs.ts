import { writable } from 'svelte/store';

export const meshNodeOutputs = writable<Record<string, string>>({});

export function recordNodeOutput(nodeId: string, text: string): void {
  meshNodeOutputs.update((m) => ({ ...m, [nodeId]: text }));
}
