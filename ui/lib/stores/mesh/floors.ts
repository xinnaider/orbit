import { writable } from 'svelte/store';
import type { Floor } from '../../types';
import {
  meshCreateFloor,
  meshDeleteFloor,
  meshListFloors,
  meshRenameFloor,
} from '../../tauri/mesh';

export const floorsStore = writable<Floor[]>([]);
export const activeFloorId = writable<number | null>(null);

export async function loadFloors(): Promise<void> {
  const list = await meshListFloors();
  floorsStore.set(list);
}

export async function addFloor(name: string): Promise<Floor> {
  const f = await meshCreateFloor(name);
  floorsStore.update((prev) => [...prev, f]);
  return f;
}

export async function renameFloor(id: number, name: string): Promise<void> {
  await meshRenameFloor(id, name);
  floorsStore.update((prev) => prev.map((f) => (f.id === id ? { ...f, name } : f)));
}

export async function removeFloor(id: number): Promise<void> {
  await meshDeleteFloor(id);
  floorsStore.update((prev) => prev.filter((f) => f.id !== id));
}
