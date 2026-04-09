<script lang="ts">
  import { onMount, tick, createEventDispatcher } from 'svelte';
  import type { JournalEntry } from '../lib/types';
  import Markdown from './Markdown.svelte';
  import ToolCallEntry from './ToolCallEntry.svelte';

  export let entries: JournalEntry[] = [];
  export let status: string = '';

  const dispatch = createEventDispatcher<{ bottomchange: { atBottom: boolean } }>();

  $: isWorking = ['working', 'running'].includes(status);

  // ── Display item grouping (unchanged) ──────────────────────────────────────
  interface DisplayItem {
    entry: JournalEntry;
    result: JournalEntry | null;
    streaming: JournalEntry[];
  }

  $: display = (() => {
    const items: DisplayItem[] = [];
    const skip = new Set<number>();
    for (let i = 0; i < entries.length; i++) {
      if (skip.has(i)) continue;
      const e = entries[i];
      if (e.entryType === 'toolCall') {
        const streaming: JournalEntry[] = [];
        let result: JournalEntry | null = null;
        for (let j = i + 1; j < entries.length; j++) {
          if (entries[j].entryType === 'toolResult') {
            result = entries[j];
            skip.add(j);
            break;
          } else if (entries[j].entryType === 'progress') {
            streaming.push(entries[j]);
            skip.add(j);
          } else {
            break;
          }
        }
        items.push({ entry: e, result, streaming });
      } else if (e.entryType === 'toolResult' || e.entryType === 'progress') {
        // handled above
      } else {
        items.push({ entry: e, result: null, streaming: [] });
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

  // ── Virtual scrolling ──────────────────────────────────────────────────────
  const ESTIMATED_HEIGHT = 120;
  const OVERSCAN = 5;

  let scrollerEl: HTMLDivElement;
  let scrollTop = 0;
  let clientHeight = 0;

  // Per-item measured heights; -1 means "not yet measured, use estimate"
  let heights: number[] = [];

  function setHeight(index: number, h: number) {
    heights[index] = h;
    heights = [...heights];
  }

  // Keep heights array in sync with display array length
  $: {
    const len = display.length;
    if (heights.length !== len) {
      heights = new Array(len).fill(0).map((_, i) => heights[i] ?? ESTIMATED_HEIGHT);
    }
  }

  // Cumulative top offsets (prefix sums); length === display.length + 1
  $: offsets = (() => {
    const arr = new Array(display.length + 1).fill(0);
    for (let i = 0; i < display.length; i++) {
      arr[i + 1] = arr[i] + (heights[i] || ESTIMATED_HEIGHT);
    }
    return arr;
  })();

  $: totalHeight = offsets[display.length] + (isWorking ? 44 : 0);

  // Visible window
  $: startIndex = (() => {
    let lo = 0;
    let hi = display.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (offsets[mid + 1] <= scrollTop) lo = mid + 1;
      else hi = mid;
    }
    return Math.max(0, lo - OVERSCAN);
  })();

  $: endIndex = (() => {
    const bottom = scrollTop + clientHeight;
    let lo = startIndex;
    let hi = display.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (offsets[mid] < bottom) lo = mid + 1;
      else hi = mid;
    }
    return Math.min(display.length, lo + OVERSCAN);
  })();

  $: topSpacerHeight = offsets[startIndex];
  $: bottomSpacerHeight = totalHeight - offsets[endIndex] - (isWorking ? 44 : 0);

  $: visibleItems = display.slice(startIndex, endIndex);

  // ── Auto-scroll & isAtBottom ───────────────────────────────────────────────
  let isAtBottom = true;

  function onScroll() {
    if (!scrollerEl) return;
    scrollTop = scrollerEl.scrollTop;
    clientHeight = scrollerEl.clientHeight;
    const newAtBottom = scrollerEl.scrollHeight - scrollTop - clientHeight < 50;
    if (newAtBottom !== isAtBottom) {
      isAtBottom = newAtBottom;
      dispatch('bottomchange', { atBottom: isAtBottom });
    }
  }

  export function scrollToBottom() {
    if (!scrollerEl) return;
    scrollerEl.scrollTop = scrollerEl.scrollHeight;
    isAtBottom = true;
  }

  // Scroll to bottom when new entries arrive and user was already at bottom
  let prevDisplayLength = 0;
  $: if (display.length !== prevDisplayLength) {
    const wasAtBottom = isAtBottom;
    prevDisplayLength = display.length;
    if (wasAtBottom) {
      tick().then(() => {
        if (scrollerEl) scrollerEl.scrollTop = scrollerEl.scrollHeight;
      });
    }
  }

  onMount(() => {
    if (scrollerEl) {
      clientHeight = scrollerEl.clientHeight;
      scrollerEl.scrollTop = scrollerEl.scrollHeight;
    }
  });

  // Svelte action: observe an item's height and update the cache
  function trackHeight(node: HTMLElement, index: number) {
    function update() {
      const h = node.offsetHeight;
      if (h > 0 && heights[index] !== h) {
        setHeight(index, h);
      }
    }
    update();
    const ro = new ResizeObserver(update);
    ro.observe(node);
    return {
      update(newIndex: number) {
        index = newIndex;
        update();
      },
      destroy() {
        ro.disconnect();
      },
    };
  }
</script>

<div class="feed-scroller" bind:this={scrollerEl} bind:clientHeight on:scroll={onScroll}>
  <!-- top spacer -->
  <div style="height:{topSpacerHeight}px; flex-shrink:0;" aria-hidden="true"></div>

  {#each visibleItems as { entry: e, result: r, streaming: s }, localIdx}
    {@const absIdx = startIndex + localIdx}
    <div class="vrow" use:trackHeight={absIdx}>
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
        {@const expanded = expandedThinking.has(absIdx)}
        <div class="row thinking">
          <div class="row-meta">
            <span class="row-who think-who">···</span>
            {#if e.thinkingDuration}
              <span class="row-ts">{e.thinkingDuration.toFixed(1)}s</span>
            {/if}
            <button class="expand-btn" on:click={() => toggleThinking(absIdx)}>
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
          <ToolCallEntry entry={e} resultEntry={r} streamingEntries={s} />
        </div>
      {:else if e.entryType === 'system'}
        <div class="row system">
          <span class="system-text">{e.text}</span>
        </div>
      {/if}
    </div>
  {/each}

  <!-- bottom spacer -->
  <div style="height:{bottomSpacerHeight}px; flex-shrink:0;" aria-hidden="true"></div>

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
  .feed-scroller {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    padding: 10px 0;
    box-sizing: border-box;
  }

  .vrow {
    flex-shrink: 0;
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
    flex-shrink: 0;
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
