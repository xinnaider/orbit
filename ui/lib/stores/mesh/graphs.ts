import { writable } from 'svelte/store';
import type { Graph } from '../../types';
import {
  meshCreateGraph,
  meshDeleteGraph,
  meshListGraphs,
  meshSetGraphProvider,
} from '../../tauri/mesh';

export const graphsStore = writable<Graph[]>([]);

export async function loadGraphs(floorId: number): Promise<void> {
  const list = await meshListGraphs(floorId);
  graphsStore.set(list);
}

export async function addGraph(
  floorId: number,
  name: string,
  provider: string | null = null
): Promise<Graph> {
  const g = await meshCreateGraph(floorId, name, provider);
  graphsStore.update((prev) => [...prev, g]);
  return g;
}

export async function removeGraph(id: number): Promise<void> {
  await meshDeleteGraph(id);
  graphsStore.update((prev) => prev.filter((g) => g.id !== id));
}

export async function changeGraphProvider(id: number, provider: string): Promise<void> {
  await meshSetGraphProvider(id, provider);
  graphsStore.update((prev) => prev.map((g) => (g.id === id ? { ...g, provider } : g)));
}
