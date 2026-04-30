import { writable } from 'svelte/store';
import type { CanvasAnnotation, GraphEdge, GraphNode } from '../../types';
import {
  meshAddEdge,
  meshAddNode,
  meshListAnnotations,
  meshListEdges,
  meshListNodes,
  meshMoveNode,
  meshRemoveEdge,
  meshRemoveNode,
  meshResizeNode,
} from '../../tauri/mesh';
import { unbindNote } from './notes';

export const nodesStore = writable<GraphNode[]>([]);
export const edgesStore = writable<GraphEdge[]>([]);
export const annotationsStore = writable<CanvasAnnotation[]>([]);
export const activeGraphId = writable<number | null>(null);

export function clearGraph(): void {
  nodesStore.set([]);
  edgesStore.set([]);
  annotationsStore.set([]);
  activeGraphId.set(null);
}

export async function loadGraph(graphId: number): Promise<void> {
  const [nodes, edges, ann] = await Promise.all([
    meshListNodes(graphId),
    meshListEdges(graphId),
    meshListAnnotations(graphId),
  ]);
  nodesStore.set(nodes);
  edgesStore.set(edges);
  annotationsStore.set(ann);
  activeGraphId.set(graphId);
}

export async function createNode(
  graphId: number,
  templateId: number,
  displayName: string,
  x: number,
  y: number
): Promise<GraphNode> {
  const n = await meshAddNode(graphId, templateId, displayName, x, y);
  nodesStore.update((prev) => [...prev, n]);
  return n;
}

export async function deleteNode(id: number): Promise<void> {
  await meshRemoveNode(id);
  nodesStore.update((prev) => prev.filter((n) => n.id !== id));
  edgesStore.update((prev) => prev.filter((e) => e.fromNodeId !== id && e.toNodeId !== id));
  // Note rows cascade-delete in DB; mirror that locally so the notes store stays in sync.
  unbindNote(id);
}

export async function relocateNode(id: number, x: number, y: number): Promise<void> {
  await meshMoveNode(id, x, y);
  nodesStore.update((prev) => prev.map((n) => (n.id === id ? { ...n, x, y } : n)));
}

export async function resizeNode(id: number, width: number, height: number): Promise<void> {
  await meshResizeNode(id, width, height);
  nodesStore.update((prev) => prev.map((n) => (n.id === id ? { ...n, width, height } : n)));
}

export async function createEdge(
  graphId: number,
  fromNodeId: number,
  toNodeId: number,
  fromHandle: string | null = null,
  toHandle: string | null = null
): Promise<GraphEdge> {
  const e = await meshAddEdge(graphId, fromNodeId, toNodeId, fromHandle, toHandle);
  edgesStore.update((prev) => [...prev, e]);
  return e;
}

export async function deleteEdge(id: number): Promise<void> {
  await meshRemoveEdge(id);
  edgesStore.update((prev) => prev.filter((e) => e.id !== id));
}
