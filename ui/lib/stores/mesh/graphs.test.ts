import { describe, expect, it, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';

vi.mock('../../tauri/mesh', () => ({
  meshCreateGraph: vi.fn(),
  meshDeleteGraph: vi.fn(),
  meshListGraphs: vi.fn(),
  meshSetGraphProvider: vi.fn(),
}));

import * as tauriMod from '../../tauri/mesh';
import { addGraph, changeGraphProvider, graphsStore, loadGraphs, removeGraph } from './graphs';

type Fn = ReturnType<typeof vi.fn>;

function graph(id: number) {
  return {
    id,
    floorId: 1,
    name: `G${id}`,
    entryNodeId: null,
    provider: 'claude-code',
    createdAt: '',
    updatedAt: '',
  };
}

describe('graphs store', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    graphsStore.set([]);
  });

  it('loads graphs for a floor', async () => {
    (tauriMod.meshListGraphs as unknown as Fn).mockResolvedValueOnce([graph(1), graph(2)]);
    await loadGraphs(7);
    expect(get(graphsStore)).toHaveLength(2);
  });

  it('addGraph appends optimistically', async () => {
    (tauriMod.meshCreateGraph as unknown as Fn).mockResolvedValueOnce(graph(5));
    const g = await addGraph(1, 'Pipeline');
    expect(g.id).toBe(5);
    expect(get(graphsStore)).toHaveLength(1);
  });

  it('removeGraph drops it from the store after delete', async () => {
    graphsStore.set([graph(1), graph(2)]);
    (tauriMod.meshDeleteGraph as unknown as Fn).mockResolvedValueOnce(undefined);
    await removeGraph(1);
    const gs = get(graphsStore);
    expect(gs).toHaveLength(1);
    expect(gs[0].id).toBe(2);
  });

  it('changeGraphProvider patches the matching entry in place', async () => {
    graphsStore.set([graph(1), graph(2)]);
    (tauriMod.meshSetGraphProvider as unknown as Fn).mockResolvedValueOnce(undefined);
    await changeGraphProvider(2, 'codex');
    const gs = get(graphsStore);
    expect(gs.find((g) => g.id === 1)?.provider).toBe('claude-code');
    expect(gs.find((g) => g.id === 2)?.provider).toBe('codex');
  });
});
