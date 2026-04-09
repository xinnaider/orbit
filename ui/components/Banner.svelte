<script lang="ts">
  export let icon: string = '';
  export let title: string = '';
  export let message: string = '';
  export let variant: 'warning' | 'error' | 'success' = 'warning';
  export let position: 'top' | 'bottom' = 'top';
  export let zIndex: number = 500;
  export let onDismiss: (() => void) | null = null;

  const VARIANT_STYLES: Record<typeof variant, { bg: string; border: string; titleColor: string }> =
    {
      warning: { bg: 'rgba(30,20,0,0.97)', border: 'rgba(180,120,0,0.55)', titleColor: '#e0a030' },
      error: {
        bg: 'rgba(22,0,0,0.97)',
        border: 'rgba(224,72,72,0.55)',
        titleColor: 'var(--s-error)',
      },
      success: { bg: 'var(--bg1)', border: 'rgba(0,212,126,0.3)', titleColor: 'var(--ac)' },
    };

  $: vs = VARIANT_STYLES[variant];
</script>

<div
  class="banner"
  class:pos-top={position === 'top'}
  class:pos-bottom={position === 'bottom'}
  style="background:{vs.bg}; z-index:{zIndex}; --banner-border:{vs.border}; --banner-title-color:{vs.titleColor};"
>
  {#if icon}<span class="banner-icon">{icon}</span>{/if}
  <div class="banner-body">
    <slot>
      {#if title}<div class="banner-title">{title}</div>{/if}
      {#if message}<div class="banner-msg">{message}</div>{/if}
    </slot>
  </div>
  <slot name="actions" />
  {#if onDismiss}
    <button class="banner-close" on:click={onDismiss}>✕</button>
  {/if}
</div>

<style>
  .banner {
    position: fixed;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
  }
  .banner.pos-top {
    top: 0;
    border-bottom: 1px solid var(--banner-border);
    animation: slideDown 0.2s ease;
  }
  .banner.pos-bottom {
    bottom: 0;
    border-top: 1px solid var(--banner-border);
    animation: slideUp 0.2s ease;
  }
  @keyframes slideDown {
    from {
      transform: translateY(-100%);
    }
    to {
      transform: translateY(0);
    }
  }
  @keyframes slideUp {
    from {
      transform: translateY(100%);
    }
    to {
      transform: translateY(0);
    }
  }
  .banner-icon {
    font-size: 14px;
    flex-shrink: 0;
  }
  .banner-body {
    flex: 1;
    min-width: 0;
  }
  .banner-title {
    font-size: var(--sm);
    color: var(--banner-title-color);
    font-weight: 500;
    margin-bottom: 2px;
  }
  .banner-msg {
    font-size: var(--xs);
    color: var(--t1);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .banner-close {
    background: none;
    border: none;
    color: var(--t2);
    font-size: 11px;
    padding: 2px 5px;
    flex-shrink: 0;
    cursor: pointer;
  }
  .banner-close:hover {
    color: var(--t0);
  }
</style>
