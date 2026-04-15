<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { renameSession } from '../lib/tauri';
  import { parseSessionName } from '../lib/android-names';
  import Modal from './shared/Modal.svelte';

  export let sessionId: number;
  export let sessionName: string;

  const dispatch = createEventDispatcher<{ done: { id: number; name: string }; cancel: void }>();

  const parsed = parseSessionName(sessionName);
  let prefix = parsed.prefix;
  let suffix = parsed.suffix;
  let loading = false;

  $: namePreview =
    prefix.trim() && suffix.trim()
      ? `${prefix.trim()} · ${suffix.trim()}`
      : prefix.trim() || suffix.trim();

  async function submit() {
    const finalName = namePreview;
    if (!finalName) return;
    loading = true;
    try {
      await renameSession(sessionId, finalName);
      dispatch('done', { id: sessionId, name: finalName });
    } finally {
      loading = false;
    }
  }

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    node.select();
    return { destroy() {} };
  }
</script>

<Modal title="rename session" width="400px" zIndex={200} on:close={() => dispatch('cancel')}>
  <div class="field">
    <label class="label" for="rn-prefix">apelido</label>
    <div class="nickname-row">
      <input
        id="rn-prefix"
        class="input"
        bind:value={prefix}
        placeholder="agent name"
        disabled={loading}
        use:focusOnMount
        on:keydown={(e) => e.key === 'Enter' && submit()}
      />
      <span class="nick-sep">·</span>
      <input
        id="rn-suffix"
        class="input"
        bind:value={suffix}
        placeholder="project"
        disabled={loading}
        on:keydown={(e) => e.key === 'Enter' && submit()}
      />
    </div>
    {#if namePreview}
      <span class="name-preview">{namePreview}</span>
    {/if}
  </div>

  <div class="actions">
    <button class="btn ghost" on:click={() => dispatch('cancel')} disabled={loading}>
      cancel
    </button>
    <button class="btn primary" on:click={submit} disabled={loading || !namePreview}>
      {loading ? 'saving...' : 'rename'}
    </button>
  </div>
</Modal>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }
  .label {
    font-size: var(--xs);
    color: var(--t2);
    letter-spacing: 0.06em;
  }
  .input {
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    color: var(--t0);
    font-size: var(--md);
    padding: var(--sp-3) var(--sp-4);
    outline: none;
    width: 100%;
    transition: border-color 0.15s;
    font-family: var(--mono);
  }
  .input:focus {
    border-color: var(--bd2);
  }
  .input:disabled {
    opacity: 0.5;
  }
  .nickname-row {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }
  .nickname-row .input {
    flex: 1;
  }
  .nick-sep {
    color: var(--t3);
    font-size: var(--md);
    flex-shrink: 0;
  }
  .name-preview {
    font-size: var(--xs);
    color: var(--t3);
    letter-spacing: 0.03em;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-4);
  }
  .btn {
    background: none;
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    color: var(--t1);
    font-size: var(--sm);
    padding: var(--sp-3) var(--sp-7);
    transition: all 0.15s;
    font-family: var(--mono);
    cursor: pointer;
  }
  .btn:hover {
    border-color: var(--bd2);
    color: var(--t0);
  }
  .btn.primary {
    background: var(--ac-d);
    border-color: var(--ac);
    color: var(--ac);
  }
  .btn.primary:hover {
    background: rgba(0, 212, 126, 0.18);
  }
  .btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
