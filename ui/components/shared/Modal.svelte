<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let title: string = '';
  export let width: string = '500px';
  export let zIndex: number = 100;
  export let closeOnOverlayClick: boolean = true;
  export let modalStyle: string = '';
  export let overlayBg: string = 'rgba(0, 0, 0, 0.7)';

  const dispatch = createEventDispatcher();

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') dispatch('close');
  }
</script>

<svelte:window on:keydown={onKey} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  style="z-index: {zIndex}; background: {overlayBg}"
  on:click|self={() => {
    if (closeOnOverlayClick) dispatch('close');
  }}
  on:keydown={onKey}
>
  <div class="modal" style="width: {width}; {modalStyle}">
    {#if title}
      <div class="modal-header">
        <span class="modal-title">{title}</span>
        <button class="close" on:click={() => dispatch('close')}>✕</button>
      </div>
    {/if}
    <slot />
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .modal {
    background: var(--bg1);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-md);
    max-width: 94vw;
    max-height: 90vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--sp-7);
    padding: var(--sp-9);
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .modal-title {
    font-size: var(--md);
    color: var(--t1);
    letter-spacing: 0.06em;
  }
  .close {
    background: none;
    border: none;
    color: var(--t2);
    font-size: 12px;
    padding: var(--sp-1) var(--sp-2);
    cursor: pointer;
  }
  .close:hover {
    color: var(--t0);
  }
</style>
