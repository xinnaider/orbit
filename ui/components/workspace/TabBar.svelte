<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { Plus } from 'lucide-svelte';
  import type { Tab } from '../../lib/stores/workspace';
  import { setActiveTab, closeTab, reorderTab } from '../../lib/stores/workspace';
  import TabItem from './TabItem.svelte';
  import TabAddMenu from './TabAddMenu.svelte';

  export let paneId: string;
  export let tabs: Tab[];
  export let activeTabId: string | null;
  export let focused: boolean = true;

  const dispatch = createEventDispatcher<{
    addaction: { action: 'terminal' | 'git' };
  }>();

  let menuOpen = false;
  let menuX = 0;
  let menuY = 0;

  let dragOverIndex: number | null = null;

  function handleTabClick(tabId: string) {
    setActiveTab(paneId, tabId);
  }

  function handleTabClose(tabId: string) {
    closeTab(paneId, tabId);
  }

  function handleAddClick(e: MouseEvent) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    menuX = rect.left;
    menuY = rect.bottom + 4;
    menuOpen = true;
  }

  function handleMenuSelect(e: CustomEvent<{ action: 'terminal' | 'git' }>) {
    dispatch('addaction', { action: e.detail.action });
  }

  function handleMenuClose() {
    menuOpen = false;
  }

  function handleDragOver(e: DragEvent, index: number) {
    e.preventDefault();
    const data = e.dataTransfer?.getData('text/plain');
    if (!data) return;
    try {
      const parsed = JSON.parse(data) as { tabId: string; sourcePaneId: string };
      if (parsed.sourcePaneId !== paneId) return;
    } catch {
      return;
    }
    e.dataTransfer!.dropEffect = 'move';
    dragOverIndex = index;
  }

  function handleDragLeave() {
    dragOverIndex = null;
  }

  function handleDrop(e: DragEvent, toIndex: number) {
    e.preventDefault();
    dragOverIndex = null;
    const data = e.dataTransfer?.getData('text/plain');
    if (!data) return;
    try {
      const parsed = JSON.parse(data) as { tabId: string; sourcePaneId: string };
      if (parsed.sourcePaneId !== paneId) return;
      const fromIndex = tabs.findIndex((t) => t.id === parsed.tabId);
      if (fromIndex === -1 || fromIndex === toIndex) return;
      reorderTab(paneId, fromIndex, toIndex);
    } catch {
      // malformed drag data — ignore
    }
  }
</script>

<div class="tab-bar" class:focused>
  <div class="tab-list">
    {#each tabs as tab, index (tab.id)}
      <div
        class="drop-zone"
        class:drag-over={dragOverIndex === index}
        role="none"
        on:dragover={(e) => handleDragOver(e, index)}
        on:dragleave={handleDragLeave}
        on:drop={(e) => handleDrop(e, index)}
      >
        <button
          type="button"
          class="tab-wrapper"
          on:click={() => handleTabClick(tab.id)}
          aria-label="Activate tab"
        >
          <TabItem
            {tab}
            active={tab.id === activeTabId}
            {paneId}
            {focused}
            on:close={(e) => handleTabClose(e.detail.tabId)}
          />
        </button>
      </div>
    {/each}

    <div
      class="drop-zone drop-zone-end"
      class:drag-over={dragOverIndex === tabs.length}
      role="none"
      on:dragover={(e) => handleDragOver(e, tabs.length)}
      on:dragleave={handleDragLeave}
      on:drop={(e) => handleDrop(e, tabs.length)}
    ></div>
  </div>

  <button
    type="button"
    class="add-button"
    on:click={handleAddClick}
    aria-label="Add tab"
    title="Add tab"
  >
    <Plus size={14} />
  </button>
</div>

{#if menuOpen}
  <TabAddMenu x={menuX} y={menuY} on:select={handleMenuSelect} on:close={handleMenuClose} />
{/if}

<style>
  .tab-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 30px;
    padding: 8px 12px 6px;
    background: var(--bg);
    flex-shrink: 0;
    overflow: hidden;
  }

  .tab-bar:not(.focused) {
    opacity: 0.85;
  }

  .tab-list {
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
    gap: 4px;
  }

  .tab-list::-webkit-scrollbar {
    display: none;
  }

  .drop-zone {
    display: flex;
    align-items: center;
    position: relative;
    flex-shrink: 0;
  }

  .drop-zone-end {
    flex: 1;
    min-width: 6px;
    align-self: stretch;
  }

  .drop-zone.drag-over::before {
    content: '';
    position: absolute;
    left: -2px;
    top: 50%;
    transform: translateY(-50%);
    width: 2px;
    height: 18px;
    background: var(--ac);
    border-radius: 1px;
    pointer-events: none;
  }

  .tab-wrapper {
    display: flex;
    align-items: center;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
  }

  .add-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    flex-shrink: 0;
    border: 1px dashed var(--bd1);
    border-radius: 999px;
    background: transparent;
    color: var(--t3);
    cursor: pointer;
    transition:
      color 0.12s,
      border-color 0.12s,
      background 0.12s;
  }

  .add-button:hover {
    color: var(--t1);
    border-color: var(--t2);
    background: color-mix(in srgb, var(--bg2), transparent 50%);
  }
</style>
