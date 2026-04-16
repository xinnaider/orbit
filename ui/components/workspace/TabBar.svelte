<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { Tab } from '../../lib/stores/workspace';
  import { setActiveTab, closeTab, reorderTab } from '../../lib/stores/workspace';
  import TabItem from './TabItem.svelte';
  import TabAddMenu from './TabAddMenu.svelte';

  export let paneId: string;
  export let tabs: Tab[];
  export let activeTabId: string | null;

  const dispatch = createEventDispatcher<{
    addaction: { action: 'terminal' | 'session' | 'open' };
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

  function handleMenuSelect(e: CustomEvent<{ action: 'terminal' | 'session' | 'open' }>) {
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
    +
  </button>
</div>

{#if menuOpen}
  <TabAddMenu x={menuX} y={menuY} on:select={handleMenuSelect} on:close={handleMenuClose} />
{/if}

<style>
  .tab-bar {
    display: flex;
    align-items: stretch;
    background: var(--bg1);
    border-bottom: 1px solid var(--bd);
    height: 33px;
    flex-shrink: 0;
    overflow: hidden;
  }

  .tab-list {
    display: flex;
    align-items: stretch;
    flex: 1;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .tab-list::-webkit-scrollbar {
    display: none;
  }

  .drop-zone {
    display: flex;
    align-items: stretch;
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
    top: 4px;
    bottom: 4px;
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
    width: 32px;
    height: 32px;
    border: none;
    background: transparent;
    color: var(--t2);
    font-size: var(--md);
    cursor: pointer;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    transition:
      background 0.1s,
      color 0.1s;
    align-self: center;
    margin: 0 var(--sp-2);
  }

  .add-button:hover {
    background: var(--bg2);
    color: var(--t0);
  }
</style>
