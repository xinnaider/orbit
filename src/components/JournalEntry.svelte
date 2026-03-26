<script lang="ts">
  import type { JournalEntry as JEntry } from '../lib/types';
  import ThinkingBlock from './ThinkingBlock.svelte';
  import ToolCallEntry from './ToolCallEntry.svelte';
  import Markdown from './Markdown.svelte';

  export let entry: JEntry;

  $: timeStr = entry.timestamp.slice(11, 16);
</script>

{#if entry.entryType === 'user'}
  <div class="entry user">
    <div class="entry-header">
      <span class="badge user-badge">USER</span>
      <span class="time">{timeStr}</span>
    </div>
    <div class="text"><Markdown content={entry.text ?? ''} /></div>
  </div>
{:else if entry.entryType === 'thinking'}
  <ThinkingBlock
    thinking={entry.thinking ?? ''}
    duration={entry.thinkingDuration}
    timestamp={entry.timestamp}
  />
{:else if entry.entryType === 'assistant'}
  <div class="entry assistant">
    <div class="entry-header">
      <span class="badge assistant-badge">ASSISTANT</span>
      <span class="time">{timeStr}</span>
    </div>
    <div class="text"><Markdown content={entry.text ?? ''} /></div>
  </div>
{:else if entry.entryType === 'toolCall'}
  <ToolCallEntry {entry} />
{:else if entry.entryType === 'toolResult'}
  <div class="entry tool-result">
    <div class="output mono">{entry.output?.slice(0, 500)}{(entry.output?.length ?? 0) > 500 ? '...' : ''}</div>
  </div>
{:else if entry.entryType === 'system'}
  <div class="entry system">
    <div class="entry-header">
      <span class="badge system-badge">SYSTEM</span>
      <span class="time">{timeStr}</span>
    </div>
    <div class="text"><Markdown content={entry.text ?? ''} /></div>
  </div>
{/if}

<style>
  .entry {
    padding: 10px 12px;
    border-radius: 8px;
    margin: 1px 0;
  }
  .entry.user {
    background: var(--bg-user);
    border-left: 3px solid var(--blue);
    padding-left: 12px;
  }
  .entry.assistant {
    background: var(--bg-assistant);
    border-left: 3px solid var(--purple);
    padding-left: 12px;
  }
  .entry.tool-result {
    padding: 4px 12px;
    margin-left: 16px;
    border-left: 1px solid var(--border);
  }
  .entry.system {
    opacity: 0.6;
    font-style: italic;
  }
  .entry-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }
  .badge {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.5px;
  }
  .user-badge { background: var(--user-badge-bg); color: var(--blue); }
  .assistant-badge { background: var(--purple-dim); color: var(--purple); }
  .system-badge { background: var(--bg-system-badge); color: var(--text-muted); }
  .time { color: var(--text-dim); font-size: 11px; }
  .text {
    font-size: 13px;
    line-height: 1.5;
  }
  .output {
    font-size: 11px;
    color: var(--text-secondary);
    background: var(--bg-code);
    padding: 6px 10px;
    border-radius: 4px;
    max-height: 100px;
    overflow-y: auto;
    white-space: pre-wrap;
    line-height: 1.5;
  }
</style>
