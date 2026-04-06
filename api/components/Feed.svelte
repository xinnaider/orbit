<script lang="ts">
  import type { JournalEntry } from '../lib/types';
  import Markdown from './Markdown.svelte';
  import ToolCallEntry from './ToolCallEntry.svelte';

  export let entries: JournalEntry[] = [];
  export let status: string = '';

  $: isWorking = ['working', 'running'].includes(status);

  // Pair toolCall with its toolResult
  interface DisplayItem {
    entry: JournalEntry;
    result: JournalEntry | null;
  }

  $: display = (() => {
    const items: DisplayItem[] = [];
    const skip = new Set<number>();
    for (let i = 0; i < entries.length; i++) {
      if (skip.has(i)) continue;
      const e = entries[i];
      if (e.entryType === 'toolCall') {
        const next = entries[i + 1];
        if (next?.entryType === 'toolResult') {
          items.push({ entry: e, result: next });
          skip.add(i + 1);
        } else {
          items.push({ entry: e, result: null });
        }
      } else if (e.entryType === 'toolResult') {
        // orphan — skip, already handled
      } else {
        items.push({ entry: e, result: null });
      }
    }
    return items;
  })();

  function ts(entry: JournalEntry) {
    return entry.timestamp?.slice(11, 16) ?? '';
  }

  let expandedThinking = new Set<number>();
  function toggleThinking(i: number) {
    const next = new Set(expandedThinking);
    if (next.has(i)) next.delete(i);
    else next.add(i);
    expandedThinking = next;
  }
</script>

<div class="feed">
  {#each display as { entry: e, result: r }, i}
    {#if e.entryType === 'user'}
      <div class="row user">
        <div class="row-meta">
          <span class="row-who user-who">you</span>
          <span class="row-ts">{ts(e)}</span>
        </div>
        <div class="row-body">
          <Markdown content={e.text ?? ''} />
        </div>
      </div>
    {:else if e.entryType === 'thinking'}
      {@const expanded = expandedThinking.has(i)}
      <div class="row thinking">
        <div class="row-meta">
          <span class="row-who think-who">···</span>
          {#if e.thinkingDuration}
            <span class="row-ts">{e.thinkingDuration.toFixed(1)}s</span>
          {/if}
          <button class="expand-btn" on:click={() => toggleThinking(i)}>
            {expanded ? '▼ collapse' : '▶ expand'}
          </button>
        </div>
        {#if expanded}
          <div class="row-body think-body">{e.thinking}</div>
        {:else}
          <div class="row-body think-preview">
            {(e.thinking ?? '').split('\n')[0].slice(0, 100)}…
          </div>
        {/if}
      </div>
    {:else if e.entryType === 'assistant'}
      <div class="row assistant">
        <div class="row-meta">
          <span class="row-who ai-who">claude</span>
          <span class="row-ts">{ts(e)}</span>
        </div>
        <div class="row-body">
          <Markdown content={e.text ?? ''} />
        </div>
      </div>
    {:else if e.entryType === 'toolCall'}
      <div class="row tool">
        <ToolCallEntry entry={e} resultEntry={r} />
      </div>
    {:else if e.entryType === 'system'}
      <div class="row system">
        <span class="system-text">{e.text}</span>
      </div>
    {/if}
  {/each}

  {#if isWorking}
    <div class="typing-row">
      <span class="typing-dots">
        <span></span><span></span><span></span>
      </span>
      <span class="typing-label">working</span>
    </div>
  {/if}
</div>

<style>
  .feed {
    padding: 10px 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .row {
    padding: 8px 14px;
  }
  .row:hover {
    background: rgba(255, 255, 255, 0.015);
  }

  .row-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }
  .row-who {
    font-size: var(--xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: lowercase;
  }
  .user-who {
    color: var(--user-fg);
  }
  .ai-who {
    color: var(--ac);
  }
  .think-who {
    color: var(--think-fg);
  }
  .row-ts {
    font-size: var(--xs);
    color: var(--t3);
  }

  .expand-btn {
    background: none;
    border: none;
    color: var(--t2);
    font-size: var(--xs);
    cursor: pointer;
    padding: 0;
  }
  .expand-btn:hover {
    color: var(--t0);
  }

  .row-body {
    font-size: var(--base);
    line-height: 1.6;
    color: var(--t0);
    padding-left: 0;
  }

  .user .row-body {
    color: var(--t0);
  }

  .think-body {
    color: var(--think-fg);
    white-space: pre-wrap;
    font-style: italic;
    font-size: var(--sm);
    background: var(--think-bg);
    border-left: 2px solid var(--think-fg);
    padding: 6px 10px;
    border-radius: 0 3px 3px 0;
    max-height: 280px;
    overflow-y: auto;
  }
  .think-preview {
    color: var(--think-fg);
    font-style: italic;
    font-size: var(--sm);
    opacity: 0.7;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .system {
    padding: 4px 14px;
  }
  .system-text {
    font-size: var(--xs);
    color: var(--t3);
    font-style: italic;
  }

  .typing-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
  }
  .typing-dots {
    display: flex;
    gap: 3px;
    align-items: center;
  }
  .typing-dots span {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--ac);
    display: block;
    animation: td 1.2s ease-in-out infinite;
    opacity: 0.4;
  }
  .typing-dots span:nth-child(2) {
    animation-delay: 0.2s;
  }
  .typing-dots span:nth-child(3) {
    animation-delay: 0.4s;
  }
  @keyframes td {
    0%,
    100% {
      opacity: 0.4;
      transform: none;
    }
    40% {
      opacity: 1;
      transform: translateY(-3px);
    }
  }
  .typing-label {
    font-size: var(--xs);
    color: var(--t2);
    letter-spacing: 0.06em;
  }
</style>
