<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import {
    SvelteFlow,
    Background,
    Controls,
    ConnectionMode,
    type Node as FlowNode,
    type Edge as FlowEdge,
    type Edge,
    type Connection,
    type Viewport,
  } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import AgentNode from './AgentNode.svelte';
  import BrowserNode from './BrowserNode.svelte';
  import SkillNode from './SkillNode.svelte';
  import NoteNode from './NoteNode.svelte';

  const nodeTypes = {
    agent: AgentNode,
    browser: BrowserNode,
    skill: SkillNode,
    note: NoteNode,
  };

  import {
    nodesStore,
    edgesStore,
    activeGraphId,
    loadGraph,
    createNode,
    deleteNode,
    createEdge,
    deleteEdge,
    relocateNode,
  } from '../../lib/stores/mesh/graph';
  import { loadTemplates, templatesStore } from '../../lib/stores/mesh/templates';
  import { activeFloorId } from '../../lib/stores/mesh/floors';
  import { changeGraphProvider, graphsStore } from '../../lib/stores/mesh/graphs';
  import { addNote, loadNotes, notesStore, unbindNote } from '../../lib/stores/mesh/notes';
  import {
    isMeshSupportedProvider,
    isNoteTemplate,
    MESH_DEFAULT_PROVIDER,
    MESH_NODE_DEFAULT_SIZE,
    MESH_NOTE_PROVIDER,
  } from '../../lib/stores/mesh/constants';
  import { backends } from '../../lib/stores/providers';
  import {
    activePipeline,
    clearAllAgents,
    startPipeline,
    stopAllAgents,
    stopPipeline,
  } from '../../lib/stores/mesh/pipeline';
  import { meshNodeSessions } from '../../lib/stores/mesh/node-sessions';
  import { addToast } from '../../lib/stores/toasts';
  import InputPromptModal from './InputPromptModal.svelte';
  import ConfirmModal from './ConfirmModal.svelte';
  import KindIcon from './KindIcon.svelte';

  let flowNodes: FlowNode[] = [];
  let flowEdges: FlowEdge[] = [];
  let viewport: Viewport = { x: 0, y: 0, zoom: 1 };

  let selectedTemplateId: number | null = null;
  // Tray drag-to-canvas: the next drop creates a fresh note (no template).
  let creatingNote = false;
  let graphName = '';
  let graphProvider = MESH_DEFAULT_PROVIDER;
  let providerPickerOpen = false;

  $: if ($activeGraphId !== null) {
    const g = $graphsStore.find((g) => g.id === $activeGraphId);
    graphName = g?.name ?? '';
    graphProvider = g?.provider ?? MESH_DEFAULT_PROVIDER;
    reloadCanvas($activeGraphId);
  }

  $: providerLabel = $backends.find((b) => b.id === graphProvider)?.name ?? graphProvider;

  async function onPickProvider(providerId: string) {
    providerPickerOpen = false;
    if ($activeGraphId === null) return;
    if (providerId === graphProvider) return;
    try {
      await changeGraphProvider($activeGraphId, providerId);
    } catch (e) {
      addToast({ type: 'error', message: `failed to switch provider: ${e}`, autoDismiss: true });
    }
  }

  async function reloadCanvas(graphId: number) {
    await loadGraph(graphId);
    try {
      await loadNotes(graphId);
    } catch (e) {
      addToast({ type: 'error', message: `failed to load notes: ${e}`, autoDismiss: true });
    }
    syncToFlow();
  }

  function syncToFlow() {
    const templates = get(templatesStore);
    const tMap = new Map(templates.map((t) => [t.id, t]));
    // Local size (NodeResizer drag, Maximize toggle) wins over DB; DB only
    // fills in on initial mount when `prevSizes` is empty.
    const prevSizes = new Map(flowNodes.map((n) => [n.id, { width: n.width, height: n.height }]));
    flowNodes = get(nodesStore).map((n) => {
      const t = tMap.get(n.templateId);
      const kind: 'agent' | 'browser' | 'skill' | 'note' =
        t?.provider === MESH_NOTE_PROVIDER
          ? 'note'
          : t?.provider === 'browser'
            ? 'browser'
            : t?.provider === 'skill'
              ? 'skill'
              : 'agent';
      const prev = prevSizes.get(String(n.id));
      const defaultW = MESH_NODE_DEFAULT_SIZE[kind].width;
      const defaultH = MESH_NODE_DEFAULT_SIZE[kind].height;
      return {
        id: String(n.id),
        type: kind,
        position: { x: n.x, y: n.y },
        width: prev?.width ?? n.width ?? defaultW,
        height: prev?.height ?? n.height ?? defaultH,
        data: {
          label: n.displayName,
          templateName: t?.name ?? '???',
          prePrompt: t?.prePrompt ?? '',
          model: t?.model ?? null,
          useWorktree: t?.useWorktree ?? true,
        },
      };
    });
    flowEdges = get(edgesStore).map((e) => ({
      id: String(e.id),
      source: String(e.fromNodeId),
      target: String(e.toNodeId),
      sourceHandle: e.fromHandle ?? undefined,
      targetHandle: e.toHandle ?? undefined,
      type: 'default',
      animated: false,
    }));
  }

  // Re-sync when nodes/edges change externally (e.g. after create/delete)
  $: $nodesStore && $edgesStore && $templatesStore && syncToFlow();

  async function onDropTemplate(e: DragEvent) {
    e.preventDefault();
    if ($activeGraphId === null) return;
    if ($activePipeline !== null) {
      addToast({
        type: 'error',
        message: 'cannot add nodes while the pipeline is running',
        autoDismiss: true,
      });
      return;
    }

    // Drop coords are in screen space; convert to flow space for pan/zoom.
    const bounds = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const screenX = e.clientX - bounds.left;
    const screenY = e.clientY - bounds.top;
    const x = (screenX - viewport.x) / viewport.zoom;
    const y = (screenY - viewport.y) / viewport.zoom;

    if (creatingNote) {
      creatingNote = false;
      try {
        await addNote($activeGraphId, 'Note', x, y);
        // loadGraph re-fetches nodes; loadTemplates pulls the lazily-created
        // system note template so syncToFlow can resolve it (otherwise the
        // node falls back to kind='agent' and renders wrong).
        const fid = $activeFloorId;
        await Promise.all([
          loadGraph($activeGraphId),
          fid !== null ? loadTemplates(fid) : Promise.resolve(),
        ]);
      } catch (err) {
        addToast({ type: 'error', message: `failed to add note: ${err}`, autoDismiss: true });
      }
      return;
    }

    if (selectedTemplateId === null) return;
    const t = $templatesStore.find((t) => t.id === selectedTemplateId);
    if (!t) return;

    // Derive a unique display_name within the graph
    const existing = new Set(get(nodesStore).map((n) => n.displayName));
    let base = t.name;
    let n = 1;
    let candidate = base;
    while (existing.has(candidate)) {
      n++;
      candidate = `${base} ${n}`;
    }

    await createNode($activeGraphId, t.id, candidate, x, y);
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
  }

  function onTemplateDragStart(t: { id: number }, e: DragEvent) {
    selectedTemplateId = t.id;
    creatingNote = false;
    e.dataTransfer?.setData('text/plain', String(t.id));
  }

  function onNoteTrayDragStart(e: DragEvent) {
    creatingNote = true;
    selectedTemplateId = null;
    e.dataTransfer?.setData('text/plain', 'note');
  }

  function kindOfNode(nodeId: string): 'agent' | 'browser' | 'skill' | 'note' | null {
    const n = flowNodes.find((x) => x.id === nodeId);
    const t = (n?.type ?? null) as 'agent' | 'browser' | 'skill' | 'note' | null;
    return t;
  }

  // Hard rule that runs before SvelteFlow commits — false hides preview too.
  function isValidConnection(_connection: Edge | Connection): boolean {
    if ($activePipeline !== null) return false;
    return true;
  }

  async function onConnect(connection: Connection) {
    const { source, target, sourceHandle, targetHandle } = connection;
    if (!source || !target || $activeGraphId === null) return;
    if ($activePipeline !== null) {
      addToast({
        type: 'error',
        message: 'cannot add edges while the pipeline is running',
        autoDismiss: true,
      });
      return;
    }

    let srcKind = kindOfNode(source);
    let tgtKind = kindOfNode(target);
    let from = source;
    let to = target;
    let fromHandle = sourceHandle ?? null;
    let toHandle = targetHandle ?? null;

    // Storage direction is always skill/note → agent (their body is injected
    // into the agent's prompt), regardless of which way the user dragged.
    if (srcKind === 'agent' && (tgtKind === 'skill' || tgtKind === 'note')) {
      [from, to] = [to, from];
      [srcKind, tgtKind] = [tgtKind, srcKind];
      [fromHandle, toHandle] = [toHandle, fromHandle];
    }

    // Skills can only feed agents.
    if (srcKind === 'skill' && tgtKind !== 'agent') {
      addToast({
        type: 'error',
        message: 'skills can only link to agents',
        autoDismiss: true,
      });
      return;
    }

    // Notes can feed agents OR other notes (chain/mind-map).
    if (srcKind === 'note' && tgtKind !== 'agent' && tgtKind !== 'note') {
      addToast({
        type: 'error',
        message: 'notes can only link to agents or other notes',
        autoDismiss: true,
      });
      return;
    }

    try {
      await createEdge($activeGraphId, Number(from), Number(to), fromHandle, toHandle);
    } catch (err) {
      addToast({ type: 'error', message: `failed to create edge: ${err}`, autoDismiss: true });
    }
  }

  let confirmDlg: {
    title: string;
    message: string;
    onConfirm: () => Promise<void> | void;
  } | null = null;

  function onEdgeClick({ edge }: { edge: FlowEdge; event: MouseEvent }) {
    if ($activePipeline !== null) {
      addToast({
        type: 'error',
        message: 'cannot delete edges while the pipeline is running',
        autoDismiss: true,
      });
      return;
    }
    confirmDlg = {
      title: 'delete edge',
      message: 'this edge will be removed from the graph.',
      onConfirm: async () => {
        confirmDlg = null;
        try {
          await deleteEdge(Number(edge.id));
        } catch (e) {
          addToast({ type: 'error', message: `failed to delete edge: ${e}`, autoDismiss: true });
        }
      },
    };
  }

  async function onNodeDragStop({
    nodes,
  }: {
    event: MouseEvent | TouchEvent;
    targetNode: FlowNode | null;
    nodes: FlowNode[];
  }) {
    for (const n of nodes) {
      try {
        await relocateNode(Number(n.id), n.position.x, n.position.y);
      } catch (e) {
        addToast({ type: 'error', message: `failed to move node: ${e}`, autoDismiss: true });
      }
    }
  }

  function onNodeDelete(id: string) {
    if ($activePipeline !== null) {
      addToast({
        type: 'error',
        message: 'cannot delete nodes while the pipeline is running',
        autoDismiss: true,
      });
      return;
    }
    confirmDlg = {
      title: 'delete node',
      message: 'the node and its edges will be removed.',
      onConfirm: async () => {
        confirmDlg = null;
        try {
          await deleteNode(Number(id));
        } catch (e) {
          addToast({ type: 'error', message: `failed to delete node: ${e}`, autoDismiss: true });
        }
      },
    };
  }

  let contextMenu: { x: number; y: number; nodeId: string; label: string } | null = null;

  function onNodeContextMenu({ event, node }: { event: MouseEvent; node: FlowNode }) {
    event.preventDefault();
    const label = typeof node.data?.label === 'string' ? (node.data.label as string) : 'this node';
    contextMenu = { x: event.clientX, y: event.clientY, nodeId: node.id, label };
  }

  // SvelteFlow's built-in delete is disabled via `deleteKeyCode={null}` so
  // every removal goes through ConfirmModal.
  function deleteSelection() {
    if ($activePipeline !== null) {
      addToast({
        type: 'error',
        message: 'cannot delete while the pipeline is running',
        autoDismiss: true,
      });
      return;
    }
    const selectedNodes = flowNodes.filter((n) => n.selected);
    const selectedEdges = flowEdges.filter((e) => e.selected);
    if (selectedNodes.length === 0 && selectedEdges.length === 0) return;

    if (selectedNodes.length === 1 && selectedEdges.length === 0) {
      onNodeDelete(selectedNodes[0].id);
      return;
    }

    const parts: string[] = [];
    if (selectedNodes.length > 0)
      parts.push(`${selectedNodes.length} node${selectedNodes.length > 1 ? 's' : ''}`);
    if (selectedEdges.length > 0)
      parts.push(`${selectedEdges.length} edge${selectedEdges.length > 1 ? 's' : ''}`);

    confirmDlg = {
      title: `delete ${parts.join(' + ')}`,
      message: 'the selected items (and any edges attached to deleted nodes) will be removed.',
      onConfirm: async () => {
        confirmDlg = null;
        // Delete nodes first — their edge cascades clean up dangling edges,
        // then we remove any explicitly-selected edges still standing.
        for (const n of selectedNodes) {
          try {
            await deleteNode(Number(n.id));
          } catch (e) {
            addToast({ type: 'error', message: `failed to delete node: ${e}`, autoDismiss: true });
          }
        }
        for (const e of selectedEdges) {
          try {
            await deleteEdge(Number(e.id));
          } catch (err) {
            addToast({
              type: 'error',
              message: `failed to delete edge: ${err}`,
              autoDismiss: true,
            });
          }
        }
      },
    };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function onContextMenuDelete() {
    if (!contextMenu) return;
    const id = contextMenu.nodeId;
    contextMenu = null;
    onNodeDelete(id);
  }

  function onWindowKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && contextMenu) {
      contextMenu = null;
      return;
    }
    // Skip when the user is typing in an input — Backspace would steal the keystroke.
    if (e.key === 'Delete' || e.key === 'Backspace') {
      const target = e.target as HTMLElement | null;
      const tag = target?.tagName?.toLowerCase();
      const isEditable =
        tag === 'input' || tag === 'textarea' || target?.isContentEditable === true;
      if (isEditable) return;
      if ($activeGraphId === null) return;
      e.preventDefault();
      deleteSelection();
    }
  }

  function backToFloor() {
    activeGraphId.set(null);
  }

  let taskText = '';
  let cwdPromptOpen = false;

  // Any agent in this graph that currently has a session bound (running or
  // stopped) — used to gate the global "stop all" / "clear all" buttons.
  $: hasAnyBoundSession = Object.keys($meshNodeSessions).length > 0;

  function onStopAll() {
    confirmDlg = {
      title: 'stop all agents',
      message: 'every running agent in this graph will be killed.',
      onConfirm: async () => {
        confirmDlg = null;
        try {
          await stopAllAgents();
        } catch (e) {
          addToast({ type: 'error', message: `failed to stop all: ${e}`, autoDismiss: true });
        }
      },
    };
  }

  function onClearAll() {
    confirmDlg = {
      title: 'clear all agents',
      message: 'every agent in this graph will be stopped and reset. accumulated context is lost.',
      onConfirm: async () => {
        confirmDlg = null;
        try {
          await clearAllAgents();
        } catch (e) {
          addToast({ type: 'error', message: `failed to clear all: ${e}`, autoDismiss: true });
        }
      },
    };
  }

  async function runPipeline(cwd: string) {
    if ($activeGraphId === null) return;
    try {
      await startPipeline($activeGraphId, taskText.trim(), cwd);
    } catch (e) {
      addToast({ type: 'error', message: String(e), autoDismiss: true });
    }
  }

  function onStartPipeline() {
    const trimmed = taskText.trim();
    if (!trimmed) {
      addToast({ type: 'error', message: 'enter the initial task first', autoDismiss: true });
      return;
    }
    const cwd = localStorage.getItem('mesh:cwd');
    if (!cwd) {
      cwdPromptOpen = true;
      return;
    }
    runPipeline(cwd);
  }

  async function onCwdSubmit(e: CustomEvent<string>) {
    const cwd = e.detail;
    localStorage.setItem('mesh:cwd', cwd);
    cwdPromptOpen = false;
    await runPipeline(cwd);
  }

  $: trayAgents = $templatesStore.filter(
    (t) => t.provider !== 'browser' && t.provider !== 'skill' && t.provider !== MESH_NOTE_PROVIDER
  );
  $: trayBrowsers = $templatesStore.filter((t) => t.provider === 'browser');
  $: traySkills = $templatesStore.filter((t) => t.provider === 'skill');
</script>

<svelte:window on:keydown={onWindowKeydown} />

<div class="canvas-root">
  <header class="toolbar">
    <button class="btn-secondary small" on:click={backToFloor}>← floor</button>
    <h3>{graphName}</h3>
    <div class="provider-anchor">
      <button
        type="button"
        class="provider-chip"
        on:click={() => (providerPickerOpen = !providerPickerOpen)}
        disabled={$activePipeline !== null}
        title={$activePipeline !== null
          ? 'stop the pipeline before switching provider'
          : 'change provider for this graph'}
      >
        {providerLabel}
        <span class="caret">▾</span>
      </button>
      {#if providerPickerOpen}
        <div
          class="picker-backdrop"
          role="presentation"
          on:click={() => (providerPickerOpen = false)}
          on:contextmenu|preventDefault={() => (providerPickerOpen = false)}
        ></div>
        <div class="provider-popover" role="menu">
          {#each $backends as b (b.id)}
            {@const supported = isMeshSupportedProvider(b.id)}
            <button
              type="button"
              class="provider-option"
              class:active={graphProvider === b.id}
              disabled={!supported}
              title={supported ? '' : 'mesh v1 supports claude code only'}
              on:click={() => onPickProvider(b.id)}
            >
              {b.name}
            </button>
          {/each}
        </div>
      {/if}
    </div>
    <div class="pipeline-bar">
      <input
        class="task-input"
        bind:value={taskText}
        placeholder="initial task (goes to the first agent)…"
        disabled={$activePipeline !== null}
      />
      {#if $activePipeline === null}
        <button class="btn-run" on:click={onStartPipeline} disabled={!taskText.trim()}>
          ▶ start pipeline
        </button>
      {:else}
        <button class="btn-stop" on:click={stopPipeline}> ■ stop pipeline </button>
      {/if}
      <button
        class="btn-bulk"
        on:click={onStopAll}
        disabled={!hasAnyBoundSession}
        title="stop every agent in this graph"
      >
        ■ stop all
      </button>
      <button
        class="btn-bulk"
        on:click={onClearAll}
        disabled={!hasAnyBoundSession}
        title="reset every agent (drops sessions and context)"
      >
        ↻ clear all
      </button>
    </div>
  </header>

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

  {#if confirmDlg}
    <ConfirmModal
      title={confirmDlg.title}
      message={confirmDlg.message}
      on:confirm={() => confirmDlg?.onConfirm()}
      on:cancel={() => (confirmDlg = null)}
    />
  {/if}

  {#if contextMenu}
    <!-- Backdrop catches outside clicks to close the menu -->
    <div
      class="ctx-backdrop"
      role="presentation"
      on:click={closeContextMenu}
      on:contextmenu|preventDefault={closeContextMenu}
    ></div>
    <div class="ctx-menu" style="left: {contextMenu.x}px; top: {contextMenu.y}px" role="menu">
      <div class="ctx-header">{contextMenu.label}</div>
      <button class="ctx-item danger" type="button" role="menuitem" on:click={onContextMenuDelete}>
        delete
      </button>
    </div>
  {/if}

  <div class="canvas-body">
    <aside class="template-tray" class:locked={$activePipeline !== null}>
      <div class="tray-title">
        Templates
        {#if $activePipeline !== null}
          <span class="lock-hint">— locked while running</span>
        {/if}
      </div>

      {#if $templatesStore.length === 0}
        <div class="empty">No templates. Go back to the floor and create one.</div>
      {/if}

      {#if trayAgents.length > 0}
        <div class="tray-group-title"><KindIcon kind="agent" /> agents</div>
        {#each trayAgents as t (t.id)}
          <div
            class="tray-item"
            draggable="true"
            on:dragstart={(e) => onTemplateDragStart(t, e)}
            role="button"
            tabindex="0"
          >
            <div class="t-name">{t.name}</div>
            <div class="t-sub">{t.prePrompt.slice(0, 40)}{t.prePrompt.length > 40 ? '…' : ''}</div>
          </div>
        {/each}
      {/if}

      {#if trayBrowsers.length > 0}
        <div class="tray-group-title"><KindIcon kind="browser" /> browsers</div>
        {#each trayBrowsers as t (t.id)}
          <div
            class="tray-item tray-item-browser"
            draggable="true"
            on:dragstart={(e) => onTemplateDragStart(t, e)}
            role="button"
            tabindex="0"
          >
            <div class="t-name">{t.name}</div>
            <div class="t-sub">{t.prePrompt.slice(0, 40)}{t.prePrompt.length > 40 ? '…' : ''}</div>
          </div>
        {/each}
      {/if}

      {#if traySkills.length > 0}
        <div class="tray-group-title"><KindIcon kind="skill" /> skills</div>
        {#each traySkills as t (t.id)}
          <div
            class="tray-item tray-item-skill"
            draggable="true"
            on:dragstart={(e) => onTemplateDragStart(t, e)}
            role="button"
            tabindex="0"
          >
            <div class="t-name">{t.name}</div>
            <div class="t-sub">{t.prePrompt.slice(0, 40)}{t.prePrompt.length > 40 ? '…' : ''}</div>
          </div>
        {/each}
      {/if}

      <div class="tray-group-title"><KindIcon kind="note" /> notes</div>
      <div
        class="tray-item tray-item-note"
        draggable="true"
        on:dragstart={onNoteTrayDragStart}
        role="button"
        tabindex="0"
        title="drag onto the canvas to drop a fresh note"
      >
        <div class="t-name">+ new note</div>
        <div class="t-sub">markdown — read by connected agents as context</div>
      </div>
    </aside>

    <div
      class="flow-wrap"
      class:pipeline-running={$activePipeline !== null}
      on:drop={onDropTemplate}
      on:dragover={onDragOver}
      role="application"
    >
      <SvelteFlow
        bind:nodes={flowNodes}
        bind:edges={flowEdges}
        bind:viewport
        {nodeTypes}
        fitView
        minZoom={0.05}
        maxZoom={3}
        deleteKey={null}
        connectionMode={ConnectionMode.Loose}
        connectionRadius={60}
        {isValidConnection}
        onconnect={onConnect}
        onedgeclick={onEdgeClick}
        onnodedragstop={onNodeDragStop}
        onnodecontextmenu={onNodeContextMenu}
      >
        <Background />
        <Controls />
      </SvelteFlow>
    </div>
  </div>
</div>

<style>
  .canvas-root {
    display: flex;
    flex-direction: column;
    height: 100%;
    flex: 1;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--sp-5);
    padding: var(--sp-4) var(--sp-6);
    border-bottom: 1px solid var(--bd);
    background: var(--bg1);
  }

  .toolbar h3 {
    margin: 0;
    font-size: var(--base);
    color: var(--t0);
  }

  .provider-anchor {
    position: relative;
  }
  .provider-chip {
    background: var(--bg);
    color: var(--t1);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: var(--sp-2) var(--sp-4);
    font-size: var(--xs);
    font-family: inherit;
    text-transform: lowercase;
    letter-spacing: 0.3px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .provider-chip:hover:not(:disabled) {
    border-color: var(--ac);
    color: var(--ac);
  }
  .provider-chip:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .caret {
    font-size: 10px;
    opacity: 0.7;
  }
  .picker-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
  }
  .provider-popover {
    position: absolute;
    top: calc(100% + var(--sp-2));
    left: 0;
    z-index: 51;
    min-width: 160px;
    background: var(--bg1);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: var(--sp-2);
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }
  .provider-option {
    background: transparent;
    color: var(--t0);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    padding: var(--sp-2) var(--sp-4);
    font-size: var(--sm);
    font-family: inherit;
    text-align: left;
    text-transform: lowercase;
    cursor: pointer;
  }
  .provider-option:hover:not(:disabled) {
    background: var(--bg3);
    border-color: var(--ac);
    color: var(--ac);
  }
  .provider-option.active {
    background: var(--ac-d);
    border-color: var(--ac);
    color: var(--ac);
  }
  .provider-option:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .pipeline-bar {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    margin-left: auto;
    flex: 1;
    justify-content: flex-end;
    max-width: 640px;
  }
  .task-input {
    flex: 1;
    background: var(--bg);
    border: 1px solid var(--bd1);
    color: var(--t0);
    padding: var(--sp-3) var(--sp-5);
    font-size: var(--sm);
    border-radius: var(--radius-sm);
    font-family: var(--mono);
    outline: none;
  }
  .task-input:focus {
    border-color: var(--ac);
  }
  .task-input:disabled {
    opacity: 0.5;
  }
  .btn-run {
    background: var(--ac);
    color: #000;
    border: 1px solid var(--ac);
    padding: var(--sp-3) var(--sp-6);
    border-radius: var(--radius-sm);
    font-size: var(--sm);
    cursor: pointer;
    font-family: var(--mono);
    white-space: nowrap;
  }
  .btn-run:hover {
    background: transparent;
    color: var(--ac);
  }
  .btn-run:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .toolbar .btn-stop {
    background: transparent;
    color: var(--s-error);
    border: 1px solid var(--s-error);
    padding: var(--sp-3) var(--sp-6);
    border-radius: var(--radius-sm);
    font-size: var(--sm);
    cursor: pointer;
    font-family: var(--mono);
    white-space: nowrap;
  }
  .btn-bulk {
    background: transparent;
    color: var(--t1);
    border: 1px solid var(--bd1);
    padding: var(--sp-3) var(--sp-5);
    border-radius: var(--radius-sm);
    font-size: var(--sm);
    cursor: pointer;
    font-family: var(--mono);
    white-space: nowrap;
  }
  .btn-bulk:hover:not(:disabled) {
    background: var(--bg3);
    border-color: var(--ac);
    color: var(--ac);
  }
  .btn-bulk:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .canvas-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .template-tray {
    width: 220px;
    padding: var(--sp-4);
    border-right: 1px solid var(--bd);
    background: var(--bg1);
    overflow-y: auto;
  }
  .template-tray.locked .tray-item {
    opacity: 0.4;
    cursor: not-allowed;
    pointer-events: none;
  }
  .lock-hint {
    text-transform: lowercase;
    font-weight: 400;
    color: var(--s-input);
    margin-left: var(--sp-2);
    letter-spacing: 0;
  }

  .tray-title {
    font-size: var(--xs);
    font-weight: 600;
    text-transform: uppercase;
    color: var(--t2);
    margin-bottom: var(--sp-3);
    letter-spacing: 0.3px;
  }

  .tray-group-title {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    font-size: var(--xs);
    text-transform: lowercase;
    color: var(--t1);
    margin: var(--sp-4) 0 var(--sp-2);
    letter-spacing: 0.3px;
    font-weight: 600;
  }
  .tray-group-title:first-of-type {
    margin-top: var(--sp-2);
  }

  .tray-item {
    padding: var(--sp-3) var(--sp-4);
    margin-bottom: var(--sp-2);
    background: var(--bg);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    cursor: grab;
    user-select: none;
  }
  .tray-item.tray-item-browser {
    border-left: 3px solid var(--s-init);
  }
  .tray-item.tray-item-skill {
    border-left: 3px solid var(--think-fg);
  }
  .tray-item.tray-item-note {
    border-left: 3px solid var(--ac);
  }

  .tray-item:active {
    cursor: grabbing;
  }

  .tray-item:hover {
    background: var(--bg3);
  }

  .t-name {
    font-weight: 600;
    font-size: var(--sm);
  }

  .t-sub {
    font-size: var(--xs);
    color: var(--t2);
    margin-top: 2px;
  }

  .empty {
    font-size: var(--sm);
    color: var(--t2);
    font-style: italic;
    padding: var(--sp-4);
  }

  .flow-wrap {
    flex: 1;
    position: relative;
    background: var(--bg);
  }
  .flow-wrap.pipeline-running :global(.svelte-flow__handle) {
    pointer-events: none;
    opacity: 0.45;
  }

  .btn-secondary {
    background: transparent;
    color: var(--t0);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: var(--sp-2) var(--sp-4);
    font-size: var(--xs);
    cursor: pointer;
    font-family: inherit;
  }

  .btn-secondary:hover {
    background: var(--bg3);
    border-color: var(--ac);
    color: var(--ac);
  }

  .btn-secondary.small {
    font-size: var(--xs);
  }

  .ctx-backdrop {
    position: fixed;
    inset: 0;
    z-index: 150;
    background: transparent;
  }

  .ctx-menu {
    position: fixed;
    z-index: 151;
    min-width: 160px;
    background: var(--bg1);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    padding: var(--sp-2);
    font-family: inherit;
  }

  .ctx-header {
    padding: var(--sp-2) var(--sp-4);
    font-size: var(--xs);
    color: var(--t2);
    text-transform: lowercase;
    letter-spacing: 0.3px;
    border-bottom: 1px solid var(--bd);
    margin-bottom: var(--sp-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ctx-item {
    display: block;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--t0);
    padding: var(--sp-3) var(--sp-4);
    font-size: var(--sm);
    font-family: inherit;
    cursor: pointer;
    border-radius: var(--radius-sm);
  }

  .ctx-item:hover {
    background: var(--bg3);
  }

  .ctx-item.danger {
    color: var(--s-error);
  }

  .ctx-item.danger:hover {
    background: rgba(224, 72, 72, 0.12);
  }

  /* SvelteFlow edges — make connections thicker so they read on a dark canvas */
  :global(.svelte-flow__edge-path) {
    stroke: var(--bd2);
    stroke-width: 2.5;
  }
  :global(.svelte-flow__edge.selected .svelte-flow__edge-path),
  :global(.svelte-flow__edge:focus .svelte-flow__edge-path),
  :global(.svelte-flow__edge:focus-visible .svelte-flow__edge-path) {
    stroke: var(--ac);
    stroke-width: 3;
  }
  :global(.svelte-flow__connection-path) {
    stroke: var(--ac);
    stroke-width: 2.5;
  }

  /* SvelteFlow Controls — override default light-theme colors */
  :global(.svelte-flow__controls) {
    background: var(--bg1);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
    overflow: hidden;
  }

  :global(.svelte-flow__controls-button) {
    background: var(--bg1);
    color: var(--t0);
    border: none;
    border-bottom: 1px solid var(--bd);
    width: 26px;
    height: 26px;
    padding: 4px;
  }

  :global(.svelte-flow__controls-button:last-child) {
    border-bottom: none;
  }

  :global(.svelte-flow__controls-button:hover) {
    background: var(--bg3);
    color: var(--ac);
  }

  :global(.svelte-flow__controls-button svg) {
    fill: currentColor;
    width: 14px;
    height: 14px;
    max-width: 14px;
    max-height: 14px;
  }
</style>
