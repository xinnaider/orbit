<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { get } from 'svelte/store';
  import { Handle, Position, NodeResizer, useSvelteFlow } from '@xyflow/svelte';
  import {
    createSession,
    stopSession,
    onSessionOutput,
    onSessionRunning,
    onSessionStopped,
    onSessionError,
    sendSessionMessage,
    getSessionJournal,
    listSessions,
  } from '../../lib/tauri';
  import { meshNodeSessions } from '../../lib/stores/mesh/node-sessions';
  import { recordNodeOutput } from '../../lib/stores/mesh/node-outputs';
  import {
    MESH_DEFAULT_PERMISSION_MODE,
    MESH_DEFAULT_PROVIDER,
    MESH_NODE_DEFAULT_SIZE,
    MESH_NODE_MAX_SIZE,
    MESH_SESSION_PREFIX,
  } from '../../lib/stores/mesh/constants';
  import { resizeNode as persistResize } from '../../lib/stores/mesh/graph';
  import { clearAgent } from '../../lib/stores/mesh/pipeline';
  import { addToast } from '../../lib/stores/toasts';
  import InputPromptModal from './InputPromptModal.svelte';
  import Feed from '../Feed.svelte';
  import type { JournalEntry } from '../../lib/types';

  const { updateNode, getNode } = useSvelteFlow();

  type AgentNodeData = {
    label: string;
    templateName: string;
    prePrompt: string;
    model: string | null;
    useWorktree: boolean;
  };

  export let id: string;
  export let data: AgentNodeData;

  let sessionId: number | null = null;
  let status: 'idle' | 'starting' | 'running' | 'stopped' | 'error' = 'idle';
  let entries: JournalEntry[] = [];
  let inputText = '';
  let unlisteners: Array<() => void> = [];
  let cwdPromptOpen = false;

  let maximized = false;
  let preMaxSize: { width: number; height: number } | null = null;
  const MAX_W = MESH_NODE_MAX_SIZE.width;
  const MAX_H = MESH_NODE_MAX_SIZE.height;
  const DEFAULT_W = MESH_NODE_DEFAULT_SIZE.agent.width;
  const DEFAULT_H = MESH_NODE_DEFAULT_SIZE.agent.height;

  async function toggleMaximize() {
    const current = getNode(id);
    if (!current) return;
    if (!maximized) {
      preMaxSize = {
        width: Number(current.width ?? DEFAULT_W),
        height: Number(current.height ?? DEFAULT_H),
      };
      updateNode(id, { width: MAX_W, height: MAX_H });
      maximized = true;
      try {
        await persistResize(Number(id), MAX_W, MAX_H);
      } catch (e) {
        addToast({ type: 'error', message: `failed to persist size: ${e}`, autoDismiss: true });
      }
    } else {
      // After reload preMaxSize is null — fall back to defaults.
      const restoreW = preMaxSize?.width ?? DEFAULT_W;
      const restoreH = preMaxSize?.height ?? DEFAULT_H;
      updateNode(id, { width: restoreW, height: restoreH });
      maximized = false;
      preMaxSize = null;
      try {
        await persistResize(Number(id), restoreW, restoreH);
      } catch (e) {
        addToast({ type: 'error', message: `failed to persist size: ${e}`, autoDismiss: true });
      }
    }
  }

  async function onResizeEnd(_e: unknown, params: { width: number; height: number }) {
    if (maximized) return;
    try {
      await persistResize(Number(id), params.width, params.height);
    } catch (e) {
      addToast({ type: 'error', message: `failed to persist size: ${e}`, autoDismiss: true });
    }
  }

  $: {
    const map = $meshNodeSessions;
    const nextSid = map[id];
    if (nextSid !== undefined && nextSid !== sessionId) {
      const isRehydrate = sessionId === null;
      sessionId = nextSid;
      entries = [];
      // Status is set by hydrateFromJournal during rehydration.
      if (!isRehydrate && (status === 'idle' || status === 'stopped' || status === 'error')) {
        status = 'starting';
      }
      if (isRehydrate) {
        void hydrateFromJournal(nextSid);
      }
    } else if (nextSid === undefined && sessionId !== null) {
      sessionId = null;
      status = 'idle';
      entries = [];
    }
  }

  async function hydrateFromJournal(sid: number) {
    try {
      const [journal, allSessions] = await Promise.all([getSessionJournal(sid), listSessions()]);
      entries = journal.slice(-200);
      const s = allSessions.find((x) => x.id === sid);
      if (s) {
        if (s.status === 'running' || s.status === 'initializing' || s.status === 'waiting') {
          status = 'running';
        } else if (s.status === 'error') {
          status = 'error';
        } else {
          status = 'stopped';
        }
      } else {
        status = 'stopped';
      }
    } catch (e) {
      console.warn('[mesh] hydrateFromJournal failed', e);
    }
  }

  function updateOutputStore(entry: JournalEntry) {
    if (entry.entryType === 'assistant' && entry.text && entry.text.trim().length > 0) {
      recordNodeOutput(id, entry.text);
    }
  }

  function pushEntry(e: JournalEntry) {
    entries = [...entries, e].slice(-200);
  }

  function makeLocalEntry(entryType: JournalEntry['entryType'], text: string): JournalEntry {
    return {
      sessionId: sessionId !== null ? String(sessionId) : '',
      timestamp: new Date().toISOString(),
      entryType,
      text,
      thinking: null,
      thinkingDuration: null,
      tool: null,
      toolInput: null,
      output: null,
      exitCode: null,
      linesChanged: null,
      seq: entries.length,
      epoch: '',
    };
  }

  // Closures over local `sessionId` don't rebind in external listener callbacks.
  function currentSid(): number | undefined {
    return get(meshNodeSessions)[id];
  }

  async function setup() {
    const unlOut = await onSessionOutput((p) => {
      if (p.sessionId !== currentSid()) return;
      pushEntry(p.entry);
      updateOutputStore(p.entry);
    });
    const unlRun = await onSessionRunning((sid) => {
      if (sid !== currentSid()) return;
      status = 'running';
    });
    const unlStop = await onSessionStopped((sid) => {
      if (sid !== currentSid()) return;
      status = 'stopped';
    });
    const unlErr = await onSessionError((sid, err) => {
      if (sid !== currentSid()) return;
      status = 'error';
      addToast({ type: 'error', message: `${data.label}: ${err}`, autoDismiss: true });
      // Surface error inline in feed too
      pushEntry(makeLocalEntry('system', err));
    });
    unlisteners = [unlOut, unlRun, unlStop, unlErr];
  }

  // Edge case: a manual resize to exactly MAX_W×MAX_H reads as maximized.
  async function detectInitialMaximize() {
    await tick();
    const current = getNode(id);
    if (current && Number(current.width) === MAX_W && Number(current.height) === MAX_H) {
      maximized = true;
    }
  }

  onMount(async () => {
    await setup();
    await detectInitialMaximize();
  });
  onDestroy(() => unlisteners.forEach((u) => u()));

  async function startWithCwd(cwd: string) {
    status = 'starting';
    entries = [];
    try {
      const session = await createSession({
        projectPath: cwd,
        prompt: data.prePrompt,
        model: data.model ?? undefined,
        permissionMode: MESH_DEFAULT_PERMISSION_MODE,
        sessionName: `${MESH_SESSION_PREFIX}${data.label}`,
        useWorktree: data.useWorktree,
      });
      sessionId = session.id;
      meshNodeSessions.update((m) => ({ ...m, [id]: session.id }));
    } catch (e) {
      status = 'error';
      addToast({ type: 'error', message: `failed to start: ${e}`, autoDismiss: true });
    }
  }

  async function onStart() {
    if (status === 'running' || status === 'starting') return;

    const cwd = localStorage.getItem('mesh:cwd');
    if (!cwd) {
      cwdPromptOpen = true;
      return;
    }
    await startWithCwd(cwd);
  }

  async function onCwdSubmit(e: CustomEvent<string>) {
    const cwd = e.detail;
    localStorage.setItem('mesh:cwd', cwd);
    cwdPromptOpen = false;
    await startWithCwd(cwd);
  }

  async function onStop() {
    if (sessionId === null) return;
    try {
      await stopSession(sessionId);
    } catch (e) {
      addToast({ type: 'error', message: `failed to stop: ${e}`, autoDismiss: true });
    }
  }

  async function onClear() {
    try {
      await clearAgent(id);
      sessionId = null;
      status = 'idle';
      entries = [];
    } catch (e) {
      addToast({ type: 'error', message: `failed to clear context: ${e}`, autoDismiss: true });
    }
  }

  async function onSendMessage() {
    if (sessionId === null || !inputText.trim()) return;
    const msg = inputText.trim();
    inputText = '';
    pushEntry(makeLocalEntry('user', msg));
    try {
      await sendSessionMessage(sessionId, msg);
    } catch (e) {
      addToast({ type: 'error', message: `failed to send: ${e}`, autoDismiss: true });
    }
  }
</script>

<div
  class="agent-node"
  class:running={status === 'running'}
  class:starting={status === 'starting'}
  class:error={status === 'error'}
>
  <NodeResizer
    minWidth={260}
    minHeight={200}
    lineClass="resize-line"
    handleClass="resize-handle"
    {onResizeEnd}
  />
  <!-- Loose mode lets a source handle also receive a drop, so one per side. -->
  <Handle id="top" type="source" position={Position.Top} />
  <Handle id="left" type="source" position={Position.Left} />
  <Handle id="right" type="source" position={Position.Right} />
  <Handle id="bottom" type="source" position={Position.Bottom} />
  <header class="node-header">
    <div class="node-title">
      <span class="status-dot status-{status}" title={status}></span>
      <strong>{data.label}</strong>
      <span class="template-badge">{data.templateName}</span>
    </div>
    <div class="node-actions">
      {#if sessionId === null || status === 'stopped' || status === 'error'}
        <button on:click={onStart} class="btn-start" title="start" aria-label="start">
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <polygon points="3,2 10,6 3,10" fill="currentColor" />
          </svg>
        </button>
      {:else if status === 'running' || status === 'starting'}
        <button on:click={onStop} class="btn-stop" title="stop" aria-label="stop">
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <rect x="3" y="3" width="6" height="6" fill="currentColor" />
          </svg>
        </button>
      {/if}
      {#if sessionId !== null}
        <button
          on:click={onClear}
          class="btn-clear"
          title="clear context"
          aria-label="clear context"
        >
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path
              d="M10.5 2.5 V5 H8 M10 5 A4 4 0 1 0 8.5 8.5"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </button>
      {/if}
      <button
        on:click={toggleMaximize}
        class="btn-maximize"
        title={maximized ? 'minimize' : 'maximize'}
        aria-label={maximized ? 'minimize' : 'maximize'}
      >
        {#if maximized}
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path
              d="M5 1 V5 H1 M11 5 H7 V1 M1 7 H5 V11 M7 11 V7 H11"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="square"
            />
          </svg>
        {:else}
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path
              d="M1 4 V1 H4 M8 1 H11 V4 M11 8 V11 H8 M4 11 H1 V8"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="square"
            />
          </svg>
        {/if}
      </button>
    </div>
  </header>

  <div class="terminal">
    {#if entries.length === 0}
      <div class="placeholder">
        <div class="pre-prompt">{data.prePrompt}</div>
        <div class="hint">
          {#if status === 'idle'}click start to begin{:else if status === 'starting'}starting…{:else if status === 'stopped'}session
            finished{:else}waiting for output…{/if}
        </div>
      </div>
    {:else}
      <Feed {entries} {status} provider={MESH_DEFAULT_PROVIDER} cwd={null} />
    {/if}
  </div>

  {#if status === 'running'}
    <footer class="node-footer">
      <input
        type="text"
        bind:value={inputText}
        on:keydown={(e) =>
          e.key === 'Enter' && !e.shiftKey && (e.preventDefault(), onSendMessage())}
        placeholder="message…"
      />
      <button
        on:click={onSendMessage}
        class="btn-send"
        disabled={!inputText.trim()}
        aria-label="send"
      >
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path
            d="M2 6 H10 M7 3 L10 6 L7 9"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    </footer>
  {/if}
</div>

{#if cwdPromptOpen}
  <InputPromptModal
    title="project path"
    label="absolute cwd where the agents will work"
    placeholder="/absolute/path/to/project"
    confirmLabel="start"
    on:submit={onCwdSubmit}
    on:cancel={() => (cwdPromptOpen = false)}
  />
{/if}

<style>
  .agent-node {
    position: relative;
    width: 100%;
    height: 100%;
    min-width: 260px;
    min-height: 200px;
    background: var(--bg);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: var(--mono);
    color: var(--t0);
  }

  .agent-node.running {
    border-color: var(--ac);
    box-shadow: 0 0 0 1px var(--ac-d);
  }
  .agent-node.starting {
    border-color: var(--s-input);
  }
  .agent-node.error {
    border-color: var(--s-error);
  }

  .node-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-3) var(--sp-5);
    background: var(--bg1);
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
  }

  .node-title {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    overflow: hidden;
    font-size: var(--sm);
  }
  .node-title strong {
    font-size: var(--base);
  }

  .template-badge {
    font-size: var(--xs);
    color: var(--ac);
    background: var(--ac-d);
    padding: 1px var(--sp-3);
    border-radius: 10px;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--s-idle);
    display: inline-block;
  }
  .status-dot.status-running {
    background: var(--s-working);
    box-shadow: 0 0 6px var(--s-working);
    animation: pulse 1.5s infinite;
  }
  .status-dot.status-starting {
    background: var(--s-input);
    animation: pulse 0.6s infinite;
  }
  .status-dot.status-error {
    background: var(--s-error);
  }
  .status-dot.status-stopped {
    background: var(--s-done);
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
  }

  .node-actions {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }

  .node-actions button {
    background: transparent;
    border: 1px solid var(--bd1);
    color: var(--t0);
    width: 22px;
    height: 22px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    padding: 0;
    font-family: inherit;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .node-actions button svg {
    width: 12px;
    height: 12px;
    display: block;
  }

  .node-actions .btn-start:hover {
    background: var(--ac-d);
    border-color: var(--ac);
    color: var(--ac);
  }
  .node-actions .btn-stop:hover {
    background: rgba(224, 72, 72, 0.1);
    border-color: var(--s-error);
    color: var(--s-error);
  }
  .node-actions .btn-clear {
    color: var(--t1);
  }
  .node-actions .btn-clear:hover {
    background: var(--bg3);
    border-color: var(--s-input);
    color: var(--s-input);
  }
  .node-actions .btn-maximize {
    color: var(--t1);
  }
  .node-actions .btn-maximize:hover {
    background: var(--bg3);
    color: var(--ac);
    border-color: var(--ac);
  }

  :global(.svelte-flow__node-agent .resize-handle) {
    width: 16px;
    height: 16px;
    background: var(--ac);
    border: 2px solid var(--bg);
    border-radius: 3px;
  }
  :global(.svelte-flow__node-agent .resize-line) {
    border-color: var(--ac);
    border-width: 3px;
  }

  .terminal {
    flex: 1;
    /* min-height: 0 lets the flex child shrink so overflow-y can scroll. */
    min-height: 0;
    overflow-y: auto;
    padding: var(--sp-4) var(--sp-5);
    background: var(--bg);
    font-family: var(--mono);
    font-size: var(--xs);
    line-height: 1.5;
  }

  .terminal .placeholder {
    color: var(--t2);
    font-style: italic;
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  .pre-prompt {
    white-space: pre-wrap;
    font-size: var(--xs);
    color: var(--t1);
    font-family: var(--mono);
    font-style: normal;
  }
  .hint {
    font-size: var(--xs);
    color: var(--t2);
  }

  .node-footer {
    display: flex;
    gap: var(--sp-2);
    padding: var(--sp-3);
    background: var(--bg1);
    border-top: 1px solid var(--bd);
    flex-shrink: 0;
  }
  .node-footer input {
    flex: 1;
    background: var(--bg);
    border: 1px solid var(--bd1);
    color: var(--t0);
    padding: var(--sp-2) var(--sp-4);
    font-size: var(--xs);
    border-radius: var(--radius-sm);
    font-family: inherit;
    outline: none;
  }
  .node-footer input:focus {
    border-color: var(--ac);
  }
  .node-footer .btn-send {
    background: var(--ac);
    color: #000;
    border: none;
    padding: var(--sp-2) var(--sp-5);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-family: inherit;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .node-footer .btn-send svg {
    width: 12px;
    height: 12px;
    display: block;
  }
  .node-footer .btn-send:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  :global(.svelte-flow__node-agent .svelte-flow__handle) {
    background: var(--ac);
    width: 24px;
    height: 24px;
    border: 2px solid var(--bg);
    z-index: 10;
  }
  :global(.svelte-flow__node-agent .svelte-flow__handle:hover) {
    background: var(--s-input);
  }
</style>
