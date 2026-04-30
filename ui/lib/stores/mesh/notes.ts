import { writable } from 'svelte/store';
import type { MeshNote } from '../../types';
import { meshNoteCreate, meshNoteList, meshNoteRename, meshNoteSetContent } from '../../tauri/mesh';

export const notesStore = writable<MeshNote[]>([]);

export async function loadNotes(graphId: number): Promise<void> {
  const list = await meshNoteList(graphId);
  notesStore.set(list);
}

export async function addNote(
  graphId: number,
  name: string,
  x: number,
  y: number
): Promise<MeshNote> {
  const n = await meshNoteCreate(graphId, name, x, y);
  notesStore.update((prev) => [...prev, n]);
  return n;
}

export async function setNoteContent(nodeId: number, content: string): Promise<void> {
  await meshNoteSetContent(nodeId, content);
  notesStore.update((prev) =>
    prev.map((n) =>
      n.nodeId === nodeId ? { ...n, content, updatedAt: new Date().toISOString() } : n
    )
  );
}

export async function renameNote(nodeId: number, name: string): Promise<void> {
  await meshNoteRename(nodeId, name);
  notesStore.update((prev) => prev.map((n) => (n.nodeId === nodeId ? { ...n, name } : n)));
}

/** Drop a note from the local store after its node was removed (cascade clears DB). */
export function unbindNote(nodeId: number): void {
  notesStore.update((prev) => prev.filter((n) => n.nodeId !== nodeId));
}
