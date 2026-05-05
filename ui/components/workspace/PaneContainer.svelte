<script lang="ts">
  import {
    addTab,
    closeTab,
    createTab,
    workspace,
    focusPane,
    splitPane,
    closePane,
    moveTab,
    type Tab,
  } from '../../lib/stores/workspace';
  import { sessions } from '../../lib/stores/sessions';
  import SplitDropZone from './SplitDropZone.svelte';
  import TabBar from './TabBar.svelte';
  import CentralPanel from '../CentralPanel.svelte';
  import GitPanel from '../GitPanel.svelte';
  import TerminalPanel from '../TerminalPanel.svelte';

  export let paneId: string;

  $: pane = $workspace.panes[paneId];
  $: isFocused = $workspace.focusedPaneId === paneId;
  $: canClose = Object.keys($workspace.panes).length > 1;
  $: activeTab = pane?.tabs.find((tab) => tab.id === pane.activeTabId) ?? pane?.tabs[0] ?? null;
  $: session = (() => {
    const target = activeTab?.target;
    if (target?.kind !== 'agent') return null;
    return $sessions.find((s) => s.id === target.sessionId) ?? null;
  })();

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

  function handleSplitDrop(
    e: CustomEvent<{
      position: 'center' | 'top' | 'bottom' | 'left' | 'right';
      data: string;
    }>
  ) {
    dragOver = false;
    dragEnterCount = 0;

    const parsed = tabFromDropData(e.detail.data);
    if (!parsed) return;

    if (e.detail.position === 'center') {
      if (parsed.sourcePaneId && parsed.sourceTabId) {
        moveTab(parsed.sourcePaneId, paneId, parsed.sourceTabId);
      } else {
        addTab(paneId, parsed.tab.target);
      }
      return;
    }

    const direction: 'horizontal' | 'vertical' =
      e.detail.position === 'left' || e.detail.position === 'right' ? 'horizontal' : 'vertical';
    const insertBefore = e.detail.position === 'left' || e.detail.position === 'top';
    splitPane(paneId, direction, parsed.tab, insertBefore);
  }

  function tabFromDropData(
    data: string
  ): { tab: Tab; sourcePaneId: string | null; sourceTabId: string | null } | null {
    try {
      const parsed = JSON.parse(data) as {
        tabId?: string;
        sourcePaneId?: string;
        sessionId?: number;
        target?: Tab['target'];
      };

      if (parsed.target) {
        return {
          tab: createTab(parsed.target),
          sourcePaneId: parsed.sourcePaneId ?? null,
          sourceTabId: parsed.tabId ?? null,
        };
      }

      if (typeof parsed.sessionId === 'number') {
        return {
          tab: createTab({ kind: 'agent', sessionId: parsed.sessionId }),
          sourcePaneId: parsed.sourcePaneId ?? null,
          sourceTabId: null,
        };
      }
    } catch {
      const sessionId = Number(data);
      if (Number.isFinite(sessionId)) {
        return { tab: createTab({ kind: 'agent', sessionId }), sourcePaneId: null, sourceTabId: null };
      }
    }

    return null;
  }

  function handleAddAction(action: 'terminal' | 'session' | 'open' | 'git') {
    const cwd = session?.cwd ?? (activeTab?.target.kind === 'git' ? activeTab.target.cwd : '.');
    if (action === 'terminal') {
      addTab(paneId, { kind: 'terminal', terminalId: crypto.randomUUID(), cwd });
    } else if (action === 'git') {
      addTab(paneId, { kind: 'git', cwd });
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="pane-container"
  class:focused={isFocused}
  role="region"
  on:click={() => focusPane(paneId)}
  on:keydown={() => focusPane(paneId)}
  on:dragenter={handleDragEnter}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
>
  {#if pane}
    <TabBar
      {paneId}
      tabs={pane.tabs}
      activeTabId={pane.activeTabId}
      focused={isFocused}
      on:addaction={(e) => handleAddAction(e.detail.action)}
    />
  {/if}

  <div class="pane-content">
  {#if activeTab?.target.kind === 'git'}
    <GitPanel cwd={activeTab.target.cwd} {paneId} focused={isFocused} onClose={() => closeTab(paneId, activeTab.id)} />
  {:else if activeTab?.target.kind === 'terminal'}
    <TerminalPanel
      terminalId={activeTab.target.terminalId}
      cwd={activeTab.target.cwd}
      {paneId}
      focused={isFocused}
      onClose={() => closeTab(paneId, activeTab.id)}
    />
  {:else if activeTab?.target.kind === 'agent' && session}
    <CentralPanel {session} {paneId} focused={isFocused} onClose={canClose ? () => closePane(paneId) : null} />
  {:else}
    <div class="empty-state">
      <span class="icon">+</span>
      <span class="hint">click a session in the sidebar</span>
    </div>
  {/if}
  </div>

  <SplitDropZone visible={dragOver} on:drop={handleSplitDrop} />
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
    padding-top: var(--sp-2);
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
    z-index: 2;
    pointer-events: none;
  }

  .pane-container.focused::before {
    opacity: 1;
  }

  .pane-content {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: var(--sp-4);
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
