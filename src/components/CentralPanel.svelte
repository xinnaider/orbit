<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { Session } from '../lib/stores/sessions';
  import { journal, pendingMessages } from '../lib/stores/journal';
  import { getSessionJournal } from '../lib/tauri';
  import { statusColor, statusLabel, isPulsing } from '../lib/status';
  import { formatTokens } from '../lib/cost';
  import Feed from './Feed.svelte';
  import InputBar from './InputBar.svelte';

  export let session: Session;

  let feedEl: HTMLDivElement;
  let atBottom = true;

  async function loadHistory(id: number) {
    try {
      const entries = await getSessionJournal(id);
      if (entries.length > 0) {
        journal.update(m => new Map(m).set(id, entries));
      }
    } catch {}
  }

  $: if (session) loadHistory(session.id);

  // Auto-scroll + clear pending messages when new entries arrive
  $: {
    const e = $journal.get(session?.id);
    if (e && e.length > 0) pendingMessages.clear();
    scrollIfNeeded();
  }

  async function scrollIfNeeded() {
    if (!atBottom) return;
    await tick();
    if (feedEl) feedEl.scrollTop = feedEl.scrollHeight;
  }

  function onScroll() {
    if (!feedEl) return;
    const { scrollTop, scrollHeight, clientHeight } = feedEl;
    atBottom = scrollHeight - scrollTop - clientHeight < 80;
  }

  function scrollToBottom() {
    if (feedEl) { feedEl.scrollTop = feedEl.scrollHeight; atBottom = true; }
  }

  $: entries = $journal.get(session?.id) ?? [];
  $: statusStr = statusLabel(session?.status ?? '');
  $: statusClr = statusColor(session?.status ?? '');
  $: pulsing = isPulsing(session?.status ?? '');

  function fmtModel(m: string | null) {
    if (!m) return 'auto';
    if (m.includes('opus'))   return 'opus-4.6';
    if (m.includes('sonnet')) return 'sonnet-4.6';
    if (m.includes('haiku'))  return 'haiku-4.5';
    return m;
  }
</script>

<div class="panel">
  <!-- Header -->
  <div class="header">
    <div class="header-left">
      <span class="dot" style="color:{statusClr}" class:pulse={pulsing}>●</span>
      <span class="session-name">
        {session.name ?? session.projectName ?? session.cwd?.split(/[\\/]/).pop() ?? `#${session.id}`}
      </span>
      {#if session.gitBranch}
        <span class="branch">{session.gitBranch}</span>
      {/if}
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
              <span class="ctx-fill" style="width:{Math.min(session.contextPercent ?? 0, 100)}%;
                background:{(session.contextPercent ?? 0) > 85 ? 'var(--s-error)' :
                             (session.contextPercent ?? 0) > 65 ? 'var(--s-input)' :
                             'var(--ac)'}">
              </span>
            </span>
            <span class="ctx-pct">{Math.round(session.contextPercent ?? 0)}%</span>
          </span>
        {/if}
      {/if}
      <span class="model">{fmtModel(session.model)}</span>
    </div>
  </div>

  <!-- Approval banner -->
  {#if session.pendingApproval && session.status !== 'working'}
    <div class="approval">
      <span class="approval-icon">⚑</span>
      <span class="approval-text">{session.pendingApproval}</span>
    </div>
  {/if}

  <!-- Feed -->
  <div class="feed-wrap" bind:this={feedEl} on:scroll={onScroll}>
    {#if entries.length === 0 && $pendingMessages.length === 0}
      <div class="feed-empty">
        <span>session #{session.id} · {statusStr}</span>
      </div>
    {:else}
      <Feed {entries} status={session.status} />
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
    flex: 1; min-width: 0; min-height: 0;
    display: flex; flex-direction: column;
    overflow: hidden;
    background: var(--bg);
  }

  .header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 14px;
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
    background: var(--bg1);
  }
  .header-left { display: flex; align-items: center; gap: 8px; }
  .dot { font-size: 8px; line-height: 1; }
  .dot.pulse { animation: pulse 2s ease-in-out infinite; }
  @keyframes pulse { 0%,100%{opacity:1} 50%{opacity:0.25} }
  .session-name { font-size: var(--md); font-weight: 500; color: var(--t0); }
  .branch {
    font-size: var(--xs); color: var(--t2);
    background: var(--bg3); border: 1px solid var(--bd);
    border-radius: 2px; padding: 0 5px;
  }
  .status { font-size: var(--xs); color: var(--t2); letter-spacing: 0.04em; }

  .header-right { display: flex; align-items: center; gap: 10px; }
  .meta { font-size: var(--xs); color: var(--t2); }
  .ctx { display: flex; align-items: center; gap: 5px; }
  .ctx-bar {
    width: 40px; height: 3px;
    background: var(--bg3); border-radius: 2px; overflow: hidden;
  }
  .ctx-fill { height: 100%; border-radius: 2px; transition: width 0.3s; }
  .ctx-pct { font-size: var(--xs); color: var(--t2); }
  .model { font-size: var(--xs); color: var(--t2); }

  .approval {
    display: flex; align-items: center; gap: 8px;
    padding: 6px 14px;
    background: rgba(232,160,48,0.07);
    border-bottom: 1px solid rgba(232,160,48,0.2);
    flex-shrink: 0;
  }
  .approval-icon { color: var(--s-input); font-size: var(--md); }
  .approval-text { font-size: var(--sm); color: var(--s-input); }

  .feed-wrap {
    flex: 1; overflow-y: auto; min-height: 0;
    padding: 0;
  }
  .feed-empty {
    display: flex; align-items: center; justify-content: center;
    height: 100%; font-size: var(--sm); color: var(--t3);
  }

  .pending-msg {
    display: flex; gap: 8px; align-items: flex-start;
    padding: 8px 14px 8px 10px;
    font-size: var(--base);
    color: var(--t1); opacity: 0.6;
    border-left: 2px solid var(--user-fg);
    margin: 2px 0;
  }
  .pending-arrow { color: var(--user-fg); flex-shrink: 0; }

  .scroll-btn {
    position: absolute; bottom: 56px; right: 16px; z-index: 10;
    background: var(--bg2); border: 1px solid var(--bd1);
    border-radius: 3px; color: var(--t1);
    font-size: var(--xs); padding: 4px 10px;
  }
  .scroll-btn:hover { border-color: var(--ac); color: var(--ac); }
</style>
