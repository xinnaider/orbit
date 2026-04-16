<script lang="ts">
  import {
    workspace,
    focusPane,
    assignSession,
    splitPane,
    closePane,
    moveSession,
  } from '../../lib/stores/workspace';
  import { sessions } from '../../lib/stores/sessions';
  import SplitDropZone from './SplitDropZone.svelte';
  import CentralPanel from '../CentralPanel.svelte';

  export let paneId: string;

  $: pane = $workspace.panes[paneId];
  $: isFocused = $workspace.focusedPaneId === paneId;
  $: canClose = Object.keys($workspace.panes).length > 1;
  $: session = (() => {
    if (!pane?.sessionId) return null;
    return $sessions.find((s) => s.id === pane.sessionId) ?? null;
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

    let parsed: { sessionId?: number; sourcePaneId?: string } = {};
    try {
      parsed = JSON.parse(e.detail.data);
    } catch {
      return;
    }

    const sid = parsed.sessionId ?? null;

    if (e.detail.position === 'center') {
      if (sid !== null) {
        if (parsed.sourcePaneId && parsed.sourcePaneId !== paneId) {
          moveSession(parsed.sourcePaneId, paneId);
        } else {
          assignSession(paneId, sid);
        }
      }
    } else {
      const direction: 'horizontal' | 'vertical' =
        e.detail.position === 'left' || e.detail.position === 'right' ? 'horizontal' : 'vertical';
      splitPane(paneId, direction, sid);
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
  {#if session}
    <CentralPanel {session} {paneId} onClose={canClose ? () => closePane(paneId) : null} />
  {:else}
    <div class="empty-state">
      <span class="icon">+</span>
      <span class="hint">click a session in the sidebar</span>
    </div>
  {/if}

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
