import { describe, expect, it, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';

vi.mock('../../tauri/mesh', () => ({
  meshListNodes: vi.fn(),
  meshListEdges: vi.fn(),
  meshListAnnotations: vi.fn(),
  meshAddNode: vi.fn(),
  meshAddEdge: vi.fn(),
  meshRemoveEdge: vi.fn(),
  meshRemoveNode: vi.fn(),
  meshMoveNode: vi.fn(),
  meshResizeNode: vi.fn(),
}));

import * as tauriMod from '../../tauri/mesh';
import {
  nodesStore,
  edgesStore,
  annotationsStore,
  loadGraph,
  clearGraph,
  createNode,
  deleteNode,
  createEdge,
  relocateNode,
} from './graph';

type Fn = ReturnType<typeof vi.fn>;

describe('graph store', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearGraph();
  });

  it('loads nodes, edges and annotations together', async () => {
    (tauriMod.meshListNodes as unknown as Fn).mockResolvedValueOnce([
      { id: 1, graphId: 7, templateId: 1, displayName: 'A', x: 0, y: 0 },
    ]);
    (tauriMod.meshListEdges as unknown as Fn).mockResolvedValueOnce([
      { id: 1, graphId: 7, fromNodeId: 1, toNodeId: 1 },
    ]);
    (tauriMod.meshListAnnotations as unknown as Fn).mockResolvedValueOnce([]);

    await loadGraph(7);
    expect(get(nodesStore)).toHaveLength(1);
    expect(get(edgesStore)).toHaveLength(1);
    expect(get(annotationsStore)).toHaveLength(0);
  });

  it('clearGraph() resets all stores', () => {
    nodesStore.set([{ id: 1, graphId: 1, templateId: 1, displayName: 'A', x: 0, y: 0 }]);
    clearGraph();
    expect(get(nodesStore)).toHaveLength(0);
  });

  it('createNode adds to store on success', async () => {
    (tauriMod.meshAddNode as unknown as Fn).mockResolvedValueOnce({
      id: 5,
      graphId: 1,
      templateId: 1,
      displayName: 'N',
      x: 10,
      y: 10,
    });
    const n = await createNode(1, 1, 'N', 10, 10);
    expect(n.id).toBe(5);
    expect(get(nodesStore)[0].id).toBe(5);
  });

  it('deleteNode cascades edges pointing to/from it', async () => {
    nodesStore.set([
      { id: 1, graphId: 1, templateId: 1, displayName: 'A', x: 0, y: 0 },
      { id: 2, graphId: 1, templateId: 1, displayName: 'B', x: 0, y: 0 },
    ]);
    edgesStore.set([
      { id: 1, graphId: 1, fromNodeId: 1, toNodeId: 2 },
      { id: 2, graphId: 1, fromNodeId: 2, toNodeId: 1 },
    ]);
    (tauriMod.meshRemoveNode as unknown as Fn).mockResolvedValueOnce(undefined);
    await deleteNode(1);
    expect(get(nodesStore).length).toBe(1);
    expect(get(edgesStore).length).toBe(0);
  });

  it('relocateNode updates x,y in store', async () => {
    nodesStore.set([{ id: 1, graphId: 1, templateId: 1, displayName: 'A', x: 0, y: 0 }]);
    (tauriMod.meshMoveNode as unknown as Fn).mockResolvedValueOnce(undefined);
    await relocateNode(1, 42, 99);
    const n = get(nodesStore)[0];
    expect(n.x).toBe(42);
    expect(n.y).toBe(99);
  });

  it('createEdge appends to edges store', async () => {
    (tauriMod.meshAddEdge as unknown as Fn).mockResolvedValueOnce({
      id: 1,
      graphId: 1,
      fromNodeId: 1,
      toNodeId: 2,
    });
    await createEdge(1, 1, 2);
    expect(get(edgesStore).length).toBe(1);
  });
});
