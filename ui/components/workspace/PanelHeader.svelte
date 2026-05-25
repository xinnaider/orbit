<script lang="ts">
  import { glassChrome } from '../../lib/stores/preferences';

  export let title: string;
  export let meta: string | null = null;
  export let status: string | null = null;
  export let model: string | null = null;
  export let contextPercent: number | null = null;
  export let statusColor: string | null = null;
  export let onClose: (() => void) | null = null;
  export let focused: boolean = true;
</script>

<header class="topbar quiet-topbar" class:glass-topbar={$glassChrome} class:focused>
  <div class="crumb">
    {#if $glassChrome && statusColor}
      <span class="status-dot" style="background: {statusColor}" aria-hidden="true"></span>
    {/if}
    <span class="crumb-title" {title}>{title}</span>
    {#if meta}
      <span class="crumb-meta" title={meta}>{meta}</span>
    {/if}
  </div>
  <div class="pills">
    {#if status}
      <span
        class="pill pill-status"
        style={statusColor
          ? `color:${statusColor};border-color:color-mix(in srgb, ${statusColor}, transparent 72%);background:color-mix(in srgb, ${statusColor}, transparent 90%)`
          : ''}>{status}</span
      >
    {/if}
    {#if model}
      <span class="pill pill-model">{model}</span>
    {/if}
    {#if contextPercent != null && contextPercent > 0}
      <span class="pill pill-ctx">{Math.round(contextPercent)}% ctx</span>
    {/if}
    {#if onClose}
      <button class="close-btn" type="button" aria-label="Close panel" on:click={onClose}>✕</button>
    {/if}
  </div>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 58px;
    padding: 0 24px;
    border-bottom: 1px solid var(--bd);
    background: rgba(255, 255, 255, 0.006);
    flex-shrink: 0;
    user-select: none;
    min-width: 0;
  }

  .topbar.glass-topbar {
    height: 36px;
    padding: 0 10px;
    border-bottom-color: var(--glass-border-subtle);
    background: var(--glass-bg-subtle);
    backdrop-filter: blur(var(--glass-blur));
    -webkit-backdrop-filter: blur(var(--glass-blur));
  }

  .topbar:not(.focused) {
    opacity: 0.65;
  }
  .topbar.focused {
    box-shadow: inset 0 1px 0 color-mix(in srgb, var(--ac), transparent 84%);
  }

  .crumb {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    overflow: hidden;
  }

  .glass-topbar .crumb {
    gap: 8px;
  }

  .status-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    flex-shrink: 0;
    box-shadow: 0 0 8px color-mix(in srgb, currentColor, transparent 40%);
  }

  .crumb-title {
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--t0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .glass-topbar .crumb-title {
    font-weight: 500;
    font-size: 10px;
    letter-spacing: -0.01em;
    color: color-mix(in srgb, var(--t0), transparent 45%);
  }

  .crumb-meta {
    color: var(--t3);
    font-family: var(--mono);
    font-size: 10px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .glass-topbar .crumb-meta {
    font-size: 8.5px;
  }

  .pills {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-shrink: 0;
  }

  .glass-topbar .pills {
    gap: 6px;
  }

  .pill {
    border: 1px solid var(--bd);
    color: var(--t2);
    border-radius: 999px;
    padding: 5px 9px;
    font-family: var(--mono);
    font-size: 10px;
    background: rgba(255, 255, 255, 0.025);
    white-space: nowrap;
  }

  .glass-topbar .pill {
    border-color: color-mix(in srgb, var(--ac), transparent 72%);
    color: color-mix(in srgb, var(--ac), transparent 35%);
    padding: 0 5px;
    font-size: 8.5px;
    background: color-mix(in srgb, var(--ac), transparent 92%);
    line-height: 1.6;
  }

  .pill-status {
    color: var(--ac);
    background: color-mix(in srgb, var(--ac), transparent 90%);
    border-color: color-mix(in srgb, var(--ac), transparent 72%);
  }

  .glass-topbar .pill-model,
  .glass-topbar .pill-ctx {
    border-color: var(--glass-border);
    color: var(--t2);
    background: var(--glass-bg);
  }

  .close-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border: 1px solid var(--bd);
    border-radius: 8px;
    background: transparent;
    color: var(--t3);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.12s;
  }

  .glass-topbar .close-btn {
    width: 20px;
    height: 20px;
    border-color: var(--glass-border);
    border-radius: 6px;
    font-size: 11px;
  }

  .close-btn:hover {
    border-color: var(--t2);
    color: var(--t0);
    background: var(--bg2);
  }

  .glass-topbar .close-btn:hover {
    background: var(--glass-bg);
  }
</style>
