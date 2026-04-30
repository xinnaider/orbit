<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { MESH_DEFAULT_PROVIDER, isMeshSupportedProvider } from '../../lib/stores/mesh/constants';
  import { backends } from '../../lib/stores/providers';
  import Modal from '../shared/Modal.svelte';

  const dispatch = createEventDispatcher<{
    submit: { name: string; provider: string };
    cancel: void;
  }>();

  let name = '';
  let providerId = MESH_DEFAULT_PROVIDER;

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    return { destroy() {} };
  }

  function submit() {
    const trimmed = name.trim();
    if (!trimmed) return;
    dispatch('submit', { name: trimmed, provider: providerId });
  }
</script>

<Modal title="new graph" width="520px" zIndex={200} on:close={() => dispatch('cancel')}>
  <div class="field">
    <label class="label" for="graph-name">graph name</label>
    <input
      id="graph-name"
      class="input"
      bind:value={name}
      placeholder="e.g. main pipeline"
      use:focusOnMount
      on:keydown={(e) => e.key === 'Enter' && submit()}
    />
  </div>

  <div class="field">
    <span class="label">provider</span>
    <div class="provider-row">
      {#each $backends as b (b.id)}
        {@const supported = isMeshSupportedProvider(b.id)}
        <button
          type="button"
          class="provider-chip"
          class:active={providerId === b.id}
          disabled={!supported}
          title={supported ? '' : 'mesh v1 supports claude code only'}
          on:click={() => (providerId = b.id)}
        >
          {b.name}
        </button>
      {/each}
    </div>
    <span class="hint">every agent in this graph will run on the chosen provider.</span>
  </div>

  <div class="actions">
    <button class="btn ghost" on:click={() => dispatch('cancel')}>cancel</button>
    <button class="btn primary" on:click={submit} disabled={!name.trim()}>create</button>
  </div>
</Modal>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
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
    font-family: inherit;
    outline: none;
  }
  .input:focus {
    border-color: var(--ac);
  }
  .provider-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
  }
  .provider-chip {
    background: var(--bg);
    color: var(--t1);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: var(--sp-2) var(--sp-5);
    font-size: var(--sm);
    font-family: inherit;
    cursor: pointer;
    text-transform: lowercase;
  }
  .provider-chip:hover:not(:disabled) {
    border-color: var(--ac);
    color: var(--ac);
  }
  .provider-chip.active {
    background: var(--ac-d);
    border-color: var(--ac);
    color: var(--ac);
  }
  .provider-chip:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .hint {
    font-size: var(--xs);
    color: var(--t2);
    line-height: 1.4;
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
    cursor: pointer;
    font-family: inherit;
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
