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
    addaction: { action: 'terminal' | 'session' | 'open' | 'git' };
  }>();

  let menuOpen = false;
  let menuX = 0;
  let menuY = 0;

  // Drag state
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

  function handleMenuSelect(e: CustomEvent<{ action: 'terminal' | 'session' | 'open' | 'git' }>) {
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
      if (parsed.sourcePaneId !== paneId) return; // cross-pane drag — not handled here
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

<div class="tab-bar">
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

    <!-- Drop zone at the end for appending -->
    <div
      class="drop-zone drop-zone-end"
      class:drag-over={dragOverIndex === tabs.length}
      role="none"
      on:dragover={(e) => handleDragOver(e, tabs.length)}
      on:dragleave={handleDragLeave}
      on:drop={(e) => handleDrop(e, tabs.length)}
    ></div>
  </div>

  <button class="add-button" on:click={handleAddClick} aria-label="Add tab" title="Add tab">
    <Plus size={15} />
  </button>
</div>

{#if menuOpen}
  <TabAddMenu x={menuX} y={menuY} on:select={handleMenuSelect} on:close={handleMenuClose} />
{/if}

<style>
  .tab-bar {
    display: flex;
    align-items: center;
    height: 36px;
    padding: 0 6px;
    gap: 3px;
    border-bottom: 1px solid var(--bd);
    background: var(--bg1);
    flex-shrink: 0;
    overflow: hidden;
  }

  .tab-list {
    display: flex;
    align-items: center;
    flex: 1;
    overflow-x: auto;
    scrollbar-width: none;
    gap: 3px;
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
    min-width: 8px;
  }

  .drop-zone.drag-over::before {
    content: '';
    position: absolute;
    left: 0;
    top: 5px;
    bottom: 5px;
    width: 2px;
    background: var(--ac);
    border-radius: 1px;
    pointer-events: none;
  }

  .tab-wrapper {
    display: flex;
    align-items: stretch;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
  }

  .add-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border: 1px dashed var(--bd1);
    background: transparent;
    color: var(--t3);
    cursor: pointer;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    transition: all 0.12s;
  }

  .add-button:hover {
    border-color: var(--ac);
    color: var(--ac);
  }
</style>
