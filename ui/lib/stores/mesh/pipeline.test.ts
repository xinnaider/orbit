import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';

vi.mock('../../tauri', () => ({
  createSession: vi.fn(),
  deleteSession: vi.fn(),
  onSessionError: vi.fn(),
  onSessionStopped: vi.fn(),
  stopSession: vi.fn(),
}));

vi.mock('../../tauri/mesh', () => ({
  meshReadSkill: vi.fn(),
  meshCreateRun: vi.fn(),
  meshFinishRun: vi.fn(),
  meshRecordRunSession: vi.fn(),
  // notes.ts is reached via graph.ts; tests don't exercise these but the imports must resolve.
  meshNoteCreate: vi.fn(),
  meshNoteList: vi.fn(),
  meshNoteRename: vi.fn(),
  meshNoteSetContent: vi.fn(),
}));

vi.mock('../toasts', () => ({
  addToast: vi.fn(),
}));

import * as tauriMod from '../../tauri';
import * as meshMod from '../../tauri/mesh';
import * as toastsMod from '../toasts';
import { nodesStore, edgesStore } from './graph';
import { graphsStore } from './graphs';
import { notesStore } from './notes';
import { templatesStore } from './templates';
import { meshNodeSessions } from './node-sessions';
import { meshNodeOutputs } from './node-outputs';
import {
  activePipeline,
  clearAgent,
  clearAllAgents,
  startPipeline,
  stopAllAgents,
  stopPipeline,
  _internal,
} from './pipeline';

type Fn = ReturnType<typeof vi.fn>;

function template(id: number, provider = 'claude-code') {
  return {
    id,
    floorId: 1,
    name: `T${id}`,
    prePrompt: `prompt ${id}`,
    model: null,
    provider,
    useWorktree: true,
    createdAt: '',
    updatedAt: '',
  };
}

function node(id: number, templateId: number, displayName = `N${id}`) {
  return { id, graphId: 1, templateId, displayName, x: 0, y: 0 };
}

describe('pipeline.findEntryNode', () => {
  beforeEach(() => {
    nodesStore.set([]);
    edgesStore.set([]);
    templatesStore.set([]);
  });

  it('returns the lone agent node when no edges', () => {
    templatesStore.set([template(1)]);
    nodesStore.set([node(10, 1)]);
    const entry = _internal.findEntryNode(1);
    expect(entry?.id).toBe(10);
  });

  it('returns the agent node with no incoming agent edges', () => {
    templatesStore.set([template(1)]);
    nodesStore.set([node(10, 1, 'A'), node(11, 1, 'B'), node(12, 1, 'C')]);
    edgesStore.set([
      { id: 1, graphId: 1, fromNodeId: 10, toNodeId: 11 },
      { id: 2, graphId: 1, fromNodeId: 11, toNodeId: 12 },
    ]);
    const entry = _internal.findEntryNode(1);
    expect(entry?.id).toBe(10);
  });

  it('ignores skill-edges when picking entry (skills are context, not flow)', () => {
    templatesStore.set([template(1, 'claude-code'), template(2, 'skill')]);
    // Skill node 99 → Agent node 10. 10 should still be entry.
    nodesStore.set([node(10, 1, 'A'), node(99, 2, 'Skill')]);
    edgesStore.set([{ id: 1, graphId: 1, fromNodeId: 99, toNodeId: 10 }]);
    const entry = _internal.findEntryNode(1);
    expect(entry?.id).toBe(10);
  });

  it('returns undefined when graph has only skill nodes', () => {
    templatesStore.set([template(1, 'skill')]);
    nodesStore.set([node(10, 1)]);
    const entry = _internal.findEntryNode(1);
    expect(entry).toBeUndefined();
  });

  it('returns undefined for cycle (every agent has incoming)', () => {
    templatesStore.set([template(1)]);
    nodesStore.set([node(10, 1, 'A'), node(11, 1, 'B')]);
    edgesStore.set([
      { id: 1, graphId: 1, fromNodeId: 10, toNodeId: 11 },
      { id: 2, graphId: 1, fromNodeId: 11, toNodeId: 10 },
    ]);
    const entry = _internal.findEntryNode(1);
    expect(entry).toBeUndefined();
  });
});

describe('pipeline.findDownstreamNodes', () => {
  beforeEach(() => {
    edgesStore.set([
      { id: 1, graphId: 1, fromNodeId: 10, toNodeId: 11 },
      { id: 2, graphId: 1, fromNodeId: 10, toNodeId: 12 },
      { id: 3, graphId: 1, fromNodeId: 11, toNodeId: 13 },
    ]);
  });

  it('returns all direct downstream node ids as strings', () => {
    expect(_internal.findDownstreamNodes('10').sort()).toEqual(['11', '12']);
  });

  it('returns empty for leaf node', () => {
    expect(_internal.findDownstreamNodes('13')).toEqual([]);
  });

  it('returns empty for unknown node id', () => {
    expect(_internal.findDownstreamNodes('999')).toEqual([]);
  });
});

describe('pipeline.isAgentNode', () => {
  beforeEach(() => {
    templatesStore.set([
      template(1, 'claude-code'),
      template(2, 'browser'),
      template(3, 'skill'),
      template(4, 'note'),
    ]);
  });

  it('returns true for claude-code provider', () => {
    expect(_internal.isAgentNode(node(1, 1))).toBe(true);
  });

  it('returns false for browser provider', () => {
    expect(_internal.isAgentNode(node(1, 2))).toBe(false);
  });

  it('returns false for skill provider', () => {
    expect(_internal.isAgentNode(node(1, 3))).toBe(false);
  });

  it('returns false for note provider', () => {
    expect(_internal.isAgentNode(node(1, 4))).toBe(false);
  });

  it('falls back to claude-code when template missing', () => {
    expect(_internal.isAgentNode(node(1, 999))).toBe(true);
  });
});

describe('pipeline.buildSkillsContext', () => {
  beforeEach(() => {
    nodesStore.set([]);
    edgesStore.set([]);
    templatesStore.set([]);
    (meshMod.meshReadSkill as unknown as Fn).mockReset();
    (toastsMod.addToast as unknown as Fn).mockReset();
  });

  it('returns empty string when no incoming skill nodes', async () => {
    templatesStore.set([template(1, 'claude-code')]);
    nodesStore.set([node(10, 1, 'A')]);
    const ctx = await _internal.buildSkillsContext(10);
    expect(ctx).toBe('');
  });

  it('joins multiple skill bodies with separator', async () => {
    templatesStore.set([
      template(1, 'claude-code'),
      { ...template(2, 'skill'), prePrompt: 'slug-a' },
      { ...template(3, 'skill'), prePrompt: 'slug-b' },
    ]);
    nodesStore.set([node(10, 1, 'Agent'), node(20, 2, 'SkA'), node(21, 3, 'SkB')]);
    edgesStore.set([
      { id: 1, graphId: 1, fromNodeId: 20, toNodeId: 10 },
      { id: 2, graphId: 1, fromNodeId: 21, toNodeId: 10 },
    ]);
    (meshMod.meshReadSkill as unknown as Fn)
      .mockResolvedValueOnce({
        slug: 'slug-a',
        name: 'A',
        description: 'desc A',
        path: '/a',
        content: 'body A',
      })
      .mockResolvedValueOnce({
        slug: 'slug-b',
        name: 'B',
        description: 'desc B',
        path: '/b',
        content: 'body B',
      });
    const ctx = await _internal.buildSkillsContext(10);
    expect(ctx).toContain('## Available skills');
    expect(ctx).toContain('### Skill: A');
    expect(ctx).toContain('body A');
    expect(ctx).toContain('### Skill: B');
    expect(ctx).toContain('body B');
    expect(ctx).toContain('---');
  });

  it('throws when a skill fails to load — caller aborts the run', async () => {
    templatesStore.set([
      template(1, 'claude-code'),
      { ...template(2, 'skill'), prePrompt: 'broken' },
    ]);
    nodesStore.set([node(10, 1, 'Agent'), node(20, 2, 'Sk')]);
    edgesStore.set([{ id: 1, graphId: 1, fromNodeId: 20, toNodeId: 10 }]);
    (meshMod.meshReadSkill as unknown as Fn).mockRejectedValueOnce(new Error('boom'));
    await expect(_internal.buildSkillsContext(10)).rejects.toThrow(/boom/);
  });
});

function fakeRun(id = 99) {
  return {
    id,
    graphId: 1,
    entryNodeId: 0,
    initialPrompt: null,
    status: 'running' as const,
    maxDepth: 5,
    timeoutSecs: 300,
    maxLoopCount: 5,
    ombroEnabled: true,
    startedAt: '2026-04-24T00:00:00Z',
    finishedAt: null,
    createdAt: '2026-04-24T00:00:00Z',
  };
}

function activeState(
  overrides: Partial<{ runId: number; triggered: string[]; done: string[]; provider: string }> = {}
) {
  return {
    runId: overrides.runId ?? 99,
    graphId: 1,
    provider: overrides.provider ?? 'claude-code',
    cwd: '/x',
    triggered: new Set(overrides.triggered ?? ['10']),
    done: new Set(overrides.done ?? []),
  };
}

function graph(id: number, provider = 'claude-code') {
  return {
    id,
    floorId: 1,
    name: `G${id}`,
    entryNodeId: null,
    provider,
    createdAt: '',
    updatedAt: '',
  };
}

describe('pipeline note context', () => {
  function noteRow(id: number, name: string, content: string) {
    return {
      nodeId: id,
      graphId: 1,
      name,
      content,
      x: 0,
      y: 0,
      width: null,
      height: null,
      updatedAt: '',
    };
  }

  beforeEach(() => {
    nodesStore.set([]);
    edgesStore.set([]);
    templatesStore.set([]);
    notesStore.set([]);
  });

  it('returns empty when no incoming notes', () => {
    templatesStore.set([template(1, 'claude-code')]);
    nodesStore.set([node(10, 1)]);
    expect(_internal.collectIncomingNotes(10)).toEqual([]);
    expect(_internal.buildNotesContext(10)).toBe('');
  });

  it('walks note→agent in BFS order', () => {
    templatesStore.set([template(1, 'claude-code'), template(2, 'note')]);
    nodesStore.set([node(10, 1, 'Agent'), node(20, 2, 'NoteA'), node(21, 2, 'NoteB')]);
    edgesStore.set([
      { id: 1, graphId: 1, fromNodeId: 20, toNodeId: 10 },
      { id: 2, graphId: 1, fromNodeId: 21, toNodeId: 10 },
    ]);
    notesStore.set([noteRow(20, 'A', 'body A'), noteRow(21, 'B', 'body B')]);
    expect(_internal.collectIncomingNotes(10).sort()).toEqual([20, 21]);
    const ctx = _internal.buildNotesContext(10);
    expect(ctx).toContain('## Connected notes');
    expect(ctx).toContain('### Note: A');
    expect(ctx).toContain('body A');
    expect(ctx).toContain('### Note: B');
  });

  it('traverses note→note chain backwards from the agent', () => {
    templatesStore.set([template(1, 'claude-code'), template(2, 'note')]);
    nodesStore.set([
      node(10, 1, 'Agent'),
      node(20, 2, 'Root'),
      node(21, 2, 'Child'),
      node(22, 2, 'Grandchild'),
    ]);
    edgesStore.set([
      { id: 1, graphId: 1, fromNodeId: 20, toNodeId: 10 },
      { id: 2, graphId: 1, fromNodeId: 21, toNodeId: 20 },
      { id: 3, graphId: 1, fromNodeId: 22, toNodeId: 21 },
    ]);
    notesStore.set([noteRow(20, 'Root', 'r'), noteRow(21, 'Child', 'c'), noteRow(22, 'GC', 'g')]);
    const order = _internal.collectIncomingNotes(10);
    expect(order).toEqual([20, 21, 22]);
  });

  it('is cycle-safe', () => {
    templatesStore.set([template(1, 'claude-code'), template(2, 'note')]);
    nodesStore.set([node(10, 1), node(20, 2), node(21, 2)]);
    edgesStore.set([
      { id: 1, graphId: 1, fromNodeId: 20, toNodeId: 10 },
      { id: 2, graphId: 1, fromNodeId: 21, toNodeId: 20 },
      { id: 3, graphId: 1, fromNodeId: 20, toNodeId: 21 }, // cycle
    ]);
    expect(_internal.collectIncomingNotes(10).sort()).toEqual([20, 21]);
  });

  it('skips notes with empty content in the rendered context', () => {
    templatesStore.set([template(1, 'claude-code'), template(2, 'note')]);
    nodesStore.set([node(10, 1), node(20, 2)]);
    edgesStore.set([{ id: 1, graphId: 1, fromNodeId: 20, toNodeId: 10 }]);
    notesStore.set([noteRow(20, 'Empty', '   ')]);
    expect(_internal.collectIncomingNotes(10)).toEqual([20]);
    expect(_internal.buildNotesContext(10)).toBe('');
  });

  it('ignores incoming non-note nodes', () => {
    templatesStore.set([template(1, 'claude-code'), template(2, 'skill'), template(3, 'browser')]);
    nodesStore.set([node(10, 1), node(20, 2), node(21, 3)]);
    edgesStore.set([
      { id: 1, graphId: 1, fromNodeId: 20, toNodeId: 10 },
      { id: 2, graphId: 1, fromNodeId: 21, toNodeId: 10 },
    ]);
    expect(_internal.collectIncomingNotes(10)).toEqual([]);
  });
});

describe('pipeline.startPipeline', () => {
  beforeEach(() => {
    nodesStore.set([]);
    edgesStore.set([]);
    templatesStore.set([]);
    graphsStore.set([graph(1)]);
    meshNodeSessions.set({});
    meshNodeOutputs.set({});
    activePipeline.set(null);
    _internal.resetGlobalListeners();
    (tauriMod.createSession as unknown as Fn).mockReset();
    (tauriMod.onSessionStopped as unknown as Fn).mockReset();
    (tauriMod.onSessionError as unknown as Fn).mockReset();
    (tauriMod.onSessionStopped as unknown as Fn).mockResolvedValue(() => {});
    (tauriMod.onSessionError as unknown as Fn).mockResolvedValue(() => {});
    (meshMod.meshCreateRun as unknown as Fn).mockReset();
    (meshMod.meshFinishRun as unknown as Fn).mockReset();
    (meshMod.meshRecordRunSession as unknown as Fn).mockReset();
    (meshMod.meshFinishRun as unknown as Fn).mockResolvedValue(undefined);
    (meshMod.meshRecordRunSession as unknown as Fn).mockResolvedValue({
      id: 1,
      runId: 99,
      nodeId: 10,
      sessionId: 42,
    });
  });

  afterEach(() => {
    activePipeline.set(null);
  });

  it('throws when graph has no entry node', async () => {
    await expect(startPipeline(1, 'do something', '/tmp')).rejects.toThrow(/no entry node/);
    expect(meshMod.meshCreateRun).not.toHaveBeenCalled();
  });

  it('persists the run, spawns the entry node, and records the run_session', async () => {
    templatesStore.set([template(1, 'claude-code')]);
    nodesStore.set([node(10, 1, 'Entry')]);
    (meshMod.meshCreateRun as unknown as Fn).mockResolvedValueOnce(fakeRun(99));
    (tauriMod.createSession as unknown as Fn).mockResolvedValueOnce({ id: 42 });

    await startPipeline(1, 'task X', '/tmp/work');

    expect(meshMod.meshCreateRun).toHaveBeenCalledWith(1, 10, 'task X');
    expect(tauriMod.createSession).toHaveBeenCalledTimes(1);
    expect(meshMod.meshRecordRunSession).toHaveBeenCalledWith(99, 10, 42);
    const state = get(activePipeline);
    expect(state?.runId).toBe(99);
    expect(state?.triggered.has('10')).toBe(true);
    expect(get(meshNodeSessions)['10']).toBe(42);
  });
});

describe('pipeline.onSessionFailure', () => {
  beforeEach(() => {
    nodesStore.set([]);
    edgesStore.set([]);
    templatesStore.set([]);
    meshNodeSessions.set({});
    activePipeline.set(null);
    (toastsMod.addToast as unknown as Fn).mockReset();
    (meshMod.meshFinishRun as unknown as Fn).mockReset();
    (meshMod.meshFinishRun as unknown as Fn).mockResolvedValue(undefined);
  });

  it('aborts the active pipeline, toasts, and finishes the run as error', () => {
    templatesStore.set([template(1)]);
    nodesStore.set([node(10, 1, 'A')]);
    meshNodeSessions.set({ '10': 42 });
    activePipeline.set(activeState({ runId: 7 }));

    _internal.onSessionFailure(42, 'CLI crashed');

    expect(get(activePipeline)).toBeNull();
    expect(toastsMod.addToast).toHaveBeenCalledTimes(1);
    expect(meshMod.meshFinishRun).toHaveBeenCalledWith(7, 'error');
  });

  it('is a no-op when no active pipeline', () => {
    _internal.onSessionFailure(99, 'whatever');
    expect(get(activePipeline)).toBeNull();
    expect(toastsMod.addToast).not.toHaveBeenCalled();
    expect(meshMod.meshFinishRun).not.toHaveBeenCalled();
  });

  it('is a no-op when session id is not in the run', () => {
    activePipeline.set(activeState({ triggered: [], done: [] }));
    _internal.onSessionFailure(42, 'unrelated');
    expect(get(activePipeline)).not.toBeNull();
    expect(toastsMod.addToast).not.toHaveBeenCalled();
    expect(meshMod.meshFinishRun).not.toHaveBeenCalled();
  });
});

describe('pipeline.onSessionStop', () => {
  beforeEach(() => {
    nodesStore.set([]);
    edgesStore.set([]);
    templatesStore.set([]);
    meshNodeSessions.set({});
    meshNodeOutputs.set({});
    activePipeline.set(null);
    (tauriMod.createSession as unknown as Fn).mockReset();
    (toastsMod.addToast as unknown as Fn).mockReset();
    (meshMod.meshFinishRun as unknown as Fn).mockReset();
    (meshMod.meshRecordRunSession as unknown as Fn).mockReset();
    (meshMod.meshFinishRun as unknown as Fn).mockResolvedValue(undefined);
    (meshMod.meshRecordRunSession as unknown as Fn).mockResolvedValue({
      id: 1,
      runId: 99,
      nodeId: 0,
      sessionId: 0,
    });
  });

  it('marks node done, finishes run as completed when no downstream', async () => {
    templatesStore.set([template(1)]);
    nodesStore.set([node(10, 1, 'A')]);
    meshNodeSessions.set({ '10': 42 });
    activePipeline.set(activeState({ runId: 7, triggered: ['10'] }));

    await _internal.onSessionStop(42);

    expect(get(activePipeline)).toBeNull();
    expect(meshMod.meshFinishRun).toHaveBeenCalledWith(7, 'completed');
  });

  it('spawns each unique downstream agent node and records sessions', async () => {
    templatesStore.set([template(1)]);
    nodesStore.set([node(10, 1, 'A'), node(11, 1, 'B'), node(12, 1, 'C')]);
    edgesStore.set([
      { id: 1, graphId: 1, fromNodeId: 10, toNodeId: 11 },
      { id: 2, graphId: 1, fromNodeId: 10, toNodeId: 12 },
    ]);
    meshNodeSessions.set({ '10': 42 });
    meshNodeOutputs.set({ '10': 'output of A' });
    activePipeline.set(activeState({ runId: 7, triggered: ['10'] }));
    (tauriMod.createSession as unknown as Fn)
      .mockResolvedValueOnce({ id: 43 })
      .mockResolvedValueOnce({ id: 44 });

    await _internal.onSessionStop(42);

    expect(tauriMod.createSession).toHaveBeenCalledTimes(2);
    expect(meshMod.meshRecordRunSession).toHaveBeenCalledTimes(2);
    const state = get(activePipeline);
    expect(state?.triggered.has('11')).toBe(true);
    expect(state?.triggered.has('12')).toBe(true);
    expect(state?.done.has('10')).toBe(true);
  });

  it('skips already-triggered downstream', async () => {
    templatesStore.set([template(1)]);
    nodesStore.set([node(10, 1, 'A'), node(11, 1, 'B')]);
    edgesStore.set([{ id: 1, graphId: 1, fromNodeId: 10, toNodeId: 11 }]);
    meshNodeSessions.set({ '10': 42 });
    activePipeline.set(activeState({ runId: 7, triggered: ['10', '11'] }));

    await _internal.onSessionStop(42);

    expect(tauriMod.createSession).not.toHaveBeenCalled();
  });

  it('aborts pipeline and finishes run as error when downstream spawn fails', async () => {
    templatesStore.set([template(1)]);
    nodesStore.set([node(10, 1, 'A'), node(11, 1, 'B')]);
    edgesStore.set([{ id: 1, graphId: 1, fromNodeId: 10, toNodeId: 11 }]);
    meshNodeSessions.set({ '10': 42 });
    activePipeline.set(activeState({ runId: 7, triggered: ['10'] }));
    (tauriMod.createSession as unknown as Fn).mockRejectedValueOnce(new Error('spawn fail'));

    await _internal.onSessionStop(42);

    expect(get(activePipeline)).toBeNull();
    expect(toastsMod.addToast).toHaveBeenCalledTimes(1);
    expect(meshMod.meshFinishRun).toHaveBeenCalledWith(7, 'error');
  });

  it('is a no-op when no active pipeline', async () => {
    await _internal.onSessionStop(42);
    expect(get(activePipeline)).toBeNull();
  });
});

describe('pipeline.stopPipeline', () => {
  beforeEach(() => {
    meshNodeSessions.set({});
    activePipeline.set(null);
    (meshMod.meshFinishRun as unknown as Fn).mockReset();
    (meshMod.meshFinishRun as unknown as Fn).mockResolvedValue(undefined);
    (tauriMod.stopSession as unknown as Fn).mockReset();
    (tauriMod.stopSession as unknown as Fn).mockResolvedValue(undefined);
  });

  it('clears the active pipeline state and marks the run as stopped', () => {
    activePipeline.set(activeState({ runId: 7 }));
    stopPipeline();
    expect(get(activePipeline)).toBeNull();
    expect(meshMod.meshFinishRun).toHaveBeenCalledWith(7, 'stopped');
  });

  it('also kills every running agent in the pipeline', () => {
    meshNodeSessions.set({ '10': 42, '11': 43, '12': 44 });
    activePipeline.set(activeState({ runId: 7 }));
    stopPipeline();
    expect(tauriMod.stopSession).toHaveBeenCalledWith(42);
    expect(tauriMod.stopSession).toHaveBeenCalledWith(43);
    expect(tauriMod.stopSession).toHaveBeenCalledWith(44);
  });

  it('does nothing when there is no active pipeline', () => {
    activePipeline.set(null);
    stopPipeline();
    expect(meshMod.meshFinishRun).not.toHaveBeenCalled();
    expect(tauriMod.stopSession).not.toHaveBeenCalled();
  });
});

describe('pipeline.stopAllAgents', () => {
  beforeEach(() => {
    meshNodeSessions.set({});
    (tauriMod.stopSession as unknown as Fn).mockReset();
    (tauriMod.stopSession as unknown as Fn).mockResolvedValue(undefined);
  });

  it('calls stopSession for every bound session', async () => {
    meshNodeSessions.set({ a: 10, b: 11, c: 12 });
    await stopAllAgents();
    expect(tauriMod.stopSession).toHaveBeenCalledTimes(3);
  });

  it('is a no-op when no sessions are bound', async () => {
    await stopAllAgents();
    expect(tauriMod.stopSession).not.toHaveBeenCalled();
  });

  it('does not throw when one session fails to stop', async () => {
    meshNodeSessions.set({ a: 10, b: 11 });
    (tauriMod.stopSession as unknown as Fn).mockRejectedValueOnce(new Error('already dead'));
    await expect(stopAllAgents()).resolves.toBeUndefined();
  });
});

describe('pipeline.clearAgent', () => {
  beforeEach(() => {
    meshNodeSessions.set({});
    meshNodeOutputs.set({});
    (tauriMod.stopSession as unknown as Fn).mockReset();
    (tauriMod.deleteSession as unknown as Fn).mockReset();
    (tauriMod.stopSession as unknown as Fn).mockResolvedValue(undefined);
    (tauriMod.deleteSession as unknown as Fn).mockResolvedValue(undefined);
  });

  it('stops, deletes, and unbinds the bound session', async () => {
    meshNodeSessions.set({ '10': 42, '11': 43 });
    meshNodeOutputs.set({ '10': 'output A', '11': 'output B' });

    await clearAgent('10');

    expect(tauriMod.stopSession).toHaveBeenCalledWith(42);
    expect(tauriMod.deleteSession).toHaveBeenCalledWith(42);
    expect(get(meshNodeSessions)).toEqual({ '11': 43 });
    expect(get(meshNodeOutputs)).toEqual({ '11': 'output B' });
  });

  it('still unbinds when the session was already stopped', async () => {
    meshNodeSessions.set({ '10': 42 });
    (tauriMod.stopSession as unknown as Fn).mockRejectedValueOnce(new Error('not running'));

    await clearAgent('10');

    expect(tauriMod.deleteSession).toHaveBeenCalledWith(42);
    expect(get(meshNodeSessions)).toEqual({});
  });

  it('is a no-op when no session is bound to the node', async () => {
    await clearAgent('99');
    expect(tauriMod.stopSession).not.toHaveBeenCalled();
    expect(tauriMod.deleteSession).not.toHaveBeenCalled();
  });
});

describe('pipeline.clearAllAgents', () => {
  beforeEach(() => {
    meshNodeSessions.set({});
    meshNodeOutputs.set({});
    activePipeline.set(null);
    (tauriMod.stopSession as unknown as Fn).mockReset();
    (tauriMod.deleteSession as unknown as Fn).mockReset();
    (meshMod.meshFinishRun as unknown as Fn).mockReset();
    (tauriMod.stopSession as unknown as Fn).mockResolvedValue(undefined);
    (tauriMod.deleteSession as unknown as Fn).mockResolvedValue(undefined);
    (meshMod.meshFinishRun as unknown as Fn).mockResolvedValue(undefined);
  });

  it('clears every bound session and resets the stores', async () => {
    meshNodeSessions.set({ a: 10, b: 11 });
    meshNodeOutputs.set({ a: 'x', b: 'y' });

    await clearAllAgents();

    expect(tauriMod.stopSession).toHaveBeenCalledWith(10);
    expect(tauriMod.stopSession).toHaveBeenCalledWith(11);
    expect(tauriMod.deleteSession).toHaveBeenCalledWith(10);
    expect(tauriMod.deleteSession).toHaveBeenCalledWith(11);
    expect(get(meshNodeSessions)).toEqual({});
    expect(get(meshNodeOutputs)).toEqual({});
  });

  it('aborts the active pipeline before clearing', async () => {
    meshNodeSessions.set({ a: 10 });
    activePipeline.set(activeState({ runId: 7 }));

    await clearAllAgents();

    expect(meshMod.meshFinishRun).toHaveBeenCalledWith(7, 'stopped');
    expect(get(activePipeline)).toBeNull();
  });

  it('is a no-op when nothing is bound', async () => {
    await clearAllAgents();
    expect(tauriMod.stopSession).not.toHaveBeenCalled();
    expect(tauriMod.deleteSession).not.toHaveBeenCalled();
  });
});
