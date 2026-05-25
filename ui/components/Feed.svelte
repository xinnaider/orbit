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
  export let compact = false;

  $: agentLabel = (() => {
    const direct = $backends.find((b) => b.id === provider);
    if (direct) return direct.name.toLowerCase();
    const parent = $backends.find((b) => b.subProviders?.some((s) => s.id === provider));
    return parent?.name.toLowerCase() ?? provider;
  })();

  const dispatch = createEventDispatcher<{ bottomchange: { atBottom: boolean } }>();

  $: isWorking = status === 'working' || status === 'input';

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

  function eventClass(entry: JournalEntry): string {
    if (entry.entryType === 'toolCall') return 'tool';
    if (entry.entryType === 'toolResult' || entry.entryType === 'progress') return 'tool-detail';
    if (entry.entryType === 'assistant') return 'assistant';
    if (entry.entryType === 'user') return 'user';
    return 'system';
  }

  function actorLabel(entry: JournalEntry): string {
    if (entry.entryType === 'user') return 'you';
    if (entry.entryType === 'assistant') return `orbit / ${agentLabel}`;
    if (entry.entryType === 'toolCall') return 'tool';
    return entry.entryType;
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
  const PAGE_SIZE = 200;

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

<div class="feed-scroller" class:compact bind:this={scrollerEl} onscroll={onScroll}>
  <div class="timeline">
    {#if hasMore}
      <button type="button" class="load-more" onclick={loadMore}>load earlier</button>
    {/if}

    {#each visibleItems as item, i (visibleFrom + i)}
      {@const entry = item.entry}
      {@const absIdx = visibleFrom + i}
      <article
        class="timeline-event {eventClass(entry)}"
        aria-label="{actorLabel(entry)} {ts(entry)}"
      >
        <div class="timeline-node {eventClass(entry)}" aria-hidden="true"></div>
        <div class="timeline-body">
          <div class="event-meta">
            <span class="event-actor {eventClass(entry)}">{actorLabel(entry)}</span>
            {#if ts(entry)}<span>{ts(entry)}</span>{/if}
          </div>

          {#if entry.entryType === 'toolCall'}
            <ToolCallEntry
              {entry}
              resultEntry={item.result}
              streamingEntries={item.streaming}
              {cwd}
              {compact}
            />
          {:else if entry.entryType === 'thinking'}
            {@const expanded = expandedThinking.has(absIdx)}
            <div class="thinking-inline" class:expanded>
              <span
                class="think-dots-label"
                onclick={() => toggleThinking(absIdx)}
                role="button"
                tabindex="0"
                onkeydown={(e) => e.key === 'Enter' && toggleThinking(absIdx)}
              >
                <span class="think-dots">
                  <span></span><span></span><span></span>
                </span>
                <span class="think-preview-text">
                  {#if expanded}
                    {(entry.thinking ?? '').split('\n')[0].slice(0, 60)}
                  {:else}
                    {(entry.thinking ?? '').split('\n')[0].slice(0, 80)}…
                  {/if}
                </span>
              </span>
              {#if entry.thinkingDuration}
                <span class="event-ts">{entry.thinkingDuration.toFixed(1)}s</span>
              {/if}
              <button class="expand-btn" onclick={() => toggleThinking(absIdx)}>
                {expanded ? '▼' : '▶'}
              </button>
            </div>
            {#if expanded}
              <div class="event-text think-body">{entry.thinking}</div>
            {/if}
          {:else if entry.entryType === 'assistant'}
            <div class="event-text assistant-text">
              <Markdown content={entry.text ?? ''} />
            </div>
          {:else if entry.entryType === 'user'}
            <div class="event-text user-text">{entry.text}</div>
          {:else}
            <div
              class="event-text system-text"
              class:system-error={entry.feedError}
            >
              {entry.text}
            </div>
          {/if}
        </div>
      </article>
    {/each}

    {#if isWorking}
      <article class="timeline-event working" aria-label="agent working">
        <div class="timeline-node working"></div>
        <div class="timeline-body">
          <div class="event-meta">
            <span class="event-actor working">orbit / {agentLabel}</span>
          </div>
          <div class="working-pill" role="status" aria-live="polite">
            <span class="working-word">working</span>
            <span class="typing-dots" aria-hidden="true">
              <span class="dot"></span>
              <span class="dot"></span>
              <span class="dot"></span>
            </span>
          </div>
        </div>
      </article>
    {/if}
  </div>
</div>

<style>
  .feed-scroller {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .load-more {
    display: block;
    margin: 0 auto 4px;
    background: none;
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    color: var(--t3);
    font-size: var(--xs);
    padding: var(--sp-2) var(--sp-5);
    cursor: pointer;
  }
  .load-more:hover {
    border-color: var(--ac);
    color: var(--ac);
  }

  .timeline {
    width: 100%;
    padding: 28px 38px 22px;
    display: flex;
    flex-direction: column;
    gap: 13px;
  }

  .timeline-event {
    display: grid;
    grid-template-columns: 20px 1fr;
    gap: 16px;
    position: relative;
  }
  .timeline-event:not(:last-child)::before {
    content: '';
    position: absolute;
    left: 9px;
    top: 29px;
    bottom: -14px;
    width: 1px;
    background: rgba(255, 255, 255, 0.07);
  }

  .timeline-node {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    margin-top: 4px;
    border: 1px solid var(--bd);
    background: var(--bg);
  }
  .timeline-node::after {
    content: '';
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--t3);
  }
  .timeline-node.user::after {
    background: var(--user-fg);
    box-shadow:
      0 0 0 3px color-mix(in srgb, var(--user-fg), transparent 78%),
      0 0 16px color-mix(in srgb, var(--user-fg), transparent 32%);
  }
  .timeline-node.assistant::after,
  .timeline-node.working::after {
    background: var(--ac);
    box-shadow:
      0 0 0 3px color-mix(in srgb, var(--ac), transparent 78%),
      0 0 16px color-mix(in srgb, var(--ac), transparent 32%);
  }
  .timeline-node.user {
    border-color: color-mix(in srgb, var(--user-fg), transparent 36%);
    background: color-mix(in srgb, var(--user-fg), transparent 86%);
  }
  .timeline-node.assistant,
  .timeline-node.working {
    border-color: color-mix(in srgb, var(--ac), transparent 36%);
    background: color-mix(in srgb, var(--ac), transparent 86%);
  }
  .timeline-node.tool::after {
    background: var(--tool-fg);
    box-shadow: 0 0 12px color-mix(in srgb, var(--tool-fg), transparent 70%);
  }

  .event-meta {
    display: flex;
    align-items: center;
    gap: 9px;
    margin-bottom: 5px;
    color: var(--t3);
    font-family: var(--mono);
    font-size: 10px;
  }
  .event-actor {
    color: var(--t1);
  }
  .event-actor.user {
    color: var(--user-fg);
  }
  .event-actor.assistant,
  .event-actor.working {
    color: var(--ac);
  }
  .event-ts {
    font-size: var(--xs);
    color: var(--t3);
  }

  .event-text {
    color: var(--t0);
    font-size: 13px;
    line-height: 1.58;
  }
  .user-text {
    max-width: 680px;
    width: fit-content;
    border: 1px solid var(--bd);
    border-radius: 18px;
    padding: 12px 14px;
    background: color-mix(in srgb, var(--t0), transparent 95%);
  }
  .system-text {
    color: var(--t1);
    font-family: var(--mono);
    font-size: 12px;
  }
  .system-text.system-error {
    color: var(--s-error);
    border-left: 2px solid var(--s-error);
    padding-left: 10px;
    background: color-mix(in srgb, var(--s-error), transparent 92%);
    border-radius: 4px;
    padding-top: 6px;
    padding-bottom: 6px;
  }

  .working-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--think-fg);
    background: var(--think-bg);
    border: 1px solid color-mix(in srgb, var(--think-fg), transparent 82%);
    border-radius: 999px;
    padding: 7px 10px;
    font-family: var(--mono);
    font-size: 10px;
    line-height: 1;
  }

  .working-word {
    letter-spacing: 0.02em;
  }

  .typing-dots {
    display: inline-flex;
    align-items: flex-end;
    gap: 3px;
    height: 10px;
    padding-bottom: 1px;
  }

  .typing-dots .dot {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: currentColor;
    opacity: 0.35;
    animation: typingDot 1.2s ease-in-out infinite;
  }

  .typing-dots .dot:nth-child(2) {
    animation-delay: 0.15s;
  }

  .typing-dots .dot:nth-child(3) {
    animation-delay: 0.3s;
  }

  @keyframes typingDot {
    0%,
    70%,
    100% {
      opacity: 0.35;
      transform: translateY(0) scale(1);
    }
    35% {
      opacity: 1;
      transform: translateY(-3px) scale(1.1);
    }
  }

  .timeline-node.working::after {
    animation: workingGlow 2.4s ease-in-out infinite;
  }

  @keyframes workingGlow {
    0%,
    100% {
      box-shadow:
        0 0 0 3px color-mix(in srgb, var(--ac), transparent 78%),
        0 0 12px color-mix(in srgb, var(--ac), transparent 60%);
    }
    50% {
      box-shadow:
        0 0 0 3px color-mix(in srgb, var(--ac), transparent 70%),
        0 0 20px color-mix(in srgb, var(--ac), transparent 30%);
    }
  }

  /* ── Thinking inline ── */
  .thinking-inline {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .think-dots-label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    color: var(--tool-fg);
    flex: 1;
    min-width: 0;
  }
  .think-dots {
    display: inline-flex;
    gap: 3px;
    align-items: center;
  }
  .think-dots span {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--tool-fg);
    opacity: 0.4;
    animation: thinkDot 1.4s ease-in-out infinite;
  }
  .think-dots span:nth-child(2) {
    animation-delay: 0.2s;
  }
  .think-dots span:nth-child(3) {
    animation-delay: 0.4s;
  }
  @keyframes thinkDot {
    0%,
    80%,
    100% {
      opacity: 0.4;
      transform: translateY(0);
    }
    40% {
      opacity: 1;
      transform: translateY(-3px);
    }
  }
  .think-preview-text {
    font-size: var(--sm);
    color: var(--tool-fg);
    opacity: 0.6;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--mono);
  }
  .thinking-inline.expanded .think-preview-text {
    opacity: 1;
    font-family: var(--mono);
    color: var(--think-fg);
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
    margin-top: 5px;
  }

  .expand-btn {
    background: none;
    border: none;
    color: var(--t2);
    font-size: 10px;
    cursor: pointer;
    font-family: var(--mono);
    padding: 0;
    line-height: 1;
    letter-spacing: 0.05em;
  }
  .expand-btn:hover {
    color: var(--t1);
  }

  /* ── Compact density ── */
  .feed-scroller.compact .timeline {
    width: 100%;
    padding: 18px 22px;
    gap: 9px;
  }
  .feed-scroller.compact .timeline-event {
    grid-template-columns: 16px 1fr;
    gap: 11px;
  }
  .feed-scroller.compact .timeline-node {
    width: 16px;
    height: 16px;
  }
  .feed-scroller.compact .timeline-node::after {
    width: 6px;
    height: 6px;
  }
  .feed-scroller.compact .timeline-event:not(:last-child)::before {
    left: 7px;
    top: 24px;
    bottom: -10px;
  }
  .feed-scroller.compact .event-text {
    font-size: 12px;
    line-height: 1.52;
  }
  .feed-scroller.compact .user-text {
    border: 0;
    background: transparent;
    padding: 0;
    max-width: none;
  }

  @media (max-width: 768px) {
    .timeline {
      padding: 20px 18px 16px;
      gap: 10px;
    }
    .think-body {
      max-height: 180px;
    }
  }
</style>
