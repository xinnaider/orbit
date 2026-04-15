<script lang="ts">
  import type { Toast } from '../lib/stores/toasts';
  import { removeToast } from '../lib/stores/toasts';

  export let toast: Toast;

  const ICONS: Record<Toast['type'], string> = {
    error: '⚠',
    warning: '⏳',
    info: 'ℹ',
    success: '✓',
    update: '↑',
  };
</script>

<div class="toast toast--{toast.type}" role="alert">
  <span class="toast-icon">{ICONS[toast.type]}</span>
  <div class="toast-body">
    <span class="toast-message">{toast.message}</span>
    {#if toast.action}
      <button
        class="toast-action"
        on:click={() => {
          toast.action?.onClick();
          removeToast(toast.id);
        }}
      >
        {toast.action.label}
      </button>
    {/if}
  </div>
  <button class="toast-close" on:click={() => removeToast(toast.id)} aria-label="Dismiss">✕</button>
</div>

<style>
  .toast {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-4);
    padding: var(--sp-5) var(--sp-6);
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    min-width: 280px;
    max-width: 400px;
    pointer-events: all;
    animation: slideIn 0.18s ease;
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateX(20px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  .toast--error {
    background: rgba(22, 0, 0, 0.97);
    border-color: rgba(224, 72, 72, 0.45);
  }

  .toast--warning {
    background: rgba(30, 20, 0, 0.97);
    border-color: rgba(180, 120, 0, 0.45);
  }

  .toast--info {
    background: var(--bg2);
    border-color: rgba(72, 136, 224, 0.35);
  }

  .toast--success {
    background: var(--bg2);
    border-color: rgba(0, 212, 126, 0.3);
  }

  .toast--update {
    background: rgba(0, 30, 18, 0.98);
    border-color: var(--ac);
    box-shadow: 0 0 12px rgba(0, 212, 126, 0.2);
  }

  .toast-icon {
    font-size: 13px;
    flex-shrink: 0;
    margin-top: var(--sp-1);
  }

  .toast--error .toast-icon {
    color: var(--s-error);
  }
  .toast--warning .toast-icon {
    color: #e0a030;
  }
  .toast--info .toast-icon {
    color: var(--s-init);
  }
  .toast--success .toast-icon {
    color: var(--ac);
  }
  .toast--update .toast-icon {
    color: var(--ac);
    font-weight: 700;
  }

  .toast-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }

  .toast-message {
    font-size: var(--sm);
    color: var(--t1);
    line-height: 1.45;
    word-break: break-word;
  }

  .toast--update .toast-message {
    color: var(--t0);
  }

  .toast-action {
    align-self: flex-start;
    background: var(--ac);
    color: #000;
    border: none;
    border-radius: var(--radius-md);
    padding: var(--sp-2) var(--sp-6);
    font-size: var(--xs);
    font-weight: 700;
    cursor: pointer;
    letter-spacing: 0.04em;
    transition: opacity 0.12s;
  }

  .toast-action:hover {
    opacity: 0.85;
  }

  .toast-close {
    background: none;
    border: none;
    color: var(--t2);
    font-size: 10px;
    padding: 1px var(--sp-2);
    flex-shrink: 0;
    cursor: pointer;
    margin-top: 1px;
    transition: color 0.12s;
  }

  .toast-close:hover {
    color: var(--t0);
  }
</style>
