import { describe, expect, it, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';

vi.mock('../../tauri/mesh', () => ({
  meshNoteCreate: vi.fn(),
  meshNoteList: vi.fn(),
  meshNoteRename: vi.fn(),
  meshNoteSetContent: vi.fn(),
}));

import * as tauriMod from '../../tauri/mesh';
import { addNote, loadNotes, notesStore, renameNote, setNoteContent, unbindNote } from './notes';

type Fn = ReturnType<typeof vi.fn>;

function note(nodeId: number, name = `n${nodeId}`, content = '') {
  return {
    nodeId,
    graphId: 1,
    name,
    content,
    x: 0,
    y: 0,
    width: null,
    height: null,
    updatedAt: '2026-04-25T00:00:00Z',
  };
}

describe('notes store', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    notesStore.set([]);
  });

  it('loadNotes replaces the store', async () => {
    (tauriMod.meshNoteList as unknown as Fn).mockResolvedValueOnce([note(1), note(2)]);
    await loadNotes(7);
    expect(get(notesStore)).toHaveLength(2);
  });

  it('addNote appends optimistically', async () => {
    (tauriMod.meshNoteCreate as unknown as Fn).mockResolvedValueOnce(note(5, 'New'));
    const n = await addNote(1, 'New', 0, 0);
    expect(n.nodeId).toBe(5);
    expect(get(notesStore)).toHaveLength(1);
  });

  it('setNoteContent patches in place', async () => {
    notesStore.set([note(1, 'A', 'old')]);
    (tauriMod.meshNoteSetContent as unknown as Fn).mockResolvedValueOnce(undefined);
    await setNoteContent(1, 'fresh body');
    expect(get(notesStore)[0].content).toBe('fresh body');
  });

  it('renameNote patches in place', async () => {
    notesStore.set([note(1, 'old')]);
    (tauriMod.meshNoteRename as unknown as Fn).mockResolvedValueOnce(undefined);
    await renameNote(1, 'fresh');
    expect(get(notesStore)[0].name).toBe('fresh');
  });

  it('unbindNote drops the note from the store', () => {
    notesStore.set([note(1), note(2)]);
    unbindNote(1);
    const list = get(notesStore);
    expect(list).toHaveLength(1);
    expect(list[0].nodeId).toBe(2);
  });
});
