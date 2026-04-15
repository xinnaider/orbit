<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';

  export let x = 0;
  export let y = 0;
  export let items: Array<{
    label: string;
    action: string;
    danger?: boolean;
    divider?: boolean;
    html?: boolean;
  }> = [];

  const dispatch = createEventDispatcher();
  let el: HTMLDivElement;

  function close() {
    dispatch('close');
  }

  function select(action: string) {
    dispatch('select', action);
    close();
  }

  // Close on outside click or Escape
  function onGlobalClick(e: MouseEvent) {
    if (el && !el.contains(e.target as Node)) close();
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  onMount(() => {
    // Adjust position if menu overflows viewport
    requestAnimationFrame(() => {
      if (!el) return;
      const rect = el.getBoundingClientRect();
      if (rect.right > window.innerWidth) {
        el.style.left = x - rect.width + 'px';
      }
      if (rect.bottom > window.innerHeight) {
        el.style.top = y - rect.height + 'px';
      }
    });
    window.addEventListener('click', onGlobalClick, true);
    window.addEventListener('keydown', onKey);
  });

  onDestroy(() => {
    window.removeEventListener('click', onGlobalClick, true);
    window.removeEventListener('keydown', onKey);
  });
</script>

<div
  class="menu"
  role="menu"
  tabindex="-1"
  bind:this={el}
  style="left:{x}px; top:{y}px"
  on:contextmenu|preventDefault
>
  {#each items as item}
    {#if item.divider || item.action === 'divider'}
      <div class="divider"></div>
    {:else}
      <button class="item" class:danger={item.danger} on:click={() => select(item.action)}>
        {#if item.html}
          {@html item.label}
        {:else}
          {item.label}
        {/if}
      </button>
    {/if}
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 999;
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-md);
    padding: var(--sp-2);
    min-width: 160px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  }
  .item {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    color: var(--t0);
    font-family: var(--mono);
    font-size: var(--sm);
    padding: var(--sp-3) var(--sp-5);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .item:hover {
    background: var(--bg3);
  }
  .item.danger {
    color: var(--s-error);
  }
  .item.danger:hover {
    background: rgba(224, 72, 72, 0.1);
  }
  .divider {
    height: 1px;
    background: var(--bd);
    margin: var(--sp-2) var(--sp-2);
  }
  .item :global(.ctx-icon-label) {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }
</style>
