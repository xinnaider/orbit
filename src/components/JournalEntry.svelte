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
{/if}

<style>
  .entry { padding: 8px 10px; border-radius: 6px; }
  .entry.user {
    background: rgba(96, 165, 250, 0.04);
    border-left: 3px solid var(--blue);
  }
  .entry.assistant {
    background: rgba(168, 85, 247, 0.02);
  }
  .entry.tool-result {
    padding: 4px 10px;
  }
  .header { display: flex; align-items: center; gap: 6px; margin-bottom: 4px; }
  .badge { padding: 1px 6px; border-radius: 4px; font-size: 9px; font-weight: 600; }
  .user-badge { background: rgba(59, 130, 246, 0.2); color: var(--blue); }
  .assistant-badge { background: var(--purple-dim); color: var(--purple); }
  .time { color: var(--text-dim); font-size: 9px; }
  .text { font-size: 11px; line-height: 1.5; }
  .output {
    font-size: 9px;
    color: var(--text-secondary);
    background: rgba(0, 0, 0, 0.15);
    padding: 4px 8px;
    border-radius: 3px;
    max-height: 80px;
    overflow-y: auto;
    white-space: pre-wrap;
    line-height: 1.4;
  }
</style>
