<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { Bot, GitBranch, Terminal, X } from 'lucide-svelte';
  import type { Tab } from '../../lib/stores/workspace';
  import { sessions } from '../../lib/stores/sessions';

  export let tab: Tab;
  export let active: boolean;
  export let paneId: string;
  export let focused: boolean = true;

  const dispatch = createEventDispatcher<{ close: { tabId: string } }>();

  $: label = (() => {
    const target = tab.target;
    if (target.kind === 'agent') {
      const s = $sessions.find((s) => s.id === target.sessionId);
      return s?.name ?? s?.projectName ?? s?.cwd?.split(/[\\/]/).pop() ?? `#${target.sessionId}`;
    }
    if (target.kind === 'git') return 'Git';
    return 'Terminal';
  })();

  function handleDragStart(e: DragEvent) {
    e.dataTransfer!.effectAllowed = 'move';
    e.dataTransfer!.setData(
      'text/plain',
      JSON.stringify({ tabId: tab.id, sourcePaneId: paneId, target: tab.target })
    );
  }

  function handleClose(e: MouseEvent) {
    e.stopPropagation();
    dispatch('close', { tabId: tab.id });
  }
</script>

<div
  class="tab-item"
  class:active
  class:focused
  draggable="true"
  on:dragstart={handleDragStart}
  role="tab"
  aria-selected={active}
  tabindex="0"
  on:keydown={(e) => e.key === 'Enter' && e.currentTarget.click()}
>
  <span class="tab-icon" aria-hidden="true">
    {#if tab.target.kind === 'agent'}
      <Bot size={13} />
    {:else if tab.target.kind === 'git'}
      <GitBranch size={13} />
    {:else}
      <Terminal size={13} />
    {/if}
  </span>
  <span class="tab-label">{label}</span>
  <button class="tab-close" on:click={handleClose} aria-label="Close tab">
    <X size={12} />
  </button>
</div>

<style>
  .tab-item {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 8px;
    height: 26px;
    min-width: 70px;
    max-width: 160px;
    cursor: pointer;
    user-select: none;
    font-family: var(--mono);
    font-size: 10.5px;
    font-weight: 450;
    color: var(--t2);
    border-radius: var(--radius-sm);
    border: none;
    background: transparent;
    flex-shrink: 0;
    white-space: nowrap;
    overflow: hidden;
    transition: background 0.1s, color 0.1s;
  }

  .tab-item:hover {
    background: var(--bg3);
    color: var(--t1);
  }

  .tab-item.active {
    background: var(--ac-d2);
    color: var(--ac);
    font-weight: 500;
  }

  /* Unfocused pane — subdued tabs */
  .tab-item:not(.focused) {
    opacity: 0.75;
  }

  .tab-item:not(.focused).active {
    background: var(--ac-d2);
    color: var(--ac);
    opacity: 0.6;
  }

  .tab-icon {
    display: inline-flex;
    color: inherit;
    flex-shrink: 0;
  }

  .tab-label {
    font-size: 10.5px;
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
    cursor: pointer;
    flex-shrink: 0;
    border-radius: 3px;
    padding: 0;
    line-height: 1;
    transition:
      color 0.15s,
      opacity 0.15s,
      background 0.15s;
    opacity: 0;
  }

  .tab-item.active .tab-close,
  .tab-item:hover .tab-close {
    opacity: 0.7;
  }

  .tab-close:hover {
    color: var(--t0);
    background: var(--ac-d2);
  }
</style>
