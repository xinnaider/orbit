<script lang="ts">
  import type { UpdateInfo } from '../lib/types';
  import { installUpdate } from '../lib/tauri';
  import Banner from './Banner.svelte';

  export let update: UpdateInfo;
  export let onDismiss: () => void = () => {};

  let installing = false;
  let installError = '';

  async function install() {
    installing = true;
    installError = '';
    try {
      await installUpdate();
      // app reinicia automaticamente após install
    } catch (e: any) {
      installError = e?.message ?? String(e);
      installing = false;
    }
  }
</script>

<Banner variant="success" position="top" icon="↑" zIndex={200} {onDismiss}>
  <div class="title">
    new version available — <span class="version">{update.version}</span>
  </div>
  {#if update.body}
    <div class="notes">{update.body}</div>
  {/if}
  {#if installError}
    <div class="install-error">{installError}</div>
  {/if}

  <svelte:fragment slot="actions">
    <button class="update-btn" on:click={install} disabled={installing}>
      {installing ? 'installing...' : 'update now'}
    </button>
  </svelte:fragment>
</Banner>

<style>
  .title {
    font-size: var(--sm);
    color: var(--t1);
    font-weight: 500;
  }
  .version {
    color: var(--ac);
  }
  .notes {
    font-size: var(--xs);
    color: var(--t2);
    margin-top: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .install-error {
    font-size: var(--xs);
    color: var(--s-error);
    margin-top: 2px;
  }
  .update-btn {
    background: var(--ac-d);
    border: 1px solid var(--ac);
    border-radius: 3px;
    color: var(--ac);
    font-size: var(--xs);
    padding: 5px 14px;
    flex-shrink: 0;
    letter-spacing: 0.04em;
    transition: background 0.15s;
  }
  .update-btn:hover:not(:disabled) {
    background: rgba(0, 212, 126, 0.18);
  }
  .update-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
