<script lang="ts">
  export let title: string;
  /** Full path for tooltip when `path` is shortened. */
  export let pathFull: string | null = null;
  export let branch: string | null = null;
  export let path: string | null = null;
  /** Fallback subtitle when branch/path are not used (e.g. Git file count). */
  export let meta: string | null = null;
  export let status: string | null = null;
  export let model: string | null = null;
  export let contextPercent: number | null = null;
  export let statusColor: string | null = null;
  export let onClose: (() => void) | null = null;
  export let focused: boolean = true;

  $: titleTooltip = [title, pathFull ?? path, branch].filter(Boolean).join(' · ');
  $: showTicks = Boolean(
    status || model || (contextPercent != null && contextPercent > 0) || onClose
  );
  $: showCrumbExtra = Boolean(branch || path || meta);
</script>

<header class="topbar quiet-topbar" class:focused>
  <div class="crumb">
    <span class="crumb-title" title={titleTooltip}>{title}</span>
    {#if showCrumbExtra}
      <span class="crumb-extra">
        {#if branch}
          <span class="branch-badge" title="Git branch">{branch}</span>
        {/if}
        {#if path}
          <span class="path-short" title={pathFull ?? path}>{path}</span>
        {:else if meta}
          <span class="path-short meta-fallback" title={meta}>{meta}</span>
        {/if}
      </span>
    {/if}
  </div>

  {#if showTicks}
    <div class="top-end">
      {#if status || model || (contextPercent != null && contextPercent > 0)}
        <span class="ticks" aria-label="Session stats">
          {#if status}
            <span class="tick-status" style={statusColor ? `color:${statusColor}` : undefined}
              >{status}</span
            >
          {/if}
          {#if status && (model || (contextPercent != null && contextPercent > 0))}
            <span class="tick-sep" aria-hidden="true">·</span>
          {/if}
          {#if model}
            <span class="tick-model">{model}</span>
          {/if}
          {#if model && contextPercent != null && contextPercent > 0}
            <span class="tick-sep" aria-hidden="true">·</span>
          {/if}
          {#if contextPercent != null && contextPercent > 0}
            <span class="tick-ctx">{Math.round(contextPercent)}% ctx</span>
          {/if}
        </span>
      {/if}
      {#if onClose}
        <button class="close-btn" type="button" aria-label="Close panel" on:click={onClose}
          >✕</button
        >
      {/if}
    </div>
  {/if}
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    height: 38px;
    padding: 0 14px;
    border-top: 1px solid var(--bd);
    border-bottom: 1px solid var(--bd);
    background: color-mix(in srgb, var(--bg), white 0.4%);
    flex-shrink: 0;
    user-select: none;
    min-width: 0;
  }

  .topbar:not(.focused) {
    opacity: 0.65;
  }

  .crumb {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    flex: 1;
    overflow: hidden;
  }

  .crumb-title {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--t0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 0;
    max-width: 45%;
  }

  .crumb-extra {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    overflow: hidden;
  }

  .branch-badge {
    flex-shrink: 0;
    font-family: var(--mono);
    font-size: 9px;
    font-weight: 500;
    color: var(--ac);
    padding: 2px 7px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--ac), transparent 72%);
    background: color-mix(in srgb, var(--ac), transparent 90%);
    letter-spacing: 0.03em;
    line-height: 1.3;
  }

  .path-short {
    font-family: var(--mono);
    font-size: 9px;
    color: var(--t3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .meta-fallback {
    color: var(--t2);
  }

  .top-end {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .ticks {
    font-family: var(--mono);
    font-size: 9px;
    color: var(--t2);
    white-space: nowrap;
    letter-spacing: 0.02em;
  }

  .tick-status {
    color: var(--ac);
    font-weight: 500;
  }

  .tick-sep {
    color: var(--t3);
    margin: 0 5px;
  }

  .tick-model,
  .tick-ctx {
    color: var(--t2);
  }

  .close-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--t3);
    font-size: 12px;
    cursor: pointer;
    transition:
      color 0.12s,
      background 0.12s;
  }

  .close-btn:hover {
    color: var(--t0);
    background: var(--bg2);
  }
</style>
