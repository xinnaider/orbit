<script lang="ts">
  import { Handle, Position, NodeResizer } from '@xyflow/svelte';
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { resizeNode as persistResize } from '../../lib/stores/mesh/graph';
  import { notesStore, renameNote, setNoteContent } from '../../lib/stores/mesh/notes';
  import { addToast } from '../../lib/stores/toasts';
  import Markdown from '../Markdown.svelte';

  type NoteNodeData = {
    label: string;
  };

  export let id: string;
  export let data: NoteNodeData;

  let mode: 'raw' | 'preview' = 'preview';
  let content = '';
  let name = '';
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  $: nodeId = Number(id);
  $: note = $notesStore.find((n) => n.nodeId === nodeId) ?? null;

  // Hydrate local edit buffer when the underlying note changes (load, rename).
  $: if (note && content === '' && name === '') {
    content = note.content;
    name = note.name || data.label;
  }
  $: if (note && name === '' && note.name) {
    name = note.name;
  }

  onMount(() => {
    const n = get(notesStore).find((x) => x.nodeId === nodeId);
    if (n) {
      content = n.content;
      name = n.name || data.label;
    } else {
      name = data.label;
    }
  });

  onDestroy(() => {
    if (saveTimer) clearTimeout(saveTimer);
  });

  function scheduleSave() {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      try {
        await setNoteContent(nodeId, content);
      } catch (e) {
        addToast({ type: 'error', message: `note save failed: ${e}`, autoDismiss: true });
      }
    }, 400);
  }

  async function commitName() {
    const trimmed = name.trim();
    if (!trimmed || trimmed === note?.name) return;
    try {
      await renameNote(nodeId, trimmed);
    } catch (e) {
      addToast({ type: 'error', message: `rename failed: ${e}`, autoDismiss: true });
    }
  }

  async function onResizeEnd(_e: unknown, params: { width: number; height: number }) {
    try {
      await persistResize(nodeId, params.width, params.height);
    } catch (e) {
      addToast({ type: 'error', message: `failed to persist size: ${e}`, autoDismiss: true });
    }
  }
</script>

<div class="note-node">
  <NodeResizer
    minWidth={220}
    minHeight={160}
    lineClass="resize-line"
    handleClass="resize-handle"
    {onResizeEnd}
  />
  <Handle id="top" type="source" position={Position.Top} />
  <Handle id="left" type="source" position={Position.Left} />
  <Handle id="right" type="source" position={Position.Right} />
  <Handle id="bottom" type="source" position={Position.Bottom} />

  <header class="nt-header">
    <span class="dot"></span>
    <input
      class="nt-name"
      type="text"
      bind:value={name}
      on:blur={commitName}
      on:keydown={(e) => e.key === 'Enter' && (e.currentTarget as HTMLInputElement).blur()}
      placeholder="untitled"
    />
    <span class="kind-badge">note</span>
    <button
      type="button"
      class="mode-toggle"
      on:click={() => (mode = mode === 'raw' ? 'preview' : 'raw')}
      title={mode === 'raw' ? 'show preview' : 'edit raw'}
    >
      {mode === 'raw' ? 'preview' : 'raw'}
    </button>
  </header>

  <div class="nt-body">
    {#if mode === 'raw'}
      <textarea
        class="nt-textarea"
        bind:value={content}
        on:input={scheduleSave}
        placeholder="write markdown…"
        spellcheck="false"
      ></textarea>
    {:else if content.trim()}
      <div class="nt-preview"><Markdown {content} /></div>
    {:else}
      <div class="nt-empty">empty note · switch to raw to write</div>
    {/if}
  </div>
</div>

<style>
  .note-node {
    position: relative;
    width: 100%;
    height: 100%;
    min-width: 220px;
    min-height: 160px;
    background: var(--bg);
    border: 1px solid var(--bd1);
    border-left: 3px solid var(--ac);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--t0);
  }

  .nt-header {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-3) var(--sp-5);
    background: var(--bg1);
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
    font-size: var(--sm);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ac);
    flex-shrink: 0;
  }
  .nt-name {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--t0);
    font-family: inherit;
    font-size: var(--sm);
    font-weight: 600;
    padding: 0;
    outline: none;
    min-width: 0;
  }
  .nt-name:focus {
    color: var(--ac);
  }
  .kind-badge {
    font-size: var(--xs);
    color: var(--ac);
    background: var(--ac-d);
    padding: 1px 6px;
    border-radius: 10px;
    text-transform: lowercase;
    flex-shrink: 0;
  }
  .mode-toggle {
    background: transparent;
    border: 1px solid var(--bd1);
    color: var(--t1);
    font-family: inherit;
    font-size: var(--xs);
    text-transform: lowercase;
    padding: 2px var(--sp-3);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .mode-toggle:hover {
    border-color: var(--ac);
    color: var(--ac);
  }

  .nt-body {
    flex: 1;
    overflow: auto;
    background: var(--bg);
  }
  .nt-textarea {
    width: 100%;
    height: 100%;
    background: var(--bg);
    color: var(--t0);
    border: none;
    outline: none;
    resize: none;
    padding: var(--sp-4) var(--sp-5);
    font-family: var(--mono);
    font-size: var(--sm);
    line-height: 1.5;
  }
  .nt-preview {
    padding: var(--sp-4) var(--sp-5);
    font-size: var(--sm);
    line-height: 1.5;
  }
  .nt-empty {
    padding: var(--sp-7);
    color: var(--t2);
    font-size: var(--sm);
    font-style: italic;
    text-align: center;
  }

  :global(.svelte-flow__node-note .svelte-flow__handle) {
    background: var(--ac);
    width: 24px;
    height: 24px;
    border: 2px solid var(--bg);
    z-index: 10;
  }
  :global(.svelte-flow__node-note .svelte-flow__handle:hover) {
    background: var(--ac-h);
  }
  :global(.svelte-flow__node-note .resize-handle) {
    width: 16px;
    height: 16px;
    background: var(--ac);
    border: 2px solid var(--bg);
    border-radius: 3px;
  }
  :global(.svelte-flow__node-note .resize-line) {
    border-color: var(--ac);
    border-width: 3px;
  }
</style>
