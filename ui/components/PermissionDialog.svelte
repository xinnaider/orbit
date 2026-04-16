<script lang="ts">
  import { respondPermission } from '../lib/tauri/attention';

  export let sessionId: number;
  export let toolName: string;
  export let description: string;
  export let onDismiss: () => void = () => {};

  let loading = false;

  async function handleAllow() {
    loading = true;
    try {
      await respondPermission(sessionId, true);
      onDismiss();
    } finally {
      loading = false;
    }
  }

  async function handleDeny() {
    loading = true;
    try {
      await respondPermission(sessionId, false);
      onDismiss();
    } finally {
      loading = false;
    }
  }
</script>

<div class="perm-bar">
  <span class="perm-icon">⚑</span>
  <div class="perm-info">
    <span class="perm-tool">{toolName}</span>
    {#if description}
      <span class="perm-desc">{description}</span>
    {/if}
  </div>
  <div class="perm-actions">
    <button class="btn deny" on:click={handleDeny} disabled={loading}>Deny</button>
    <button class="btn allow" on:click={handleAllow} disabled={loading}>Allow</button>
  </div>
</div>

<style>
  .perm-bar {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-3) var(--sp-5);
    background: rgba(255, 180, 50, 0.08);
    border-bottom: 1px solid rgba(255, 180, 50, 0.2);
    font-size: var(--sm);
  }

  .perm-icon {
    color: var(--s-input);
    font-size: var(--md);
    flex-shrink: 0;
  }

  .perm-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .perm-tool {
    font-weight: 600;
    color: var(--t0);
  }

  .perm-desc {
    color: var(--t1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .perm-actions {
    display: flex;
    gap: var(--sp-3);
    flex-shrink: 0;
  }

  .btn {
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    padding: var(--sp-2) var(--sp-5);
    font-size: var(--xs);
    cursor: pointer;
    font-family: var(--mono);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .deny {
    background: none;
    color: var(--t1);
  }

  .deny:hover:not(:disabled) {
    border-color: var(--s-error);
    color: var(--s-error);
  }

  .allow {
    background: rgba(0, 212, 126, 0.1);
    color: var(--ac);
    border-color: var(--ac);
  }

  .allow:hover:not(:disabled) {
    background: rgba(0, 212, 126, 0.2);
  }
</style>
