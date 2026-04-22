<script lang="ts">
  import { createEventDispatcher, onMount, tick } from 'svelte';

  type SelectItem = { id: string; name: string };

  export let items: SelectItem[] = [];
  export let value: string = '';
  export let placeholder: string = 'Select...';
  export let disabled: boolean = false;

  const dispatch = createEventDispatcher<{ change: { id: string } }>();

  let open = false;
  let query = '';
  let highlightIndex = -1;
  let containerEl: HTMLElement;
  let inputEl: HTMLInputElement;
  let listEl: HTMLElement;

  $: selectedItem = items.find((i) => i.id === value) ?? null;
  $: inputValue = open ? query : (selectedItem?.name ?? '');

  $: filtered = items.filter(
    (i) =>
      query === '' ||
      i.name.toLowerCase().includes(query.toLowerCase()) ||
      i.id.toLowerCase().includes(query.toLowerCase())
  );

  function openDropdown() {
    if (disabled || open) return;
    open = true;
    query = '';
    highlightIndex = items.findIndex((i) => i.id === value);
    tick().then(() => {
      inputEl?.focus();
      inputEl?.select();
    });
  }

  function close() {
    open = false;
    query = '';
    highlightIndex = -1;
  }

  function handleInput(e: Event) {
    if (!open) {
      openDropdown();
      return;
    }
    query = (e.target as HTMLInputElement).value;
  }

  function selectItem(item: SelectItem) {
    value = item.id;
    dispatch('change', { id: item.id });
    close();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!open) {
      if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') {
        e.preventDefault();
        openDropdown();
      }
      return;
    }

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        highlightIndex = Math.min(highlightIndex + 1, filtered.length - 1);
        scrollIntoView();
        break;
      case 'ArrowUp':
        e.preventDefault();
        highlightIndex = Math.max(highlightIndex - 1, 0);
        scrollIntoView();
        break;
      case 'Enter':
        e.preventDefault();
        if (highlightIndex >= 0 && highlightIndex < filtered.length) {
          selectItem(filtered[highlightIndex]);
        }
        break;
      case 'Escape':
        e.preventDefault();
        close();
        break;
    }
  }

  function scrollIntoView() {
    tick().then(() => {
      const el = listEl?.children[highlightIndex] as HTMLElement | undefined;
      el?.scrollIntoView({ block: 'nearest' });
    });
  }

  function handleClickOutside(e: MouseEvent) {
    if (containerEl && !containerEl.contains(e.target as Node)) {
      close();
    }
  }

  onMount(() => {
    document.addEventListener('mousedown', handleClickOutside, true);
    return () => document.removeEventListener('mousedown', handleClickOutside, true);
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="search-select" bind:this={containerEl} on:keydown={handleKeydown}>
  <input
    class="input"
    bind:this={inputEl}
    value={inputValue}
    {placeholder}
    {disabled}
    on:input={handleInput}
    on:focus={openDropdown}
    on:click={openDropdown}
    on:blur={() => {}}
    role="combobox"
    aria-expanded={open}
    aria-controls="search-select-list"
    aria-haspopup="listbox"
    readonly={!open}
  />
  <span class="chevron" class:open>{'▾'}</span>
  {#if open && filtered.length > 0}
    <ul class="dropdown" bind:this={listEl} role="listbox" id="search-select-list">
      {#each filtered as item, i}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <li
          class="option"
          class:selected={item.id === value}
          class:highlight={i === highlightIndex}
          on:click={() => selectItem(item)}
          on:mouseenter={() => (highlightIndex = i)}
          role="option"
          aria-selected={item.id === value}
        >
          <span class="option-name">{item.name}</span>
          <span class="option-id">{item.id}</span>
        </li>
      {/each}
    </ul>
  {:else if open && filtered.length === 0}
    <ul class="dropdown" role="listbox">
      <li class="no-results">No results for "{query}"</li>
    </ul>
  {/if}
</div>

<style>
  .search-select {
    position: relative;
  }
  .input {
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    color: var(--t0);
    font-size: var(--md);
    padding: var(--sp-3) var(--sp-8) var(--sp-3) var(--sp-4);
    outline: none;
    width: 100%;
    transition: border-color 0.15s;
    cursor: pointer;
  }
  .input:focus {
    border-color: var(--bd2);
    cursor: text;
  }
  .input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .chevron {
    position: absolute;
    right: var(--sp-4);
    top: 50%;
    transform: translateY(-50%);
    color: var(--t3);
    font-size: 10px;
    pointer-events: none;
    transition: transform 0.15s;
  }
  .chevron.open {
    transform: translateY(-50%) rotate(180deg);
  }
  .dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    max-height: 200px;
    overflow-y: auto;
    background: var(--bg2);
    border: 1px solid var(--bd2);
    border-radius: var(--radius-sm);
    list-style: none;
    margin: 0;
    padding: 0;
    z-index: 100;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }
  .option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-2) var(--sp-4);
    cursor: pointer;
    font-size: var(--xs);
    color: var(--t1);
    border-bottom: 1px solid var(--bd);
  }
  .option:hover,
  .option.highlight {
    background: var(--bg3);
    color: var(--t0);
  }
  .option.selected {
    background: rgba(0, 212, 126, 0.08);
    color: var(--ac);
  }
  .option-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .option-id {
    color: var(--t3);
    font-size: 10px;
    margin-left: var(--sp-3);
    flex-shrink: 0;
  }
  .no-results {
    padding: var(--sp-4);
    font-size: var(--xs);
    color: var(--t3);
    text-align: center;
  }
</style>
