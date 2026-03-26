<script lang="ts">
  import type { JournalEntry as JEntry } from '../lib/types';
  import ThinkingBlock from './ThinkingBlock.svelte';
  import ToolCallEntry from './ToolCallEntry.svelte';

  export let entry: JEntry;

  $: timeStr = entry.timestamp.slice(11, 16);
</script>

{#if entry.entryType === 'user'}
  <div class="entry user">
    <div class="header">
      <span class="badge user-badge">USER</span>
      <span class="time">{timeStr}</span>
    </div>
    <div class="text">{entry.text}</div>
  </div>
{:else if entry.entryType === 'thinking'}
  <ThinkingBlock
    thinking={entry.thinking ?? ''}
    duration={entry.thinkingDuration}
    timestamp={entry.timestamp}
  />
{:else if entry.entryType === 'assistant'}
  <div class="entry assistant">
    <div class="header">
      <span class="badge assistant-badge">ASSISTANT</span>
      <span class="time">{timeStr}</span>
    </div>
    <div class="text">{entry.text}</div>
  </div>
{:else if entry.entryType === 'toolCall'}
  <ToolCallEntry {entry} />
{:else if entry.entryType === 'toolResult'}
  <div class="entry tool-result">
    <div class="output mono">{entry.output?.slice(0, 500)}{(entry.output?.length ?? 0) > 500 ? '...' : ''}</div>
  </div>
{:else if entry.entryType === 'system'}
  <div class="entry system">
    <div class="header">
      <span class="badge system-badge">SYSTEM</span>
      <span class="time">{timeStr}</span>
    </div>
    <div class="text">{entry.text}</div>
  </div>
{/if}

<style>
  .entry { padding: 8px 10px; border-radius: 6px; }
  .entry.user {
    background: var(--bg-user);
    border-left: 3px solid var(--blue);
  }
  .entry.assistant {
    background: var(--bg-assistant);
  }
  .entry.tool-result {
    padding: 4px 10px;
  }
  .header { display: flex; align-items: center; gap: 6px; margin-bottom: 4px; }
  .badge { padding: 1px 6px; border-radius: 4px; font-size: 11px; font-weight: 600; }
  .user-badge { background: var(--user-badge-bg); color: var(--blue); }
  .assistant-badge { background: var(--purple-dim); color: var(--purple); }
  .system-badge { background: var(--bg-system-badge); color: var(--text-muted); }
  .entry.system { opacity: 0.7; }
  .time { color: var(--text-dim); font-size: 11px; }
  .text { font-size: 13px; line-height: 1.5; }
  .output {
    font-size: 11px;
    color: var(--text-secondary);
    background: var(--bg-code);
    padding: 4px 8px;
    border-radius: 3px;
    max-height: 80px;
    overflow-y: auto;
    white-space: pre-wrap;
    line-height: 1.4;
  }
</style>
