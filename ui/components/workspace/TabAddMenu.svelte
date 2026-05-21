<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { GitBranch, Terminal } from 'lucide-svelte';

  export let x: number;
  export let y: number;

  type AddAction = 'terminal' | 'git';

  const dispatch = createEventDispatcher<{
    select: { action: AddAction };
    close: void;
  }>();

  $: viewportWidth = typeof window === 'undefined' ? 176 : window.innerWidth;
  $: viewportHeight = typeof window === 'undefined' ? 132 : window.innerHeight;
  $: menuLeft = Math.min(x, viewportWidth - 176);
  $: menuTop = Math.min(y, viewportHeight - 132);

  function select(action: AddAction) {
    dispatch('select', { action });
    dispatch('close');
  }

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
  <button class="menu-item" role="menuitem" on:click={() => select('terminal')}>
    <Terminal size={14} />
    New terminal
  </button>
  <button
    class="menu-item"
    data-testid="add-git-tab-option"
    role="menuitem"
    on:click={() => select('git')}
  >
    <GitBranch size={14} />
    Git overview
  </button>
</div>

<style>
  .menu {
    position: fixed;
    min-width: 176px;
    padding: var(--sp-2) 0;
    z-index: 1000;
    border: 1px solid var(--bd);
    border-radius: 6px;
    background: var(--bg1);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.38);
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

  .menu-item:hover {
    background: var(--ac-d2);
    color: var(--t0);
  }
</style>
