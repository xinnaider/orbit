<script lang="ts">
  import { toasts } from '../lib/stores/toasts';
  import Toast from './Toast.svelte';
</script>

{#if $toasts.length > 0}
  <div class="toast-container" aria-live="polite" aria-atomic="false">
    {#each $toasts as toast (toast.id)}
      <Toast {toast} />
    {/each}
  </div>
{/if}

<style>
  .toast-container {
    position: fixed;
    top: 16px;
    right: 16px;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    pointer-events: none;
    align-items: flex-end;
  }
</style>
