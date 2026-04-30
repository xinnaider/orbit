import { invoke } from './invoke';
import type {
  AgentTemplate,
  CanvasAnnotation,
  Floor,
  Graph,
  GraphEdge,
  GraphNode,
  MeshNote,
  NewAnnotation,
  Run,
  RunSession,
  Skill,
} from '../types';

// ── Mesh — Floors ─────────────────────────────────────────

export const meshCreateFloor = (name: string) => invoke<Floor>('mesh_create_floor', { name });

export const meshListFloors = () => invoke<Floor[]>('mesh_list_floors');

export const meshRenameFloor = (id: number, name: string) =>
  invoke<void>('mesh_rename_floor', { id, name });

export const meshDeleteFloor = (id: number) => invoke<void>('mesh_delete_floor', { id });

// ── Mesh — Templates ───────────────────────────────────────

export const meshCreateTemplate = (
  floorId: number,
  name: string,
  prePrompt: string,
  model: string | null,
  useWorktree: boolean,
  provider: string = 'claude-code'
) =>
  invoke<AgentTemplate>('mesh_create_template', {
    floorId,
    name,
    prePrompt,
    model,
    useWorktree,
    provider,
  });

export const meshListTemplates = (floorId: number) =>
  invoke<AgentTemplate[]>('mesh_list_templates', { floorId });

export const meshUpdateTemplate = (
  id: number,
  name: string,
  prePrompt: string,
  model: string | null,
  useWorktree: boolean
) =>
  invoke<void>('mesh_update_template', {
    id,
    name,
    prePrompt,
    model,
    useWorktree,
  });

export const meshDeleteTemplate = (id: number) => invoke<void>('mesh_delete_template', { id });

// ── Mesh — Graphs ──────────────────────────────────────────

export const meshCreateGraph = (floorId: number, name: string, provider: string | null = null) =>
  invoke<Graph>('mesh_create_graph', { floorId, name, provider });

export const meshListGraphs = (floorId: number) => invoke<Graph[]>('mesh_list_graphs', { floorId });

export const meshSetGraphEntry = (id: number, entryNodeId: number | null) =>
  invoke<void>('mesh_set_graph_entry', { id, entryNodeId });

export const meshSetGraphProvider = (id: number, provider: string) =>
  invoke<void>('mesh_set_graph_provider', { id, provider });

export const meshDeleteGraph = (id: number) => invoke<void>('mesh_delete_graph', { id });

// ── Mesh — Nodes ───────────────────────────────────────────

export const meshAddNode = (
  graphId: number,
  templateId: number,
  displayName: string,
  x: number,
  y: number
) =>
  invoke<GraphNode>('mesh_add_node', {
    graphId,
    templateId,
    displayName,
    x,
    y,
  });

export const meshMoveNode = (id: number, x: number, y: number) =>
  invoke<void>('mesh_move_node', { id, x, y });

export const meshResizeNode = (id: number, width: number, height: number) =>
  invoke<void>('mesh_resize_node', { id, width, height });

export const meshRemoveNode = (id: number) => invoke<void>('mesh_remove_node', { id });

export const meshListNodes = (graphId: number) =>
  invoke<GraphNode[]>('mesh_list_nodes', { graphId });

// ── Mesh — Edges ───────────────────────────────────────────

export const meshAddEdge = (
  graphId: number,
  fromNodeId: number,
  toNodeId: number,
  fromHandle: string | null = null,
  toHandle: string | null = null
) =>
  invoke<GraphEdge>('mesh_add_edge', {
    graphId,
    fromNodeId,
    toNodeId,
    fromHandle,
    toHandle,
  });

export const meshRemoveEdge = (id: number) => invoke<void>('mesh_remove_edge', { id });

export const meshListEdges = (graphId: number) =>
  invoke<GraphEdge[]>('mesh_list_edges', { graphId });

// ── Mesh — Notes ───────────────────────────────────────────

export const meshNoteCreate = (graphId: number, name: string, x: number, y: number) =>
  invoke<MeshNote>('mesh_note_create', { graphId, name, x, y });

export const meshNoteGet = (nodeId: number) => invoke<MeshNote>('mesh_note_get', { nodeId });

export const meshNoteList = (graphId: number) => invoke<MeshNote[]>('mesh_note_list', { graphId });

export const meshNoteSetContent = (nodeId: number, content: string) =>
  invoke<void>('mesh_note_set_content', { nodeId, content });

export const meshNoteRename = (nodeId: number, name: string) =>
  invoke<void>('mesh_note_rename', { nodeId, name });

// ── Mesh — Annotations ─────────────────────────────────────

export const meshSaveAnnotations = (graphId: number, items: NewAnnotation[]) =>
  invoke<void>('mesh_save_annotations', {
    graphId,
    items: items.map((a) => ({
      kind: a.kind,
      payload: a.payload,
      z_index: a.zIndex,
    })),
  });

export const meshListAnnotations = (graphId: number) =>
  invoke<CanvasAnnotation[]>('mesh_list_annotations', { graphId });

// ── Mesh — Skills (Claude Code skills from ~/.claude/skills/) ──────────────

export const meshListSkills = () => invoke<Skill[]>('mesh_list_skills');

export const meshReadSkill = (slug: string) => invoke<Skill>('mesh_read_skill', { slug });

// ── Mesh — Runs ────────────────────────────────────────────────────────────

export const meshCreateRun = (graphId: number, entryNodeId: number, initialPrompt: string | null) =>
  invoke<Run>('mesh_create_run', { graphId, entryNodeId, initialPrompt });

export const meshFinishRun = (runId: number, status: 'completed' | 'stopped' | 'error') =>
  invoke<void>('mesh_finish_run', { runId, status });

export const meshRecordRunSession = (runId: number, nodeId: number, sessionId: number) =>
  invoke<RunSession>('mesh_record_run_session', { runId, nodeId, sessionId });

export const meshListRunSessions = (runId: number) =>
  invoke<RunSession[]>('mesh_list_run_sessions', { runId });

export const meshFindActiveRun = (graphId: number) =>
  invoke<Run | null>('mesh_find_active_run', { graphId });
