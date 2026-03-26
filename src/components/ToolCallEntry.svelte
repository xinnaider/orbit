<script lang="ts">
  import type { JournalEntry } from '../lib/types';

  export let entry: JournalEntry;

  $: toolClass = (entry.tool ?? '').toLowerCase();
  $: target = extractTarget(entry);
  $: timeStr = entry.timestamp.slice(11, 16);

  function extractTarget(e: JournalEntry): string {
    if (!e.toolInput) return '';
    if (e.toolInput.file_path) {
      const parts = (e.toolInput.file_path as string).split(/[/\\]/);
      return parts[parts.length - 1] ?? '';
    }
    if (e.toolInput.command) {
      const cmd = e.toolInput.command as string;
      const first = cmd.split('\n')[0];
      return first.length > 60 ? first.slice(0, 60) + '...' : first;
    }
    if (e.toolInput.pattern) return e.toolInput.pattern as string;
    if (e.toolInput.description) return e.toolInput.description as string;
    return '';
  }
</script>

<div class="tool-entry">
  <div class="main-row">
    <span class="time">{timeStr}</span>
    <span class="tool {toolClass}">{entry.tool}</span>
    <span class="target mono">{target}</span>
    {#if entry.linesChanged}
      <span class="added">+{entry.linesChanged.added}</span>
      <span class="removed">-{entry.linesChanged.removed}</span>
    {/if}
  </div>
</div>

<style>
  .tool-entry {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 5px 8px;
    border-radius: 4px;
    font-size: 12px;
  }
  .main-row { display: flex; align-items: center; gap: 6px; }
  .time { color: var(--text-dim); min-width: 38px; font-size: 12px; }
  .tool { min-width: 32px; font-weight: 500; }
  .tool.read, .tool.grep { color: var(--blue); }
  .tool.edit, .tool.write { color: var(--orange); }
  .tool.bash { color: var(--green); }
  .tool.agent { color: var(--purple); }
  .target { color: var(--text-secondary); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .added { color: var(--green); font-size: 11px; }
  .removed { color: var(--red); font-size: 11px; }
</style>
