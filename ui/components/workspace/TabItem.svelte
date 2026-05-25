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
  class="tab-cap"
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
  .tab-cap {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 11px;
    min-width: 0;
    max-width: 160px;
    border-radius: 999px;
    border: 1px solid transparent;
    background: transparent;
    cursor: pointer;
    user-select: none;
    font-family: var(--mono);
    font-size: 10px;
    font-weight: 450;
    color: var(--t2);
    flex-shrink: 0;
    white-space: nowrap;
    overflow: hidden;
    transition:
      background 0.12s,
      color 0.12s,
      border-color 0.12s,
      box-shadow 0.12s;
  }

  .tab-cap:hover {
    color: var(--t1);
    background: color-mix(in srgb, var(--bg2), transparent 40%);
  }

  .tab-cap.active {
    color: var(--t0);
    font-weight: 500;
    background: var(--bg2);
    border-color: var(--bd1);
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.35);
  }

  .tab-cap:not(.focused) {
    opacity: 0.72;
  }

  .tab-cap:not(.focused).active {
    opacity: 0.85;
  }

  .tab-icon {
    display: inline-flex;
    color: inherit;
    flex-shrink: 0;
  }

  .tab-label {
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
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
    border-radius: 999px;
    padding: 0;
    opacity: 0;
    transition:
      opacity 0.12s,
      background 0.12s;
  }

  .tab-cap.active .tab-close,
  .tab-cap:hover .tab-close {
    opacity: 0.45;
  }

  .tab-close:hover {
    opacity: 1;
    background: color-mix(in srgb, var(--t0), transparent 92%);
  }
</style>
