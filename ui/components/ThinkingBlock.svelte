<script lang="ts">
  export let thinking: string;
  export let duration: number | null = null;
  export let timestamp: string;

  let expanded = false;

  $: preview = thinking.split('\n')[0].slice(0, 120);
  $: timeStr = timestamp.slice(11, 16);
  $: durationStr = duration ? `${duration.toFixed(1)}s` : '';
</script>

<div class="thinking-block">
  <div
    class="header"
    onclick={() => (expanded = !expanded)}
    role="button"
    tabindex="0"
    onkeydown={(e) => e.key === 'Enter' && (expanded = !expanded)}
  >
    <div class="left">
      <span class="badge">THINKING</span>
      <span class="meta"
        >{timeStr}
        {#if durationStr}• {durationStr}{/if}</span
      >
    </div>
    <span class="toggle">{expanded ? '▼' : '▶'} {expanded ? 'expanded' : 'collapsed'}</span>
  </div>
  {#if expanded}
    <div class="content">{thinking}</div>
  {:else}
    <div class="preview">{preview}...</div>
  {/if}
</div>

<style>
  .thinking-block {
    padding: var(--sp-4) var(--sp-5);
    background: var(--bg-thinking);
  }
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--sp-3);
    cursor: pointer;
  }
  .left {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }
  .badge {
    background: var(--purple-dim);
    color: var(--purple);
    padding: 1px var(--sp-3);
    border-radius: var(--radius-md);
    font-size: 11px;
    font-weight: 600;
  }
  .meta {
    color: var(--text-dim);
    font-size: 11px;
  }
  .toggle {
    color: var(--text-dim);
    font-size: 11px;
  }
  .content {
    font-size: 13px;
    color: var(--thinking-text);
    line-height: 1.6;
    padding: var(--sp-3) var(--sp-4);
    background: var(--bg-thinking);
    border-radius: var(--radius-md);
    font-style: italic;
    border: 1px solid var(--border-thinking);
    white-space: pre-wrap;
    max-height: 300px;
    overflow-y: auto;
  }
  .preview {
    font-size: 12px;
    color: var(--thinking-preview);
    padding: var(--sp-1) var(--sp-4);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
</style>
