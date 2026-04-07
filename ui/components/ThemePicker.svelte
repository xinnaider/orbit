<script lang="ts">
  import { theme, THEME_OPTIONS, THEME_LABELS, type Theme } from '../lib/stores/preferences';
  import { Palette } from 'lucide-svelte';

  let open = false;

  function select(t: Theme) {
    theme.set(t);
    open = false;
  }

  function toggle(e: MouseEvent) {
    e.stopPropagation();
    open = !open;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      open = false;
    }
  }

  function closeOnOutside() {
    if (open) open = false;
  }
</script>

<svelte:window onclick={closeOnOutside} onkeydown={handleKeydown} />

<div class="theme-picker">
  <button class="trigger" title="Change theme" onclick={toggle}>
    <Palette size={13} />
  </button>
  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dropdown" onclick={(e) => e.stopPropagation()}>
      {#each THEME_OPTIONS as t}
        <button class="option" class:active={$theme === t} onclick={() => select(t)}>
          <span class="swatch" data-theme-preview={t}></span>
          {THEME_LABELS[t]}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .theme-picker {
    position: relative;
  }

  .trigger {
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: 0px solid var(--bd1);
    border-radius: 0px;
    outline: none;
    color: var(--t2);
    padding: 3px 5px;
    font-size: var(--xs);
    transition:
      color 0.15s,
      border-color 0.15s;
  }
  .trigger:hover {
    color: var(--t0);
    border-color: var(--bd2);
  }

  .dropdown {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: 6px;
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 130px;
    z-index: 100;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  }

  .option {
    display: flex;
    align-items: center;
    gap: 8px;
    background: none;
    border: none;
    border-radius: 4px;
    color: var(--t1);
    font-size: var(--xs);
    padding: 5px 8px;
    text-align: left;
    transition:
      background 0.1s,
      color 0.1s;
  }
  .option:hover {
    background: var(--bg4);
    color: var(--t0);
  }
  .option.active {
    color: var(--ac);
  }

  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 1px solid var(--bd2);
    flex-shrink: 0;
  }

  .swatch[data-theme-preview='dark'] {
    background: #080808;
  }
  .swatch[data-theme-preview='light'] {
    background: #f7f7f7;
  }
  .swatch[data-theme-preview='nord'] {
    background: #2e3440;
  }
  .swatch[data-theme-preview='dracula'] {
    background: #282a36;
  }
  .swatch[data-theme-preview='catppuccin'] {
    background: #1e1e2e;
  }
</style>
