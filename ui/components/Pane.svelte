<script lang="ts">
  import { closePane, focusPane, splitPane } from '../lib/stores/layout';
  import type { PaneId } from '../lib/stores/layout';
  import type { Session } from '../lib/stores/sessions';
  import CentralPanel from './CentralPanel.svelte';

  export let paneId: PaneId;
  export let gridArea: string;
  export let session: Session | null;
  export let focused: boolean;
  export let canClose: boolean;
  export let atMaxPanes: boolean;
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
<div
  class="pane"
  class:focused={focused && canClose}
  tabindex="0"
  style="grid-area:{gridArea}"
  role="region"
  aria-label="pane {paneId}"
  on:click={() => focusPane(paneId)}
  on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && focusPane(paneId)}
>
  {#if session}
    <CentralPanel
      {session}
      onSplit={atMaxPanes ? null : () => splitPane(paneId)}
      onClose={canClose ? () => closePane(paneId) : null}
    />
  {:else}
    <div class="empty">
      <span class="plus">+</span>
      <span class="hint">focus here and click a session</span>
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

  .pane.focused::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--ac);
    z-index: 5;
    pointer-events: none;
  }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sp-3);
  }

  .plus {
    font-size: 28px;
    color: var(--t3);
  }

  .hint {
    font-size: var(--xs);
    color: var(--t3);
    text-align: center;
    max-width: 160px;
  }
</style>
