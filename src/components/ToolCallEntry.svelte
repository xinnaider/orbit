<script lang="ts">
  import type { JournalEntry } from '../lib/types';

  export let entry: JournalEntry;

  $: toolClass = (entry.tool ?? '').toLowerCase();
  $: target = extractTarget(entry);
  $: timeStr = entry.timestamp.slice(11, 16);

  const toolIcons: Record<string, string> = {
    read: '📄', edit: '✏️', write: '📝', bash: '⚡',
    grep: '🔍', glob: '📁', agent: '🤖', skill: '🔧',
  };

  $: icon = toolIcons[toolClass] ?? '⚙️';

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
  <span class="icon">{icon}</span>
  <span class="tool {toolClass}">{entry.tool}</span>
  <span class="target mono">{target}</span>
  <span class="time">{timeStr}</span>
  {#if entry.linesChanged}
    <span class="changes">
      <span class="added">+{entry.linesChanged.added}</span>
      <span class="removed">-{entry.linesChanged.removed}</span>
    </span>
  {/if}
</div>

<style>
  .tool-entry {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    border-radius: 6px;
    font-size: 12px;
    margin-left: 16px;
    border-left: 1px solid var(--border);
    transition: background 0.15s;
  }
  .tool-entry:hover {
    background: var(--bg-hover);
  }
  .icon { font-size: 12px; flex-shrink: 0; }
  .tool { font-weight: 600; flex-shrink: 0; }
  .tool.read, .tool.grep, .tool.glob { color: var(--blue); }
  .tool.edit, .tool.write { color: var(--orange); }
  .tool.bash { color: var(--green); }
  .tool.agent { color: var(--purple); }
  .tool.skill { color: var(--pink); }
  .target {
    color: var(--text-secondary);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }
  .time {
    color: var(--text-dim);
    font-size: 10px;
    flex-shrink: 0;
  }
  .changes { display: flex; gap: 3px; flex-shrink: 0; }
  .added { color: var(--green); font-size: 11px; }
  .removed { color: var(--red); font-size: 11px; }
</style>
