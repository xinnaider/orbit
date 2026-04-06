<script lang="ts">
  import type { Session } from '../lib/stores/sessions';
  import { journal, pendingMessages } from '../lib/stores/journal';
  import { detailLevel } from '../lib/stores/preferences';
  import { getSessionJournal } from '../lib/tauri';
  import JournalEntry from './JournalEntry.svelte';
  import CommandInput from './CommandInput.svelte';
  import TypingIndicator from './TypingIndicator.svelte';

  export let session: Session;

  let logContainer: HTMLDivElement;
  let userScrolledUp = false;
  let showScrollBtn = false;

  function handleScroll() {
    if (!logContainer) return;
    const { scrollTop, scrollHeight, clientHeight } = logContainer;
    const atBottom = scrollHeight - scrollTop - clientHeight < 60;
    userScrolledUp = !atBottom;
    showScrollBtn = !atBottom;
  }

  function scrollToBottom() {
    if (logContainer) {
      logContainer.scrollTop = logContainer.scrollHeight;
      userScrolledUp = false;
      showScrollBtn = false;
    }
  }

  let prevEntryCount = 0;

  // Auto-scroll when a new pending message is added
  let prevPendingCount = 0;
  $: {
    const count = $pendingMessages.length;
    if (count > prevPendingCount) {
      requestAnimationFrame(() => {
        if (logContainer) {
          logContainer.scrollTop = logContainer.scrollHeight;
          userScrolledUp = false;
          showScrollBtn = false;
        }
      });
    }
    prevPendingCount = count;
  }

  // Load historical journal from backend when session changes
  async function loadJournal(sessionId: number) {
    const entries = await getSessionJournal(sessionId);
    if (entries.length > prevEntryCount && prevEntryCount > 0) {
      pendingMessages.clear();
    }
    prevEntryCount = entries.length;
    journal.update(map => new Map(map).set(sessionId, entries));
    if (!userScrolledUp) {
      requestAnimationFrame(() => {
        if (logContainer) logContainer.scrollTop = logContainer.scrollHeight;
      });
    }
  }

  $: if (session) {
    loadJournal(session.id);
  }

  $: sessionEntries = $journal.get(session?.id) ?? [];

  $: filteredEntries = $detailLevel === 'compact'
    ? sessionEntries.filter(e => e.entryType === 'toolCall' || e.entryType === 'toolResult')
    : sessionEntries;

  // Build display list: pair toolCall with its following toolResult, skip standalone toolResults
  $: displayEntries = (() => {
    const result: { entry: typeof filteredEntries[0]; resultEntry: typeof filteredEntries[0] | null; skip: boolean }[] = [];
    const skipSet = new Set<number>();

    for (let i = 0; i < filteredEntries.length; i++) {
      if (skipSet.has(i)) continue;
      const entry = filteredEntries[i];

      if (entry.entryType === 'toolCall') {
        const next = filteredEntries[i + 1];
        if (next && next.entryType === 'toolResult') {
          result.push({ entry, resultEntry: next, skip: false });
          skipSet.add(i + 1);
        } else {
          result.push({ entry, resultEntry: null, skip: false });
        }
      } else if (entry.entryType === 'toolResult') {
        // Orphan toolResult (no preceding toolCall) — skip it
        continue;
      } else {
        result.push({ entry, resultEntry: null, skip: false });
      }
    }
    return result;
  })();

  // Determine typing label based on last entry
  $: typingLabel = (() => {
    const last = sessionEntries[sessionEntries.length - 1];
    if (!last) return 'Thinking';
    if (last.entryType === 'thinking') return 'Thinking';
    if (last.entryType === 'toolCall') return `Running ${last.tool ?? 'tool'}`;
    if (last.entryType === 'toolResult') return 'Processing result';
    return 'Thinking';
  })();
</script>

<div class="central-panel">
  <div class="header">
    <div class="left">
      <span class="name">{session.projectName ?? session.cwd ?? 'Session'}</span>
      <span class="status {session.status}">
        {#if session.status === 'working' || session.status === 'running'}
          <span class="status-dot-anim"></span>
        {/if}
        {session.status.toUpperCase()}
      </span>
      <span class="meta">
        {session.gitBranch ?? ''} · {session.model ?? '—'} · {Math.round(((session.tokens?.input ?? 0) + (session.tokens?.output ?? 0)) / 1000)}K
        {#if (session.contextPercent ?? 0) > 0}
          · {Math.round(session.contextPercent ?? 0)}% ctx
        {/if}
      </span>
    </div>
    <div class="level-toggle">
      <button class:active={$detailLevel === 'compact'} onclick={() => detailLevel.set('compact')}>Compact</button>
      <button class:active={$detailLevel === 'full'} onclick={() => detailLevel.set('full')}>Full</button>
      <button class:active={$detailLevel === 'raw'} onclick={() => detailLevel.set('raw')}>Raw</button>
    </div>
  </div>

  <div class="log-wrapper">
    <div class="log" bind:this={logContainer} onscroll={handleScroll}>
      {#if $detailLevel === 'raw'}
        <pre class="raw-log mono">{JSON.stringify($journal, null, 2)}</pre>
      {:else}
        {#each displayEntries as { entry, resultEntry }, i (entry.timestamp + entry.entryType + i)}
          {@const prevItem = displayEntries[i - 1]}
          {@const isChild = entry.entryType === 'toolCall'}
          {@const isNewGroup = !prevItem ||
            (entry.entryType === 'user' && prevItem.entry.entryType !== 'user') ||
            (entry.entryType === 'assistant' && prevItem.entry.entryType !== 'thinking' && prevItem.entry.entryType !== 'assistant') ||
            (entry.entryType === 'thinking' && prevItem.entry.entryType !== 'thinking')}
          {#if isNewGroup && i > 0}
            <div class="gap"></div>
          {/if}
          <div class="entry-row" class:child={isChild}>
            <JournalEntry {entry} {resultEntry} />
          </div>
        {/each}

        {#each $pendingMessages as msg (msg.id)}
          <div class="pending-msg">
            <span class="pending-icon">↗</span>
            <span class="pending-text">{msg.text}</span>
            <span class="pending-label">sending...</span>
          </div>
        {/each}

        {#if session.status === 'working' || session.status === 'running'}
          <TypingIndicator label={typingLabel} />
        {/if}

        {#if session.pendingApproval && session.status !== 'working'}
          <div class="approval-banner">
            <span class="approval-icon">⏳</span>
            <span class="approval-text">{session.pendingApproval}</span>
          </div>
        {/if}
      {/if}
    </div>

    {#if showScrollBtn}
      <button class="scroll-btn" onclick={scrollToBottom} title="Scroll to bottom">
        ↓
      </button>
    {/if}
  </div>

  <CommandInput sessionId={session.id} agentName={session.projectName ?? 'Session'} agentCwd={session.cwd ?? ''} />
</div>

<style>
  .central-panel {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  .header {
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .left { display: flex; align-items: center; gap: 8px; }
  .name { font-weight: 600; font-size: 14px; }
  .status {
    padding: 2px 8px;
    border-radius: 8px;
    font-size: 11px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .status.working { background: var(--green-dim); color: var(--green); }
  .status.input { background: var(--amber-dim); color: var(--amber); }
  .status.idle { background: var(--bg-idle); color: var(--text-muted); }
  .status-dot-anim {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--green);
    animation: blink 1.2s ease-in-out infinite;
  }
  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
  .meta { font-size: 12px; color: var(--text-dim); }
  .level-toggle {
    display: flex;
    gap: 2px;
    background: var(--bg-overlay);
    border-radius: 6px;
    padding: 2px;
  }
  .level-toggle button {
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 12px;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s;
  }
  .level-toggle button.active {
    background: var(--bg-active-toggle);
    color: var(--blue);
  }

  .log-wrapper {
    flex: 1;
    position: relative;
    overflow: hidden;
  }
  .log {
    height: 100%;
    overflow-y: auto;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .raw-log {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
    white-space: pre-wrap;
  }
  .gap { height: 10px; }
  .entry-row {
    animation: fadeIn 0.2s ease-out;
  }
  .entry-row.child {
    margin-left: 16px;
  }
  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .scroll-btn {
    position: absolute;
    bottom: 12px;
    right: 20px;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-size: 16px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
    transition: all 0.15s;
    z-index: 10;
  }
  .scroll-btn:hover {
    background: var(--bg-tertiary);
    transform: scale(1.1);
  }

  .pending-msg {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 14px;
    border-radius: 10px;
    background: var(--bg-user);
    border: 1px dashed color-mix(in srgb, var(--blue) 40%, transparent);
    margin: 4px 0;
    animation: fadeIn 0.2s ease-out;
    opacity: 0.7;
  }
  .pending-icon {
    color: var(--blue);
    font-size: 12px;
    flex-shrink: 0;
    margin-top: 1px;
  }
  .pending-text {
    font-size: 13px;
    color: var(--text-primary);
    white-space: pre-wrap;
    flex: 1;
    min-width: 0;
  }
  .pending-label {
    font-size: 10px;
    color: var(--text-dim);
    flex-shrink: 0;
    font-style: italic;
  }

  .approval-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: var(--amber-dim);
    border: 1px solid var(--amber);
    border-radius: 8px;
    margin-top: 6px;
    animation: pulseGlow 2s ease-in-out infinite;
  }
  @keyframes pulseGlow {
    0%, 100% { box-shadow: none; }
    50% { box-shadow: 0 0 12px var(--pulse-glow); }
  }
  .approval-icon { font-size: 16px; }
  .approval-text { font-size: 12px; color: var(--amber); font-weight: 500; }
</style>
