<script lang="ts">
  import type { MiniLogEntry } from '../lib/types';

  export let entries: MiniLogEntry[];
  export let pendingApproval: string | null = null;
</script>

<div class="mini-log">
  {#each entries as entry}
    <div class="entry">
      <span class="tool {entry.tool.toLowerCase()}">{entry.tool}</span>
      <span class="target">{entry.target}</span>
      {#if entry.success === true}
        <span class="result ok">✓</span>
      {:else if entry.success === false}
        <span class="result fail">✗</span>
      {/if}
    </div>
  {/each}
  {#if pendingApproval}
    <div class="entry pending">⏳ {pendingApproval}</div>
  {/if}
</div>

<style>
  .mini-log {
    margin-top: 6px;
    padding: 4px 6px;
    background: var(--bg-inset);
    border-radius: 4px;
    font-size: 11px;
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    line-height: 1.5;
  }
  .entry { display: flex; gap: 4px; color: var(--text-dim); }
  .tool { min-width: 32px; font-weight: 500; }
  .tool.read { color: var(--blue); }
  .tool.edit, .tool.write { color: var(--orange); }
  .tool.bash { color: var(--green); }
  .tool.agent { color: var(--purple); }
  .tool.grep { color: var(--blue); }
  .target { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .result.ok { color: var(--green); }
  .result.fail { color: var(--red); }
  .pending { color: var(--amber); font-weight: 500; }
</style>
