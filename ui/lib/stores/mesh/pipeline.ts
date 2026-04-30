import { writable, get } from 'svelte/store';
import type { GraphNode } from '../../types';
import {
  createSession,
  deleteSession,
  onSessionError,
  onSessionStopped,
  stopSession,
} from '../../tauri';
import {
  meshCreateRun,
  meshFinishRun,
  meshReadSkill,
  meshRecordRunSession,
} from '../../tauri/mesh';
import { nodesStore, edgesStore } from './graph';
import { graphsStore } from './graphs';
import { notesStore } from './notes';
import { getAgentPrompt, getSkillSlug, isSkillTemplate, templatesStore } from './templates';
import { meshNodeSessions } from './node-sessions';
import { meshNodeOutputs } from './node-outputs';
import {
  MESH_DEFAULT_PERMISSION_MODE,
  MESH_DEFAULT_PROVIDER,
  MESH_NOTE_PROVIDER,
  MESH_SESSION_PREFIX,
} from './constants';
import { addToast } from '../toasts';

type ActivePipeline = {
  runId: number;
  graphId: number;
  provider: string;
  cwd: string;
  triggered: Set<string>;
  done: Set<string>;
};

export const activePipeline = writable<ActivePipeline | null>(null);
let globalUnlisteners: Array<() => void> = [];

function reverseSessionMap(): Map<number, string> {
  const sessions = get(meshNodeSessions);
  const map = new Map<number, string>();
  for (const [nodeId, sid] of Object.entries(sessions)) {
    map.set(sid, nodeId);
  }
  return map;
}

function templateProvider(templateId: number): string {
  const t = get(templatesStore).find((x) => x.id === templateId);
  return t?.provider ?? MESH_DEFAULT_PROVIDER;
}

function isAgentNode(n: GraphNode): boolean {
  const p = templateProvider(n.templateId);
  return p !== 'browser' && p !== 'skill' && p !== MESH_NOTE_PROVIDER;
}

function findDownstreamNodes(nodeId: string): string[] {
  const edges = get(edgesStore);
  const nid = Number(nodeId);
  return edges.filter((e) => e.fromNodeId === nid).map((e) => String(e.toNodeId));
}

function findEntryNode(graphId: number): GraphNode | undefined {
  // Entry = an AGENT node with no incoming edges from other AGENT nodes.
  // Skill edges are context, not pipeline flow.
  const nodes = get(nodesStore).filter((n) => n.graphId === graphId);
  const edges = get(edgesStore).filter((e) => e.graphId === graphId);
  const agentEdges = edges.filter((e) => {
    const src = nodes.find((n) => n.id === e.fromNodeId);
    return src ? isAgentNode(src) : false;
  });
  const agentTargets = new Set(agentEdges.map((e) => e.toNodeId));
  return nodes.find((n) => isAgentNode(n) && !agentTargets.has(n.id));
}

async function buildSkillsContext(agentNodeId: number): Promise<string> {
  const nodes = get(nodesStore);
  const edges = get(edgesStore);
  const incoming = edges.filter((e) => e.toNodeId === agentNodeId);
  const skillNodes = incoming
    .map((e) => nodes.find((n) => n.id === e.fromNodeId))
    .filter((n): n is GraphNode => !!n && templateProvider(n.templateId) === 'skill');

  if (skillNodes.length === 0) return '';

  const chunks: string[] = [];
  for (const sn of skillNodes) {
    const template = get(templatesStore).find((t) => t.id === sn.templateId);
    if (!template || !isSkillTemplate(template)) continue;
    const slug = getSkillSlug(template);
    // Throw instead of toasting — the caller aborts the run rather than
    // ship a partial prompt missing a skill the user wired in.
    const skill = await meshReadSkill(slug);
    chunks.push(`### Skill: ${skill.name}\n\n_${skill.description}_\n\n${skill.content}`);
  }
  if (chunks.length === 0) return '';
  return `## Available skills\n\n${chunks.join('\n\n---\n\n')}\n\n`;
}

/** Walks every note reachable backwards from `agentNodeId` (note→agent + note→note),
 * returning the unique notes in BFS order. Cycle-safe. */
function collectIncomingNotes(agentNodeId: number): number[] {
  const nodes = get(nodesStore);
  const edges = get(edgesStore);
  const ordered: number[] = [];
  const seen = new Set<number>();
  const queue: number[] = [agentNodeId];
  while (queue.length > 0) {
    const current = queue.shift()!;
    for (const e of edges.filter((x) => x.toNodeId === current)) {
      if (seen.has(e.fromNodeId)) continue;
      const src = nodes.find((n) => n.id === e.fromNodeId);
      if (!src || templateProvider(src.templateId) !== MESH_NOTE_PROVIDER) continue;
      seen.add(e.fromNodeId);
      ordered.push(e.fromNodeId);
      queue.push(e.fromNodeId); // walk note→note chain
    }
  }
  return ordered;
}

function buildNotesContext(agentNodeId: number): string {
  const noteIds = collectIncomingNotes(agentNodeId);
  if (noteIds.length === 0) return '';
  const notes = get(notesStore);
  const chunks: string[] = [];
  for (const id of noteIds) {
    const n = notes.find((x) => x.nodeId === id);
    if (!n || !n.content.trim()) continue;
    chunks.push(`### Note: ${n.name}\n\n${n.content}`);
  }
  if (chunks.length === 0) return '';
  return `## Connected notes\n\n${chunks.join('\n\n---\n\n')}\n\n`;
}

async function startNode(
  nodeId: string,
  prompt: string,
  cwd: string,
  provider: string
): Promise<void> {
  const node = get(nodesStore).find((n) => String(n.id) === nodeId);
  if (!node) return;
  const template = get(templatesStore).find((t) => t.id === node.templateId);
  if (!template) return;
  // Browser/skill/note nodes don't run a Claude session — skip.
  if (
    template.provider === 'browser' ||
    template.provider === 'skill' ||
    template.provider === MESH_NOTE_PROVIDER
  )
    return;

  const skillsCtx = await buildSkillsContext(node.id);
  const notesCtx = buildNotesContext(node.id);
  const fullPrompt = `${getAgentPrompt(template)}\n\n${skillsCtx}${notesCtx}---\n\n${prompt}`;
  const session = await createSession({
    projectPath: cwd,
    prompt: fullPrompt,
    model: template.model ?? undefined,
    permissionMode: MESH_DEFAULT_PERMISSION_MODE,
    sessionName: `${MESH_SESSION_PREFIX}${node.displayName}`,
    useWorktree: template.useWorktree,
    provider,
  });
  meshNodeSessions.update((m) => ({ ...m, [nodeId]: session.id }));

  const state = get(activePipeline);
  if (state) {
    try {
      await meshRecordRunSession(state.runId, node.id, session.id);
    } catch (e) {
      console.warn('[mesh pipeline] failed to record run_session', e);
    }
  }
}

async function finishRunSafely(
  runId: number,
  status: 'completed' | 'stopped' | 'error'
): Promise<void> {
  try {
    await meshFinishRun(runId, status);
  } catch (e) {
    console.warn('[mesh pipeline] failed to finish run', runId, status, e);
  }
}

async function onSessionStop(sessionId: number): Promise<void> {
  const state = get(activePipeline);
  if (!state) return;

  const nodeId = reverseSessionMap().get(sessionId);
  if (!nodeId || state.done.has(nodeId)) return;

  state.done.add(nodeId);
  activePipeline.set(state);

  const output = get(meshNodeOutputs)[nodeId] ?? '';
  const downstreamIds = findDownstreamNodes(nodeId);

  for (const downId of downstreamIds) {
    // Race guard: pipeline may have been stopped during an earlier await.
    if (get(activePipeline) !== state) return;
    if (state.triggered.has(downId)) continue;
    const downNode = get(nodesStore).find((n) => String(n.id) === downId);
    if (!downNode || !isAgentNode(downNode)) continue; // skip browser/skill
    state.triggered.add(downId);
    const contextMsg = `## Previous agent's output\n\n${output}`;
    try {
      await startNode(downId, contextMsg, state.cwd, state.provider);
    } catch (e) {
      addToast({
        type: 'error',
        message: `failed to start downstream node: ${e}`,
        autoDismiss: true,
      });
      await finishRunSafely(state.runId, 'error');
      activePipeline.set(null);
      return;
    }
  }
  if (get(activePipeline) !== state) return;
  activePipeline.set(state);

  const pending = [...state.triggered].filter((id) => !state.done.has(id));
  if (pending.length === 0) {
    await finishRunSafely(state.runId, 'completed');
    activePipeline.set(null);
  }
}

function onSessionFailure(sessionId: number, err: string): void {
  const state = get(activePipeline);
  if (!state) return;

  const nodeId = reverseSessionMap().get(sessionId);
  if (!nodeId) return;

  // Abort the run — never propagate a failed/partial output downstream.
  addToast({
    type: 'error',
    message: `pipeline aborted: node "${nodeId}" failed (${err})`,
    autoDismiss: true,
  });
  void finishRunSafely(state.runId, 'error');
  activePipeline.set(null);
}

async function ensureGlobalListeners(): Promise<void> {
  if (globalUnlisteners.length > 0) return;
  const unStop = await onSessionStopped(onSessionStop);
  const unErr = await onSessionError(onSessionFailure);
  globalUnlisteners = [unStop, unErr];
}

export async function startPipeline(graphId: number, task: string, cwd: string): Promise<void> {
  const entry = findEntryNode(graphId);
  if (!entry) {
    throw new Error('graph has no entry node (every node has an incoming edge). break a cycle.');
  }
  const graph = get(graphsStore).find((g) => g.id === graphId);
  const provider = graph?.provider ?? MESH_DEFAULT_PROVIDER;
  meshNodeOutputs.set({});
  meshNodeSessions.set({});

  // Create the run before spawning so startNode has a runId to bind to.
  const run = await meshCreateRun(graphId, entry.id, task);

  activePipeline.set({
    runId: run.id,
    graphId,
    provider,
    cwd,
    triggered: new Set([String(entry.id)]),
    done: new Set(),
  });

  await ensureGlobalListeners();
  try {
    await startNode(String(entry.id), `## Initial task\n\n${task}`, cwd, provider);
  } catch (e) {
    await finishRunSafely(run.id, 'error');
    activePipeline.set(null);
    throw e;
  }
}

export function stopPipeline(): void {
  const state = get(activePipeline);
  if (state) {
    void finishRunSafely(state.runId, 'stopped');
    void stopAllAgents();
  }
  activePipeline.set(null);
}

/** Stop every running session bound to a Mesh node. Does NOT touch activePipeline. */
export async function stopAllAgents(): Promise<void> {
  const sessions = get(meshNodeSessions);
  const ids = Object.values(sessions);
  await Promise.allSettled(ids.map((sid) => stopSession(sid)));
}

/** Stop, delete, and unbind an agent's session so the next Start is fresh. */
export async function clearAgent(nodeId: string): Promise<void> {
  const sid = get(meshNodeSessions)[nodeId];
  if (sid !== undefined) {
    try {
      await stopSession(sid);
    } catch {
      // Session may already be stopped — proceed to delete anyway.
    }
    try {
      await deleteSession(sid);
    } catch (e) {
      console.warn('[mesh] failed to delete session on clear', e);
    }
  }
  meshNodeSessions.update((m) => {
    const next = { ...m };
    delete next[nodeId];
    return next;
  });
  meshNodeOutputs.update((m) => {
    const next = { ...m };
    delete next[nodeId];
    return next;
  });
}

/** clearAgent for every bound node; aborts the active pipeline first. */
export async function clearAllAgents(): Promise<void> {
  if (get(activePipeline)) stopPipeline();
  const sessions = get(meshNodeSessions);
  await Promise.allSettled(
    Object.values(sessions).map(async (sid) => {
      try {
        await stopSession(sid);
      } catch {
        /* may not be running */
      }
      try {
        await deleteSession(sid);
      } catch (e) {
        console.warn('[mesh] failed to delete session on clear all', e);
      }
    })
  );
  meshNodeSessions.set({});
  meshNodeOutputs.set({});
}

/** Test-only exports. Use `startPipeline`/`stopPipeline` from production code. */
export const _internal = {
  findEntryNode,
  findDownstreamNodes,
  isAgentNode,
  buildSkillsContext,
  collectIncomingNotes,
  buildNotesContext,
  templateProvider,
  onSessionStop,
  onSessionFailure,
  resetGlobalListeners(): void {
    for (const u of globalUnlisteners) u();
    globalUnlisteners = [];
  },
};
