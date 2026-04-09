<script lang="ts">
  import type { Session } from '../lib/stores/sessions';
  import { journal, pendingMessages } from '../lib/stores/journal';
  import { getSessionJournal } from '../lib/tauri';
  import { statusColor, statusLabel, isPulsing } from '../lib/status';
  import { formatTokens } from '../lib/cost';
  import Feed from './Feed.svelte';
  import InputBar from './InputBar.svelte';

  export let session: Session;
  export let onSplit: (() => void) | null = null;
  export let onClose: (() => void) | null = null;

  let feedComponent: Feed;
  let atBottom = true;

  async function loadHistory(id: number) {
    try {
      const entries = await getSessionJournal(id);
      if (entries.length > 0) {
        journal.update((m) => new Map(m).set(id, entries));
      }
    } catch (_e) {
      /* no-op */
    }
  }

  let loadedId: number | null = null;
  $: if (session?.id != null && session.id !== loadedId) {
    loadedId = session.id;
    loadHistory(session.id);
  }

  // Clear pending only when assistant responds (not on user entry echo)
  $: {
    const e = $journal.get(session?.id);
    if (e && e.some((entry) => entry.entryType === 'assistant' || entry.entryType === 'toolCall')) {
      pendingMessages.clear();
    }
  }

  function onFeedBottomChange(event: CustomEvent<{ atBottom: boolean }>) {
    atBottom = event.detail.atBottom;
  }

  function scrollToBottom() {
    feedComponent?.scrollToBottom();
    atBottom = true;
  }

  $: entries = $journal.get(session?.id) ?? [];
  $: statusStr = statusLabel(session?.status ?? '');
  $: statusClr = statusColor(session?.status ?? '');
  $: pulsing = isPulsing(session?.status ?? '');

  function fmtModel(m: string | null) {
    if (!m) return 'auto';
    if (m.includes('opus')) return 'opus-4.6';
    if (m.includes('sonnet')) return 'sonnet-4.6';
    if (m.includes('haiku')) return 'haiku-4.5';
    return m;
  }
</script>

<div class="panel">
  <!-- Header -->
  <div class="header">
    <div class="header-left">
      <span class="dot" style="color:{statusClr}" class:pulse={pulsing}>●</span>
      <span
        class="session-name"
        title={session.name ??
          session.projectName ??
          session.cwd?.split(/[\\/]/).pop() ??
          `#${session.id}`}
      >
        {session.name ??
          session.projectName ??
          session.cwd?.split(/[\\/]/).pop() ??
          `#${session.id}`}
      </span>
      <span class="status" style="color:{statusClr}">{statusStr}</span>
    </div>
    <div class="header-right">
      {#if session.tokens}
        <span class="meta">
          {formatTokens(session.tokens.input + session.tokens.output)}
        </span>
        {#if (session.contextPercent ?? 0) > 0}
          <span class="ctx">
            <span class="ctx-bar">
              <span
                class="ctx-fill"
                style="width:{Math.min(session.contextPercent ?? 0, 100)}%;
                background:{(session.contextPercent ?? 0) > 85
                  ? 'var(--s-error)'
                  : (session.contextPercent ?? 0) > 65
                    ? 'var(--s-input)'
                    : 'var(--ac)'}"
              >
              </span>
            </span>
            <span class="ctx-pct">{Math.round(session.contextPercent ?? 0)}%</span>
          </span>
        {/if}
      {/if}
      <span class="model">{fmtModel(session.model)}</span>
      {#if onSplit || onClose}
        <div class="header-actions">
          {#if onSplit}
            <button class="action-btn" title="Split panel" on:click={onSplit}>⊞</button>
          {/if}
          {#if onClose}
            <button class="action-btn close-action" title="Close panel" on:click={onClose}>×</button
            >
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <!-- Branch strip -->
  {#if session.branchName ?? session.gitBranch}
    {@const branchLabel = session.branchName ?? session.gitBranch ?? ''}
    <div class="branch-strip" title={branchLabel}>
      <span class="branch-icon">⎇</span>
      <span class="branch-text">{branchLabel}</span>
    </div>
  {/if}

  <!-- Approval banner -->
  {#if session.pendingApproval && (session.status as string) !== 'working'}
    <div class="approval">
      <span class="approval-icon">⚑</span>
      <span class="approval-text">{session.pendingApproval}</span>
    </div>
  {/if}

  <!-- Feed -->
  <div class="feed-wrap">
    {#if entries.length === 0 && $pendingMessages.length === 0}
      <div class="feed-empty">
        <span>session #{session.id} · {statusStr}</span>
      </div>
    {:else}
      <Feed
        bind:this={feedComponent}
        {entries}
        status={session.status}
        on:bottomchange={onFeedBottomChange}
      />
      {#each $pendingMessages as msg (msg.id)}
        <div class="pending-msg">
          <span class="pending-arrow">›</span>
          <span>{msg.text}</span>
        </div>
      {/each}
    {/if}
  </div>

  {#if !atBottom}
    <button class="scroll-btn" on:click={scrollToBottom}>↓ scroll to bottom</button>
  {/if}

  <!-- Input -->
  <InputBar sessionId={session.id} cwd={session.cwd ?? ''} sessionStatus={session.status} />
</div>

<style>
  .panel {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg);
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 14px;
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
    background: var(--bg1);
  }
  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 1;
    overflow: hidden;
  }
  .dot {
    font-size: 8px;
    line-height: 1;
  }
  .dot.pulse {
    animation: pulse 2s ease-in-out infinite;
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.25;
    }
  }
  .session-name {
    font-size: var(--md);
    font-weight: 500;
    color: var(--t0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .branch-strip {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 2px 14px;
    border-bottom: 1px solid var(--bd);
    background: var(--bg1);
    flex-shrink: 0;
    min-width: 0;
    overflow: hidden;
  }
  .branch-icon {
    font-size: 10px;
    color: var(--t3);
    flex-shrink: 0;
  }
  .branch-text {
    font-size: var(--xs);
    color: var(--t3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status {
    font-size: var(--xs);
    color: var(--t2);
    letter-spacing: 0.04em;
    flex-shrink: 0;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
    padding-left: 12px;
  }
  .meta {
    font-size: var(--xs);
    color: var(--t2);
  }
  .ctx {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .ctx-bar {
    width: 40px;
    height: 3px;
    background: var(--bg3);
    border-radius: 2px;
    overflow: hidden;
  }
  .ctx-fill {
    height: 100%;
    border-radius: 2px;
    transition: width 0.3s;
  }
  .ctx-pct {
    font-size: var(--xs);
    color: var(--t2);
  }
  .model {
    font-size: var(--xs);
    color: var(--t2);
  }

  .approval {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    background: rgba(232, 160, 48, 0.07);
    border-bottom: 1px solid rgba(232, 160, 48, 0.2);
    flex-shrink: 0;
  }
  .approval-icon {
    color: var(--s-input);
    font-size: var(--md);
  }
  .approval-text {
    font-size: var(--sm);
    color: var(--s-input);
  }

  .feed-wrap {
    flex: 1;
    overflow: hidden;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 3px;
    margin-left: 4px;
  }

  .action-btn {
    background: var(--bg3);
    border: 1px solid var(--bd1);
    color: var(--t2);
    width: 18px;
    height: 18px;
    border-radius: 3px;
    font-size: 11px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    transition:
      border-color 0.15s,
      color 0.15s;
  }

  .action-btn:hover {
    border-color: var(--ac);
    color: var(--ac);
  }

  .action-btn.close-action:hover {
    border-color: var(--s-error);
    color: var(--s-error);
  }
  .feed-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    font-size: var(--sm);
    color: var(--t3);
  }

  .pending-msg {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    padding: 8px 14px 8px 10px;
    font-size: var(--base);
    color: var(--t1);
    opacity: 0.6;
    border-left: 2px solid var(--user-fg);
    margin: 2px 0;
  }
  .pending-arrow {
    color: var(--user-fg);
    flex-shrink: 0;
  }

  .scroll-btn {
    position: absolute;
    bottom: 56px;
    right: 16px;
    z-index: 10;
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: 3px;
    color: var(--t1);
    font-size: var(--xs);
    padding: 4px 10px;
  }
  .scroll-btn:hover {
    border-color: var(--ac);
    color: var(--ac);
  }
</style>
