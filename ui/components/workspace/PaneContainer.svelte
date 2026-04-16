<script lang="ts">
  import {
    workspace,
    focusPane,
    moveTab,
    splitPane,
    addTab,
    closeTab,
  } from '../../lib/stores/workspace';
  import type { TabTarget } from '../../lib/stores/workspace';
  import { sessions } from '../../lib/stores/sessions';
  import TabBar from './TabBar.svelte';
  import SplitDropZone from './SplitDropZone.svelte';
  import CentralPanel from '../CentralPanel.svelte';
  import TerminalPanel from '../TerminalPanel.svelte';

  export let paneId: string;

  $: pane = $workspace.panes[paneId];
  $: isFocused = $workspace.focusedPaneId === paneId;
  $: activeTab = pane?.tabs.find((t) => t.id === pane.activeTabId) ?? null;
  $: activeSession =
    activeTab?.target.kind === 'agent'
      ? ($sessions.find((s) => s.id === activeTab!.target.sessionId) ?? null)
      : null;

  let dragOver = false;
  let dragEnterCount = 0;

  function handleDragEnter(e: DragEvent) {
    e.preventDefault();
    dragEnterCount++;
    dragOver = true;
  }

  function handleDragLeave() {
    dragEnterCount--;
    if (dragEnterCount <= 0) {
      dragEnterCount = 0;
      dragOver = false;
    }
  }

  function handleDrop() {
    dragEnterCount = 0;
    dragOver = false;
  }

  function handleSplitDrop(e: CustomEvent<{ position: 'center' | 'top' | 'bottom' | 'left' | 'right'; data: string }>) {
    dragOver = false;
    dragEnterCount = 0;

    const { position, data } = e.detail;
    let parsed: { tabId?: string; sourcePaneId?: string; sessionId?: number } = {};
    try {
      parsed = JSON.parse(data);
    } catch {
      return;
    }

    if (position === 'center') {
      if (parsed.tabId && parsed.sourcePaneId) {
        moveTab(parsed.sourcePaneId, paneId, parsed.tabId);
      } else if (parsed.sessionId != null) {
        addTab(paneId, { kind: 'agent', sessionId: parsed.sessionId });
      }
    } else {
      const direction: 'horizontal' | 'vertical' =
        position === 'left' || position === 'right' ? 'horizontal' : 'vertical';

      let target: TabTarget | null = null;
      if (parsed.tabId && parsed.sourcePaneId) {
        const sourcePane = $workspace.panes[parsed.sourcePaneId];
        const tab = sourcePane?.tabs.find((t) => t.id === parsed.tabId);
        if (tab) target = tab.target;
      } else if (parsed.sessionId != null) {
        target = { kind: 'agent', sessionId: parsed.sessionId };
      }

      if (target) {
        splitPane(paneId, direction, target);
        if (parsed.tabId && parsed.sourcePaneId) {
          closeTab(parsed.sourcePaneId, parsed.tabId);
        }
      }
    }
  }

  function handleAddAction(e: CustomEvent<{ action: 'terminal' | 'session' | 'open' }>) {
    const { action } = e.detail;
    if (action === 'terminal') {
      addTab(paneId, { kind: 'terminal', terminalId: crypto.randomUUID().slice(0, 8) });
    } else if (action === 'session') {
      window.dispatchEvent(new CustomEvent('orbit:new-session'));
    }
    // 'open' is a no-op for now
  }

  function handlePaneClick() {
    focusPane(paneId);
  }
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="pane-container"
  class:focused={isFocused}
  on:click={handlePaneClick}
  on:dragenter={handleDragEnter}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
>
  {#if pane}
    <TabBar
      {paneId}
      tabs={pane.tabs}
      activeTabId={pane.activeTabId}
      on:addaction={handleAddAction}
    />

    <div class="pane-content">
      {#if activeTab}
        {#if activeTab.target.kind === 'agent'}
          {#if activeSession}
            <CentralPanel session={activeSession} onSplit={null} onClose={null} />
          {:else}
            <div class="empty-state">
              <span class="icon">⚠</span>
              <span class="hint">Session not found</span>
            </div>
          {/if}
        {:else if activeTab.target.kind === 'terminal'}
          <TerminalPanel sessionId={0} />
        {/if}
      {:else}
        <div class="empty-state">
          <span class="icon">+</span>
          <span class="hint">Open a session or terminal to get started</span>
        </div>
      {/if}

      <SplitDropZone visible={dragOver} on:drop={handleSplitDrop} />
    </div>
  {/if}
</div>

<style>
  .pane-container {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--bg);
    position: relative;
  }

  .pane-container::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--ac);
    opacity: 0;
    transition: opacity 0.15s;
    z-index: 1;
    pointer-events: none;
  }

  .pane-container.focused::before {
    opacity: 1;
  }

  .pane-content {
    display: flex;
    flex: 1;
    min-height: 0;
    position: relative;
    overflow: hidden;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: 8px;
    color: var(--t3);
  }

  .empty-state .icon {
    font-size: 2rem;
    line-height: 1;
  }

  .empty-state .hint {
    font-size: var(--xs);
  }
</style>
