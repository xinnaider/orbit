<script lang="ts">
  import type { Session } from '../lib/stores/sessions';
  import { journal, pendingMessages } from '../lib/stores/journal';
  import { getSessionJournal } from '../lib/tauri';
  import { backends as backendsStore } from '../lib/stores/providers';
  import { statusColor, statusLabel, isPulsing, modelShortName } from '../lib/status';
  import { formatTokens } from '../lib/cost';
  import { mutedSessions, toggleMute } from '../lib/stores/ui';
  import Feed from './Feed.svelte';
  import InputBar from './InputBar.svelte';
  import PermissionDialog from './PermissionDialog.svelte'; // TODO: re-enable when auto-deny error is fixed

  export let session: Session;
  export let onClose: (() => void) | null = null;
  export let paneId: string = '';

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
  $: muted = mutedSessions.isMuted($mutedSessions, String(session?.id));

  function fmtModel(m: string | null): string {
    return modelShortName(m);
  }

  // Provider models for /model autocomplete — read from store
  function parseToolName(approval: string): string {
    const match = approval.match(/^Allow\s+(.+?)\?/);
    return match ? match[1] : approval;
  }

  function parseToolDesc(approval: string): string {
    const match = approval.match(/^Allow\s+.+?\?\s*(.*)/);
    return match ? match[1].trim() : '';
  }

  $: providerModelIds = (() => {
    const p = session?.provider ?? 'claude-code';
    // Find models from matching backend or sub-provider
    for (const b of $backendsStore) {
      if (b.id === p) return b.models.map((m) => m.id);
      const sub = b.subProviders?.find((s) => s.id === p);
      if (sub) return sub.models.map((m) => m.id);
    }
    return [];
  })();
</script>

<div class="panel">
  <!-- Header — draggable to create splits -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="header"
    draggable="true"
    on:dragstart={(e) => {
      if (e.dataTransfer) {
        e.dataTransfer.setData(
          'text/plain',
          JSON.stringify({ sessionId: session.id, sourcePaneId: paneId })
        );
        e.dataTransfer.effectAllowed = 'move';
      }
    }}
  >
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
      <span class="model" title={session.model ?? ''}>{fmtModel(session.model)}</span>
      <div class="header-actions">
        <button
          class="action-btn mute-btn"
          class:muted
          title={muted ? 'Unmute session' : 'Mute session'}
          on:click={() => toggleMute(String(session.id))}
        >
          {#if muted}
            <svg
              width="11"
              height="11"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon>
              <line x1="23" y1="9" x2="17" y2="15"></line>
              <line x1="17" y1="9" x2="23" y2="15"></line>
            </svg>
          {:else}
            <svg
              width="11"
              height="11"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon>
              <path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path>
              <path d="M19.07 4.93a10 10 0 0 1 0 14.14"></path>
            </svg>
          {/if}
        </button>
        {#if onClose}
          <button class="action-btn close-action" title="Close pane" on:click={onClose}>×</button>
        {/if}
      </div>
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

  <!-- Approval banner — TODO: re-enable when auto-deny error is fixed -->

  <!-- Feed -->
  <div class="feed-wrap">
    {#if entries.length === 0 && $pendingMessages.length === 0}
      <div class="feed-empty">
        <span>session #{session.id} · {statusStr}</span>
      </div>
    {:else}
      {#key session.id}
        <Feed
          bind:this={feedComponent}
          {entries}
          status={session.status}
          provider={session.provider}
          cwd={session.cwd}
          on:bottomchange={onFeedBottomChange}
        />
      {/key}
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
  <InputBar
    sessionId={session.id}
    cwd={session.cwd ?? ''}
    sessionStatus={session.status}
    provider={session.provider}
    providerModels={providerModelIds}
  />
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
    padding: var(--sp-4) var(--sp-7);
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
    background: var(--bg1);
  }
  .header-left {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
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
    gap: var(--sp-3);
    padding: var(--sp-1) var(--sp-7);
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
    gap: var(--sp-5);
    flex-shrink: 0;
    padding-left: var(--sp-6);
  }
  .meta {
    font-size: var(--xs);
    color: var(--t2);
  }
  .ctx {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }
  .ctx-bar {
    width: 40px;
    height: 3px;
    background: var(--bg3);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .ctx-fill {
    height: 100%;
    border-radius: var(--radius-sm);
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

  /* approval banner CSS removed — TODO: re-enable when auto-deny error is fixed */

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
    gap: var(--sp-2);
    margin-left: var(--sp-2);
  }

  .action-btn {
    background: var(--bg3);
    border: 1px solid var(--bd1);
    color: var(--t2);
    width: 18px;
    height: 18px;
    border-radius: var(--radius-sm);
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

  .mute-btn {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .mute-btn.muted {
    border-color: var(--t3);
    color: var(--t3);
  }

  .mute-btn.muted:hover {
    border-color: var(--ac);
    color: var(--ac);
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
    gap: var(--sp-4);
    align-items: flex-start;
    padding: var(--sp-4) var(--sp-7) var(--sp-4) var(--sp-5);
    font-size: var(--base);
    color: var(--t1);
    opacity: 0.6;
    border-left: 2px solid var(--user-fg);
    margin: var(--sp-1) 0;
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
    border-radius: var(--radius-sm);
    color: var(--t1);
    font-size: var(--xs);
    padding: var(--sp-2) var(--sp-5);
  }
  .scroll-btn:hover {
    border-color: var(--ac);
    color: var(--ac);
  }
</style>
