<script lang="ts">
  import type { JournalEntry as JEntry } from '../lib/types';
  import ThinkingBlock from './ThinkingBlock.svelte';
  import ToolCallEntry from './ToolCallEntry.svelte';
  import Markdown from './Markdown.svelte';

  export let entry: JEntry;
  export let resultEntry: JEntry | null = null;

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
  <ToolCallEntry {entry} {resultEntry} />
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
    padding: 10px 14px;
    border-radius: 10px;
    margin: 2px 0;
  }
  .entry.user {
    background: var(--bg-user);
  }
  .entry.assistant {
    background: var(--bg-assistant);
  }
  .entry.user,
  .entry.assistant {
    border: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  }
  .entry.system {
    border: 1px solid var(--border);
    opacity: 0.7;
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
  .user-badge {
    background: var(--user-badge-bg);
    color: var(--blue);
  }
  .assistant-badge {
    background: var(--purple-dim);
    color: var(--purple);
  }
  .system-badge {
    background: var(--bg-system-badge);
    color: var(--text-muted);
  }
  .time {
    color: var(--text-dim);
    font-size: 11px;
  }
  .text {
    font-size: 13px;
    line-height: 1.5;
  }
</style>
