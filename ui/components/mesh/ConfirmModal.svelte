<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Modal from '../shared/Modal.svelte';

  export let title: string;
  export let message: string;
  export let confirmLabel = 'delete';
  export let danger = true;

  const dispatch = createEventDispatcher<{ confirm: void; cancel: void }>();
</script>

<Modal {title} width="400px" zIndex={200} on:close={() => dispatch('cancel')}>
  <div class="msg">{message}</div>

  <div class="actions">
    <button class="btn ghost" on:click={() => dispatch('cancel')}>cancel</button>
    <button class="btn {danger ? 'danger' : 'primary'}" on:click={() => dispatch('confirm')}>
      {confirmLabel}
    </button>
  </div>
</Modal>

<style>
  .msg {
    font-size: var(--base);
    color: var(--t0);
    line-height: 1.5;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-4);
  }
  .btn {
    padding: var(--sp-3) var(--sp-7);
    border-radius: var(--radius-sm);
    font-size: var(--sm);
    border: 1px solid var(--bd1);
    text-transform: lowercase;
    letter-spacing: 0.3px;
    cursor: pointer;
  }
  .btn.ghost {
    background: transparent;
    color: var(--t0);
  }
  .btn.ghost:hover {
    background: var(--bg3);
  }
  .btn.primary {
    background: var(--ac);
    color: #000;
    border-color: var(--ac);
  }
  .btn.danger {
    background: var(--s-error);
    color: #fff;
    border-color: var(--s-error);
  }
</style>
