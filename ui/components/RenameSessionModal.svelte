<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { renameSession } from '../lib/tauri';
  import { parseSessionName } from '../lib/android-names';

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

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') dispatch('cancel');
  }

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    node.select();
    return { destroy() {} };
  }
</script>

<svelte:window on:keydown={onKey} />

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  on:click|self={() => dispatch('cancel')}
  on:keydown={onKey}
>
  <div class="modal">
    <div class="modal-header">
      <span class="modal-title">rename session</span>
      <button class="close" on:click={() => dispatch('cancel')}>✕</button>
    </div>

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
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .modal {
    background: var(--bg1);
    border: 1px solid var(--bd1);
    border-radius: 4px;
    width: 400px;
    max-width: 94vw;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px;
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
    padding: 2px 4px;
    cursor: pointer;
  }
  .close:hover {
    color: var(--t0);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .label {
    font-size: var(--xs);
    color: var(--t2);
    letter-spacing: 0.06em;
  }
  .input {
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: 3px;
    color: var(--t0);
    font-size: var(--md);
    padding: 6px 8px;
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
    gap: 6px;
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
    gap: 8px;
  }
  .btn {
    background: none;
    border: 1px solid var(--bd1);
    border-radius: 3px;
    color: var(--t1);
    font-size: var(--sm);
    padding: 5px 14px;
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
