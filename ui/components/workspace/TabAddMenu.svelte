<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { Bot, FolderOpen, GitBranch, Terminal } from 'lucide-svelte';

  export let x: number;
  export let y: number;

  type AddAction = 'terminal' | 'session' | 'open' | 'git';

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
  <button class="menu-item" role="menuitem" on:click={() => select('git')}>
    <GitBranch size={14} />
    Git overview
  </button>
  <button class="menu-item" role="menuitem" on:click={() => select('session')}>
    <Bot size={14} />
    New session
  </button>
  <div class="menu-divider"></div>
  <button class="menu-item" role="menuitem" on:click={() => select('open')}>
    <FolderOpen size={14} />
    Open session...
  </button>
</div>

<style>
  .menu {
    position: fixed;
    min-width: 176px;
    padding: var(--sp-2) 0;
    z-index: 1000;
    border: 1px solid color-mix(in srgb, var(--ac), transparent 78%);
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

  .menu-divider {
    height: 1px;
    background: var(--bd);
    margin: var(--sp-2) 0;
  }
</style>
