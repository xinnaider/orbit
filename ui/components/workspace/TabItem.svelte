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
    padding: 0 12px;
    height: 100%;
    min-width: 60px;
    max-width: 160px;
    cursor: pointer;
    user-select: none;
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 450;
    color: var(--t2);
    border: none;
    border-right: 1px solid var(--bd);
    background: transparent;
    flex-shrink: 0;
    white-space: nowrap;
    overflow: hidden;
    position: relative;
    transition:
      background 0.1s,
      color 0.1s;
  }

  :global(html[data-glass-chrome='true']) .tab-item {
    padding: 0 8px 0 6px;
    height: 26px;
    font-size: 9.5px;
    color: color-mix(in srgb, var(--t0), transparent 70%);
    border-right: none;
    border-radius: 6px;
    transition:
      background 0.12s,
      color 0.12s,
      box-shadow 0.12s;
  }

  .tab-item:hover {
    background: color-mix(in srgb, var(--bg), white 4%);
    color: var(--t1);
  }

  :global(html[data-glass-chrome='true']) .tab-item:hover:not(.active) {
    background: transparent;
    color: color-mix(in srgb, var(--t0), transparent 50%);
  }

  .tab-item.active {
    background: var(--bg);
    color: var(--t0);
    font-weight: 500;
    border-bottom: none;
  }

  .tab-item.active::after {
    content: '';
    position: absolute;
    bottom: -1px;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--ac);
    z-index: 1;
  }

  :global(html[data-glass-chrome='true']) .tab-item.active {
    background: var(--glass-tab-active-bg);
    box-shadow: var(--glass-tab-active-shadow);
  }

  :global(html[data-glass-chrome='true']) .tab-item.active::after {
    content: none;
  }

  /* Unfocused pane — subdued tabs */
  .tab-item:not(.focused) {
    opacity: 0.7;
  }

  .tab-item:not(.focused).active {
    background: var(--bg);
    color: var(--t1);
    opacity: 0.7;
  }

  .tab-item:not(.focused).active::after {
    opacity: 0.4;
  }

  :global(html[data-glass-chrome='true']) .tab-item:not(.focused).active {
    background: var(--glass-tab-active-bg);
    color: var(--t0);
    opacity: 0.85;
  }

  :global(html[data-glass-chrome='true']) .tab-item:not(.focused).active::after {
    content: none;
  }

  .tab-icon {
    display: inline-flex;
    color: inherit;
    flex-shrink: 0;
  }

  :global(html[data-glass-chrome='true']) .tab-icon {
    opacity: 0.85;
  }

  .tab-label {
    font-size: 10.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  :global(html[data-glass-chrome='true']) .tab-label {
    font-size: inherit;
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

  :global(html[data-glass-chrome='true']) .tab-close {
    width: 14px;
    height: 14px;
    color: inherit;
    border-radius: 4px;
  }

  .tab-item.active .tab-close {
    opacity: 0.5;
  }

  .tab-item:hover .tab-close {
    opacity: 0.7;
  }

  :global(html[data-glass-chrome='true']) .tab-item.active .tab-close {
    opacity: 0.45;
  }

  :global(html[data-glass-chrome='true']) .tab-item:hover .tab-close {
    opacity: 0.65;
  }

  .tab-close:hover {
    color: var(--t0);
    background: var(--bg3);
    opacity: 1;
  }

  :global(html[data-glass-chrome='true']) .tab-close:hover {
    background: var(--glass-bg-subtle);
  }
</style>
