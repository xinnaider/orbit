<script lang="ts">
  import {
    splitLayout,
    openPane,
    closePane,
    focusPane,
    assignSession,
    getDraggingSession,
  } from '../lib/stores/layout';
  import type { PaneId } from '../lib/stores/layout';
  import type { Session } from '../lib/stores/sessions';
  import CentralPanel from './CentralPanel.svelte';

  export let paneId: PaneId;
  export let session: Session | null;
  export let focused: boolean;
  export let canClose: boolean;
  export let atMaxPanes: boolean;

  type Zone = 'center' | 'top' | 'bottom' | 'left' | 'right';

  // Which pane is adjacent in each direction for each pane ID
  const adjacent: Record<PaneId, Partial<Record<Zone, PaneId>>> = {
    tl: { right: 'tr', bottom: 'bl' },
    tr: { left: 'tl', bottom: 'br' },
    bl: { right: 'br', top: 'tl' },
    br: { left: 'bl', top: 'tr' },
  };

  let activeZone: Zone | null = null;

  function getZone(e: DragEvent, el: HTMLElement): Zone {
    const rect = el.getBoundingClientRect();
    const relX = e.clientX - rect.left;
    const relY = e.clientY - rect.top;
    const pctX = relX / rect.width;
    const pctY = relY / rect.height;
    if (pctX < 0.2) return 'left';
    if (pctX > 0.8) return 'right';
    if (pctY < 0.2) return 'top';
    if (pctY > 0.8) return 'bottom';
    return 'center';
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    const zone = getZone(e, e.currentTarget as HTMLElement);
    const targetPane = adjacent[paneId][zone];
    const wouldOpenNew = targetPane && !$splitLayout.visible.includes(targetPane);
    if (wouldOpenNew && atMaxPanes) {
      activeZone = 'center';
    } else {
      activeZone = zone;
    }
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
  }

  function onDragLeave(e: DragEvent) {
    // Only clear when leaving the pane entirely (not entering a child element)
    const related = e.relatedTarget as Node | null;
    if (related && (e.currentTarget as HTMLElement).contains(related)) return;
    activeZone = null;
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    const raw = e.dataTransfer?.getData('text/plain') ?? '';
    let sessionId = parseInt(raw, 10);
    // WebView2 fallback: getData() returns empty string in drop events
    if (isNaN(sessionId)) {
      const fallback = getDraggingSession();
      if (fallback !== null) sessionId = fallback;
    }
    if (isNaN(sessionId)) {
      activeZone = null;
      return;
    }

    const zone = getZone(e, e.currentTarget as HTMLElement);
    activeZone = null;

    const targetPane = adjacent[paneId][zone];
    const isAlreadyVisible = targetPane && $splitLayout.visible.includes(targetPane);
    const maxPanesReached = $splitLayout.visible.length >= 4;

    if (zone === 'center' || !targetPane || (maxPanesReached && !isAlreadyVisible)) {
      assignSession(paneId, sessionId);
    } else {
      openPane(targetPane, sessionId);
    }
  }

  function onClick() {
    focusPane(paneId);
  }

  // Map pane ID to CSS grid position (row-start / col-start / row-end / col-end)
  const gridArea: Record<PaneId, string> = {
    tl: '1 / 1 / 2 / 2',
    tr: '1 / 2 / 2 / 3',
    bl: '2 / 1 / 3 / 2',
    br: '2 / 2 / 3 / 3',
  };

  $: dropShadow = (() => {
    if (!activeZone || activeZone === 'center') return '';
    const c = 'var(--ac)';
    const shadows: Record<Zone, string> = {
      center: '',
      top: `box-shadow:inset 0 2px 0 ${c};`,
      bottom: `box-shadow:inset 0 -2px 0 ${c};`,
      left: `box-shadow:inset 2px 0 0 ${c};`,
      right: `box-shadow:inset -2px 0 0 ${c};`,
    };
    return shadows[activeZone];
  })();
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
<div
  class="pane"
  class:focused
  tabindex="0"
  style="grid-area:{gridArea[paneId]};{dropShadow}"
  role="region"
  aria-label="pane {paneId}"
  on:click={onClick}
  on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && onClick()}
  on:dragover={onDragOver}
  on:dragleave={onDragLeave}
  on:drop={onDrop}
>
  {#if canClose}
    <button class="close-btn" title="Close pane" on:click|stopPropagation={() => closePane(paneId)}
      >×</button
    >
  {/if}

  {#if session}
    <CentralPanel {session} />
  {:else}
    <div class="empty">
      <span class="plus">+</span>
      <span class="hint">drag a session here</span>
    </div>
  {/if}
</div>

<style>
  .pane {
    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--bg);
  }

  .pane.focused {
    outline: 1px solid var(--bd2);
    outline-offset: -1px;
  }

  .close-btn {
    position: absolute;
    top: 4px;
    right: 6px;
    z-index: 10;
    background: var(--bg3);
    border: 1px solid var(--bd1);
    color: var(--t1);
    width: 18px;
    height: 18px;
    border-radius: 3px;
    font-size: 13px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .pane:hover .close-btn {
    opacity: 1;
  }

  .close-btn:hover {
    border-color: var(--s-error);
    color: var(--s-error);
  }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }

  .plus {
    font-size: 28px;
    color: var(--t3);
  }

  .hint {
    font-size: var(--xs);
    color: var(--t3);
  }
</style>
