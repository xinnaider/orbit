<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { Tab } from '../../lib/stores/workspace';
  import { sessions } from '../../lib/stores/sessions';

  export let tab: Tab;
  export let active: boolean;
  export let paneId: string;

  const dispatch = createEventDispatcher<{ close: { tabId: string } }>();

  $: isAgent = tab.target.kind === 'agent';
  $: icon = isAgent ? '●' : '>';
  $: label = (() => {
    if (tab.target.kind === 'agent') {
      const target = tab.target as { kind: 'agent'; sessionId: number };
      const s = $sessions.find((s) => s.id === target.sessionId);
      return (
        s?.name ??
        s?.projectName ??
        s?.cwd?.split(/[\\/]/).pop() ??
        `#${target.sessionId}`
      );
    }
    return 'Terminal';
  })();

  function handleDragStart(e: DragEvent) {
    e.dataTransfer!.effectAllowed = 'move';
    e.dataTransfer!.setData('text/plain', JSON.stringify({ tabId: tab.id, sourcePaneId: paneId }));
  }

  function handleClose(e: MouseEvent) {
    e.stopPropagation();
    dispatch('close', { tabId: tab.id });
  }
</script>

<div
  class="tab-item"
  class:active
  draggable="true"
  on:dragstart={handleDragStart}
  role="tab"
  aria-selected={active}
  tabindex="0"
  on:keydown={(e) => e.key === 'Enter' && e.currentTarget.click()}
>
  <span class="tab-icon">{icon}</span>
  <span class="tab-label">{label}</span>
  <button class="tab-close" on:click={handleClose} aria-label="Close tab">×</button>
</div>

<style>
  .tab-item {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: 0 var(--sp-3);
    height: 32px;
    min-width: 120px;
    max-width: 200px;
    cursor: pointer;
    user-select: none;
    color: var(--t2);
    background: transparent;
    border-bottom: 2px solid transparent;
    flex-shrink: 0;
    white-space: nowrap;
    overflow: hidden;
    transition: background 0.1s;
  }

  .tab-item:hover {
    background: var(--bg2);
  }

  .tab-item.active {
    color: var(--t0);
    background: var(--bg);
    border-bottom-color: var(--ac);
  }

  .tab-icon {
    font-size: var(--xs);
    flex-shrink: 0;
  }

  .tab-label {
    font-size: var(--sm);
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .tab-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border: none;
    background: transparent;
    color: var(--t2);
    font-size: var(--md);
    cursor: pointer;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    padding: 0;
    line-height: 1;
    transition: color 0.1s;
  }

  .tab-close:hover {
    color: var(--s-error);
  }
</style>
