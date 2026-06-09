<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { GitBranch, Terminal } from 'lucide-svelte';

  export let x: number;
  export let y: number;

  // TEMPORARILY DISABLED: Terminal and Git overview tabs are turned off for now
  // — we are shipping chats only. The options are shown greyed out so users know
  // they are coming back. To re-enable, restore the `select(action)` dispatch on
  // each item below (see git history) and re-enable PaneContainer.handleAddAction.
  const dispatch = createEventDispatcher<{
    close: void;
  }>();

  $: viewportWidth = typeof window === 'undefined' ? 200 : window.innerWidth;
  $: viewportHeight = typeof window === 'undefined' ? 150 : window.innerHeight;
  $: menuLeft = Math.min(x, viewportWidth - 200);
  $: menuTop = Math.min(y, viewportHeight - 150);

  onMount(() => {
    setTimeout(() => {
      window.addEventListener('click', handleOutsideClick, { once: true });
    }, 0);

    return () => {
      window.removeEventListener('click', handleOutsideClick);
    };
  });

  function handleOutsideClick() {
    dispatch('close');
  }
</script>

<div class="menu" style="left: {menuLeft}px; top: {menuTop}px;" role="menu">
  <div class="menu-note">Paused — chats only for now</div>
  <button class="menu-item" role="menuitem" disabled title="Coming back soon">
    <Terminal size={14} />
    New terminal
  </button>
  <button
    class="menu-item"
    data-testid="add-git-tab-option"
    role="menuitem"
    disabled
    title="Coming back soon"
  >
    <GitBranch size={14} />
    Git overview
  </button>
</div>

<style>
  .menu {
    position: fixed;
    min-width: 200px;
    padding: var(--sp-2) 0;
    z-index: 1000;
    border: 1px solid var(--bd);
    border-radius: 6px;
    background: var(--bg1);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.38);
  }

  .menu-note {
    padding: var(--sp-1) var(--sp-4) var(--sp-2);
    font-size: var(--xs);
    color: var(--t3);
    font-family: var(--mono);
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    width: 100%;
    padding: var(--sp-2) var(--sp-4);
    border: none;
    background: transparent;
    color: var(--t1);
    font-size: var(--sm);
    cursor: pointer;
    text-align: left;
    transition:
      background 0.15s,
      color 0.15s;
  }

  .menu-item:hover:not(:disabled) {
    background: var(--ac-d2);
    color: var(--t0);
  }

  .menu-item:disabled {
    color: var(--t3);
    cursor: not-allowed;
    opacity: 0.6;
  }
</style>
