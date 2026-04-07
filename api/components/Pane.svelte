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
  class:focused
  tabindex="0"
  style="grid-area:{gridArea}"
  role="region"
  aria-label="pane {paneId}"
  on:click={() => focusPane(paneId)}
  on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && focusPane(paneId)}
>
  {#if canClose}
    <button
      class="close-btn"
      title="Fechar painel"
      on:click|stopPropagation={() => closePane(paneId)}>×</button
    >
  {/if}

  {#if session}
    <CentralPanel {session} onSplit={atMaxPanes ? null : () => splitPane(paneId)} />
  {:else}
    <div class="empty">
      <span class="plus">+</span>
      <span class="hint">foque aqui e clique em uma sessão</span>
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

  .close-btn {
    position: absolute;
    top: 6px;
    right: 8px;
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
    text-align: center;
    max-width: 160px;
  }
</style>
