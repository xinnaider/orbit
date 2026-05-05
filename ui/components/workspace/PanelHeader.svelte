<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { X } from 'lucide-svelte';

  export let title: string;
  export let status: string | null = null;
  export let dragPayload: string | null = null;
  export let closeLabel = 'Close panel';
  export let onClose: (() => void) | null = null;
  export let focused: boolean = true;

  const dispatch = createEventDispatcher<{ dragstart: DragEvent }>();

  function handleDragStart(e: DragEvent) {
    if (!dragPayload) return;
    e.dataTransfer?.setData('text/plain', dragPayload);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
    dispatch('dragstart', e);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header class="panel-header" class:focused draggable={!!dragPayload} on:dragstart={handleDragStart}>
  <slot name="leading" />

  <div class="panel-title-block">
    <span class="panel-title-row">
      <span class="panel-title">{title}</span>
      {#if status}
        <span class="panel-title-sep">/</span>
        <span class="panel-subtitle">{status}</span>
      {/if}
    </span>
  </div>

  <div class="panel-meta">
    <slot name="meta" />
  </div>

  <div class="panel-actions">
    <slot name="actions" />
    {#if onClose}
      <button class="panel-icon-button" type="button" aria-label={closeLabel} on:click={onClose}>
        <X size={14} />
      </button>
    {/if}
  </div>
</header>

<style>
  .panel-header {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 34px;
    padding: 0 10px;
    border-bottom: 1px solid var(--bd);
    background: var(--bg);
    flex-shrink: 0;
    user-select: none;
    transition: opacity 0.15s;
  }

  .panel-header:not(.focused) {
    opacity: 0.65;
  }

  .panel-header[draggable='true'] {
    cursor: grab;
  }

  .panel-title-block {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: 1;
    overflow: hidden;
  }

  .panel-title-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .panel-title {
    overflow: hidden;
    color: var(--t0);
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
    letter-spacing: -0.01em;
  }

  .panel-title-sep {
    color: var(--t3);
    font-family: var(--mono);
    font-size: 10px;
    flex-shrink: 0;
  }

  .panel-subtitle {
    overflow: hidden;
    color: var(--t2);
    font-family: var(--mono);
    font-size: 10px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .panel-meta,
  .panel-actions {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    flex-shrink: 0;
  }

  .panel-icon-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: 1px solid var(--bd1);
    border-radius: 4px;
    background: var(--bg2);
    color: var(--t2);
    transition:
      border-color 0.15s,
      background 0.15s,
      color 0.15s;
  }

  .panel-icon-button:hover {
    border-color: color-mix(in srgb, var(--ac), transparent 70%);
    background: color-mix(in srgb, var(--ac), transparent 90%);
    color: var(--t0);
  }
</style>
