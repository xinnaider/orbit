<script lang="ts">
  import { onMount, tick, createEventDispatcher } from 'svelte';
  import type { JournalEntry } from '../lib/types';
  import Markdown from './Markdown.svelte';
  import ToolCallEntry from './ToolCallEntry.svelte';
  import { backends } from '../lib/stores/providers';

  export let entries: JournalEntry[] = [];
  export let status: string = '';
  export let provider: string = 'claude-code';
  export let cwd: string | null = null;

  $: agentLabel = (() => {
    const direct = $backends.find((b) => b.id === provider);
    if (direct) return direct.name.toLowerCase();
    const parent = $backends.find((b) => b.subProviders?.some((s) => s.id === provider));
    return parent?.name.toLowerCase() ?? provider;
  })();

  const dispatch = createEventDispatcher<{ bottomchange: { atBottom: boolean } }>();

  $: isWorking = ['working', 'running'].includes(status);

  // ── Display item grouping ──────────────────────────────────────────────────
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

  // ── Chunk-based loading ────────────────────────────────────────────────────
  // Render the last PAGE_SIZE items. When the user scrolls to the top,
  // prepend another chunk. Normal browser scroll — no spacers, no height cache.
  const PAGE_SIZE = 50;

  let visibleFrom = 0; // index into display[] from which we render
  let isAtBottom = true;
  let lastScrollTop = 0;
  let scrollerEl: HTMLDivElement;
  let programmaticScroll = false; // flag to ignore onScroll after programmatic changes

  // When display grows, reset visibleFrom to show the tail if at bottom.
  // When display shrinks (session switch via {#key}), always reset.
  let prevTotal = 0;
  $: {
    const total = display.length;
    if (total < prevTotal) {
      // session remount or reset — start at tail
      visibleFrom = Math.max(0, total - PAGE_SIZE);
    } else if (total > prevTotal && isAtBottom) {
      // new entries arrived while at bottom — keep showing tail
      visibleFrom = Math.max(0, total - PAGE_SIZE);
    }
    prevTotal = total;
  }

  $: visibleItems = display.slice(visibleFrom);

  $: hasMore = visibleFrom > 0;

  // ── Scroll handling ────────────────────────────────────────────────────────
  function onScroll() {
    if (!scrollerEl) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollerEl;

    // Skip follow-mode logic for programmatic scrolls (ResizeObserver, scrollToBottom).
    if (programmaticScroll) {
      programmaticScroll = false;
      lastScrollTop = scrollTop;
      return;
    }

    const atBottom = scrollHeight - scrollTop - clientHeight < 80;

    if (atBottom) {
      if (!isAtBottom) {
        isAtBottom = true;
        dispatch('bottomchange', { atBottom: true });
      }
    } else if (scrollTop < lastScrollTop) {
      // User scrolled UP intentionally — stop following.
      if (isAtBottom) {
        isAtBottom = false;
        dispatch('bottomchange', { atBottom: false });
      }
    }

    lastScrollTop = scrollTop;

    // Near the top — load previous chunk
    if (scrollTop < 80 && visibleFrom > 0) {
      loadMore();
    }
  }

  async function loadMore() {
    if (visibleFrom === 0) return;
    // Capture anchor element before prepending items
    const anchor = scrollerEl.firstElementChild as HTMLElement | null;
    const anchorTop = anchor ? anchor.getBoundingClientRect().top : 0;

    const prevFrom = visibleFrom;
    visibleFrom = Math.max(0, visibleFrom - PAGE_SIZE);
    const added = prevFrom - visibleFrom; // how many items were actually prepended

    await tick();

    // Restore scroll so the old first item stays in the same visual position.
    // The "load more" button may occupy children[0], so account for it.
    const offset = visibleFrom > 0 ? 1 : 0; // 1 if "load more" button still present
    const anchorIdx = added + offset;
    const newAnchor = scrollerEl.children[anchorIdx] as HTMLElement | null;
    if (newAnchor) {
      programmaticScroll = true;
      scrollerEl.scrollTop += newAnchor.getBoundingClientRect().top - anchorTop;
      lastScrollTop = scrollerEl.scrollTop;
    }
  }

  // ── Auto-scroll to bottom ──────────────────────────────────────────────────
  export function scrollToBottom() {
    if (!scrollerEl) return;
    visibleFrom = Math.max(0, display.length - PAGE_SIZE);
    isAtBottom = true;
    tick().then(() => {
      if (scrollerEl) {
        programmaticScroll = true;
        scrollerEl.scrollTop = scrollerEl.scrollHeight;
        lastScrollTop = scrollerEl.scrollTop;
      }
    });
  }

  // Auto-scroll when content grows (new entries OR existing entries expanding).
  // A ResizeObserver on the scroller catches every scrollHeight change — covers
  // streaming progress, markdown rendering, code blocks expanding, etc.
  let resizeObs: ResizeObserver | undefined;

  onMount(() => {
    if (scrollerEl) scrollerEl.scrollTop = scrollerEl.scrollHeight;

    resizeObs = new ResizeObserver(() => {
      if (isAtBottom && scrollerEl) {
        programmaticScroll = true;
        scrollerEl.scrollTop = scrollerEl.scrollHeight;
        lastScrollTop = scrollerEl.scrollTop;
      }
    });

    // Observe the scroller's *content* — when any child changes size the
    // observer fires, letting us pin to the bottom without tracking item count.
    for (const child of scrollerEl.children) {
      resizeObs.observe(child);
    }

    // Also observe the scroller itself so we catch container resizes (e.g. window resize).
    resizeObs.observe(scrollerEl);

    return () => resizeObs?.disconnect();
  });

  // When visible items change, observe any newly added children.
  let prevVisibleLen = 0;
  $: if (visibleItems.length !== prevVisibleLen) {
    prevVisibleLen = visibleItems.length;
    if (scrollerEl && resizeObs) {
      for (const child of scrollerEl.children) {
        resizeObs.observe(child); // no-op if already observed
      }
    }
  }
</script>

<div class="feed-scroller" bind:this={scrollerEl} on:scroll={onScroll}>
  {#if hasMore}
    <div class="load-more">
      <button on:click={loadMore}>↑ load earlier messages</button>
    </div>
  {/if}

  {#each visibleItems as { entry: e, result: r, streaming: s }, i}
    {@const absIdx = visibleFrom + i}
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
          <span class="row-who ai-who">{agentLabel}</span>
          <span class="row-ts">{ts(e)}</span>
        </div>
        <div class="row-body">
          <Markdown content={e.text ?? ''} />
        </div>
      </div>
    {:else if e.entryType === 'toolCall'}
      <div class="row tool">
        <ToolCallEntry entry={e} resultEntry={r} streamingEntries={s} {cwd} />
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
  .feed-scroller {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    padding: var(--sp-5) 0;
    box-sizing: border-box;
  }

  .load-more {
    display: flex;
    justify-content: center;
    padding: var(--sp-4) 0 var(--sp-2);
    flex-shrink: 0;
  }

  .load-more button {
    background: none;
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    color: var(--t3);
    font-size: var(--xs);
    padding: var(--sp-2) var(--sp-5);
    cursor: pointer;
  }

  .load-more button:hover {
    border-color: var(--ac);
    color: var(--ac);
  }

  .row {
    padding: var(--sp-6) var(--sp-7);
    flex-shrink: 0;
  }
  .row:hover {
    background: rgba(255, 255, 255, 0.015);
  }

  .row-meta {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    margin-bottom: var(--sp-2);
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
    line-height: var(--lh);
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
    padding: var(--sp-3) var(--sp-5);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
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
    padding: var(--sp-2) var(--sp-7);
  }
  .system-text {
    font-size: var(--xs);
    color: var(--t3);
    font-style: italic;
  }

  .typing-row {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-5) var(--sp-7);
    flex-shrink: 0;
  }
  .typing-dots {
    display: flex;
    gap: var(--sp-2);
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
