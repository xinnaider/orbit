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
      <Bot size={12} />
    {:else if tab.target.kind === 'git'}
      <GitBranch size={12} />
    {:else}
      <Terminal size={12} />
    {/if}
  </span>
  <span class="tab-label">{label}</span>
  <button class="tab-close" on:click={handleClose} aria-label="Close tab">
    <X size={11} />
  </button>
</div>

<style>
  .tab-item {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 8px 0 6px;
    height: 26px;
    min-width: 60px;
    max-width: 160px;
    cursor: pointer;
    user-select: none;
    font-family: var(--mono);
    font-size: 9.5px;
    font-weight: 450;
    color: color-mix(in srgb, var(--t0), transparent 70%);
    border: none;
    border-radius: 6px;
    background: transparent;
    flex-shrink: 0;
    white-space: nowrap;
    overflow: hidden;
    position: relative;
    transition:
      background 0.12s,
      color 0.12s,
      box-shadow 0.12s;
  }

  .tab-item:hover:not(.active) {
    color: color-mix(in srgb, var(--t0), transparent 50%);
  }

  .tab-item.active {
    background: var(--glass-tab-active-bg);
    color: var(--t0);
    font-weight: 500;
    box-shadow: var(--glass-tab-active-shadow);
  }

  /* Unfocused pane — subdued tabs */
  .tab-item:not(.focused) {
    opacity: 0.7;
  }

  .tab-item:not(.focused).active {
    opacity: 0.85;
  }

  .tab-icon {
    display: inline-flex;
    color: inherit;
    flex-shrink: 0;
    opacity: 0.85;
  }

  .tab-label {
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .tab-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
    flex-shrink: 0;
    border-radius: 4px;
    padding: 0;
    line-height: 1;
    transition:
      color 0.15s,
      opacity 0.15s,
      background 0.15s;
    opacity: 0;
  }

  .tab-item.active .tab-close {
    opacity: 0.45;
  }

  .tab-item:hover .tab-close {
    opacity: 0.65;
  }

  .tab-close:hover {
    color: var(--t0);
    background: var(--glass-bg-subtle);
    opacity: 1;
  }
</style>
