<script lang="ts">
  import type { Toast } from '../lib/stores/toasts';
  import { removeToast } from '../lib/stores/toasts';

  export let toast: Toast;

  const ICONS: Record<Toast['type'], string> = {
    error: '⚠',
    warning: '⏳',
    info: 'ℹ',
    success: '✓',
  };
</script>

<div class="toast toast--{toast.type}" role="alert">
  <span class="toast-icon">{ICONS[toast.type]}</span>
  <span class="toast-message">{toast.message}</span>
  <button class="toast-close" on:click={() => removeToast(toast.id)} aria-label="Dismiss">✕</button>
</div>

<style>
  .toast {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 4px;
    border: 1px solid transparent;
    min-width: 260px;
    max-width: 380px;
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

  .toast-icon {
    font-size: 13px;
    flex-shrink: 0;
    margin-top: 1px;
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

  .toast-message {
    flex: 1;
    min-width: 0;
    font-size: var(--sm);
    color: var(--t1);
    line-height: 1.45;
    word-break: break-word;
  }

  .toast-close {
    background: none;
    border: none;
    color: var(--t2);
    font-size: 10px;
    padding: 1px 4px;
    flex-shrink: 0;
    cursor: pointer;
    margin-top: 1px;
    transition: color 0.12s;
  }

  .toast-close:hover {
    color: var(--t0);
  }
</style>
