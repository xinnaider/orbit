<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Modal from '../shared/Modal.svelte';

  export let title: string;
  export let label: string;
  export let placeholder = '';
  export let initialValue = '';
  export let confirmLabel = 'Ok';

  const dispatch = createEventDispatcher<{ submit: string; cancel: void }>();

  let value = initialValue;

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    node.select();
    return { destroy() {} };
  }

  function submit() {
    const trimmed = value.trim();
    if (!trimmed) return;
    dispatch('submit', trimmed);
  }
</script>

<Modal {title} width="420px" zIndex={200} on:close={() => dispatch('cancel')}>
  <div class="field">
    <label class="label" for="mesh-prompt-input">{label}</label>
    <input
      id="mesh-prompt-input"
      class="input"
      bind:value
      {placeholder}
      use:focusOnMount
      on:keydown={(e) => e.key === 'Enter' && submit()}
    />
  </div>

  <div class="actions">
    <button class="btn ghost" on:click={() => dispatch('cancel')}>cancel</button>
    <button class="btn primary" on:click={submit} disabled={!value.trim()}>
      {confirmLabel}
    </button>
  </div>
</Modal>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  .label {
    font-size: var(--sm);
    color: var(--t1);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .input {
    background: var(--bg);
    color: var(--t0);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: var(--sp-4) var(--sp-5);
    font-size: var(--base);
    outline: none;
  }
  .input:focus {
    border-color: var(--ac);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-4);
    margin-top: var(--sp-3);
  }
  .btn {
    padding: var(--sp-3) var(--sp-7);
    border-radius: var(--radius-sm);
    font-size: var(--sm);
    border: 1px solid var(--bd1);
    text-transform: lowercase;
    letter-spacing: 0.3px;
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
  .btn.primary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
