<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { SlashCommand } from '../../lib/types';

  export let commands: SlashCommand[] = [];
  export let text: string = '';
  export let visible: boolean = false;
  export let providerModels: string[] = [];
  export let modelOptions: string[] = [];
  export let supportsEffort: boolean = false;
  export let effortLevels: string[] = ['low', 'medium', 'high', 'max'];
  export let files: string[] = [];
  export let atQuery: string | null = null;

  const dispatch = createEventDispatcher<{
    select: { type: 'cmd' | 'subOption' | 'file'; value: string };
    close: void;
  }>();

  // Sub-options for /model and /effort
  $: subOptions = (() => {
    const lower = text.toLowerCase();
    if (lower.startsWith('/model ')) {
      const arg = text.slice(7).toLowerCase();
      const opts = modelOptions.length > 0 ? modelOptions : providerModels;
      const filtered = arg ? opts.filter((o) => o.toLowerCase().includes(arg)) : opts;
      return filtered.slice(0, 10);
    } else if (lower.startsWith('/effort ') && supportsEffort) {
      const arg = lower.slice(8);
      return effortLevels.filter((o) => o.startsWith(arg));
    }
    return [];
  })();

  $: showSubOptions = visible && subOptions.length > 0;

  $: fileSuggestions =
    atQuery === null
      ? []
      : atQuery === ''
        ? files.slice(0, 10)
        : (() => {
            const q = (atQuery as string).toLowerCase();
            const name = files.filter((f) => f.split('/').pop()!.toLowerCase().includes(q));
            const path = files.filter((f) => !name.includes(f) && f.toLowerCase().includes(q));
            return [...name, ...path].slice(0, 10);
          })();

  $: showFiles = visible && fileSuggestions.length > 0;

  $: suggestions =
    subOptions.length > 0
      ? []
      : text.startsWith('/')
        ? text.length === 1
          ? commands.slice(0, 8)
          : commands.filter((c) => c.cmd.toLowerCase().includes(text.toLowerCase())).slice(0, 8)
        : [];
  $: showSuggestions = visible && suggestions.length > 0;

  let selIdx = 0;
  let subSelIdx = 0;
  let fileSelIdx = 0;

  // Track previous list lengths so we reset the selection only when a list changes,
  // not on every re-render (which would break arrow-key navigation).
  let prevSuggestionsLen = -1;
  let prevSubOptionsLen = -1;
  let prevFileSuggestionsLen = -1;

  $: if (suggestions.length !== prevSuggestionsLen) {
    selIdx = 0;
    prevSuggestionsLen = suggestions.length;
  }
  $: if (subOptions.length !== prevSubOptionsLen) {
    subSelIdx = 0;
    prevSubOptionsLen = subOptions.length;
  }
  $: if (fileSuggestions.length !== prevFileSuggestionsLen) {
    fileSelIdx = 0;
    prevFileSuggestionsLen = fileSuggestions.length;
  }

  $: if (selIdx >= suggestions.length) selIdx = 0;
  $: if (subSelIdx >= subOptions.length) subSelIdx = 0;
  $: if (fileSelIdx >= fileSuggestions.length) fileSelIdx = 0;

  /**
   * Handle keyboard navigation. Returns true if the key was consumed.
   * InputBar should call this in its keydown handler.
   */
  export function handleKey(e: KeyboardEvent): boolean {
    if (showSubOptions) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        subSelIdx = (subSelIdx + 1) % subOptions.length;
        return true;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        subSelIdx = (subSelIdx - 1 + subOptions.length) % subOptions.length;
        return true;
      }
      if (e.key === 'Tab' || (e.key === 'Enter' && !e.shiftKey)) {
        e.preventDefault();
        dispatch('select', { type: 'subOption', value: subOptions[subSelIdx] });
        return true;
      }
      if (e.key === 'Escape') {
        dispatch('close');
        return true;
      }
    }
    if (showFiles) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        fileSelIdx = (fileSelIdx + 1) % fileSuggestions.length;
        return true;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        fileSelIdx = (fileSelIdx - 1 + fileSuggestions.length) % fileSuggestions.length;
        return true;
      }
      if (e.key === 'Tab' || (e.key === 'Enter' && !e.shiftKey)) {
        e.preventDefault();
        dispatch('select', { type: 'file', value: fileSuggestions[fileSelIdx] });
        return true;
      }
      if (e.key === 'Escape') {
        dispatch('close');
        return true;
      }
    }
    if (showSuggestions) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        selIdx = (selIdx + 1) % suggestions.length;
        return true;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        selIdx = (selIdx - 1 + suggestions.length) % suggestions.length;
        return true;
      }
      if (e.key === 'Tab' || (e.key === 'Enter' && !e.shiftKey)) {
        e.preventDefault();
        dispatch('select', { type: 'cmd', value: suggestions[selIdx].cmd });
        return true;
      }
      if (e.key === 'Escape') {
        dispatch('close');
        return true;
      }
    }
    return false;
  }
</script>

{#if showSubOptions}
  <div class="dropdown">
    {#each subOptions as opt, i}
      <button
        class="drop-item"
        class:sel={i === subSelIdx}
        on:click={() => dispatch('select', { type: 'subOption', value: opt })}
      >
        <span class="drop-main">{opt}</span>
      </button>
    {/each}
  </div>
{:else if showFiles || showSuggestions}
  <div class="dropdown">
    {#if showFiles}
      {#each fileSuggestions as f, i}
        <button
          class="drop-item"
          class:sel={i === fileSelIdx}
          on:click={() => dispatch('select', { type: 'file', value: f })}
        >
          <span class="drop-icon">@</span>
          <span class="drop-main">{f.split('/').pop()}</span>
          <span class="drop-sub">{f}</span>
        </button>
      {/each}
    {:else}
      {#each suggestions as s, i}
        <button
          class="drop-item"
          class:sel={i === selIdx}
          on:click={() => dispatch('select', { type: 'cmd', value: s.cmd })}
        >
          <span class="drop-main">{s.cmd}</span>
          <span class="drop-sub">{s.desc}</span>
        </button>
      {/each}
    {/if}
  </div>
{/if}

<style>
  .dropdown {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-bottom: none;
    border-radius: var(--radius-md) var(--radius-md) 0 0;
    max-height: 200px;
    overflow-y: auto;
  }
  .drop-item {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: var(--sp-3) var(--sp-6);
    cursor: pointer;
    border-bottom: 1px solid var(--bd);
  }
  .drop-item:hover,
  .drop-item.sel {
    background: var(--bg3);
  }
  .drop-icon {
    color: var(--ac);
    font-size: var(--xs);
    width: 14px;
    flex-shrink: 0;
  }
  .drop-main {
    font-size: var(--md);
    color: var(--t0);
    font-weight: 500;
    flex-shrink: 0;
  }
  .drop-sub {
    font-size: var(--xs);
    color: var(--t2);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
