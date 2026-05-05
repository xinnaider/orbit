<script lang="ts">
  import type { Session } from '../lib/stores/sessions';
  import { GitBranch, Volume2, VolumeX } from 'lucide-svelte';
  import { journal, pendingMessages } from '../lib/stores/journal';
  import { getSessionJournal } from '../lib/tauri';
  import { backends as backendsStore } from '../lib/stores/providers';
  import { statusColor, statusLabel, isPulsing, modelShortName } from '../lib/status';
  import { formatTokens } from '../lib/cost';
  import { mutedSessions, toggleMute } from '../lib/stores/ui';
  import Feed from './Feed.svelte';
  import InputBar from './InputBar.svelte';
  import PanelHeader from './workspace/PanelHeader.svelte';
  import PermissionDialog from './PermissionDialog.svelte'; // TODO: re-enable when auto-deny error is fixed

  export let session: Session;
  export let onClose: (() => void) | null = null;
  export let paneId: string = '';
  export let focused: boolean = true;

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
  <PanelHeader
    title={session.name ?? session.projectName ?? session.cwd?.split(/[\\/]/).pop() ?? `#${session.id}`}
    status={statusStr}
    dragPayload={JSON.stringify({ sessionId: session.id, sourcePaneId: paneId })}
    {onClose}
    {focused}
  >
    <span slot="leading" class="dot" style="color:{statusClr}" class:pulse={pulsing}></span>
      <div slot="meta" class="header-right">
        {#if session.tokens}
          <span class="meta">
            {formatTokens(session.tokens.input + session.tokens.output)}
          </span>
          {#if (session.contextPercent ?? 0) > 0}
            <div class="hdr-divider"></div>
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
          <div class="hdr-divider"></div>
        {/if}
        <span class="model-pill" title={session.model ?? ''}>
          {fmtModel(session.model)}
        </span>
      </div>
    <button
      slot="actions"
      class="action-btn mute-btn"
      class:muted
      title={muted ? 'Unmute session' : 'Mute session'}
      aria-label={muted ? 'Unmute session' : 'Mute session'}
      on:click={() => toggleMute(String(session.id))}
    >
      {#if muted}
        <VolumeX size={12} />
      {:else}
        <Volume2 size={12} />
      {/if}
    </button>
  </PanelHeader>

  <!-- Branch strip -->
  {#if session.branchName ?? session.gitBranch}
    {@const branchLabel = session.branchName ?? session.gitBranch ?? ''}
    <div class="branch-strip" title={branchLabel}>
      <span class="branch-icon" aria-hidden="true"><GitBranch size={12} /></span>
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

  .dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 2px;
    background: currentColor;
    flex-shrink: 0;
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
  .branch-strip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    height: 20px;
    border-bottom: 1px solid var(--ac-border);
    background: var(--ac-d2);
    flex-shrink: 0;
    min-width: 0;
    overflow: hidden;
  }
  .branch-icon {
    display: flex;
    color: var(--ac);
    flex-shrink: 0;
  }
  .branch-text {
    font-family: var(--mono);
    font-size: 9px;
    color: var(--ac);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .header-right {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
  .meta {
    font-family: var(--mono);
    font-size: 9.5px;
    color: var(--t2);
    font-variant-numeric: tabular-nums;
  }
  .ctx {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .ctx-bar {
    width: 32px;
    height: 3px;
    background: var(--bg3);
    border-radius: 2px;
    overflow: hidden;
  }
  .ctx-fill {
    height: 100%;
    border-radius: var(--radius-sm);
    transition: width 0.3s;
  }
  .ctx-pct {
    font-family: var(--mono);
    font-size: 9.5px;
    color: var(--t2);
    font-variant-numeric: tabular-nums;
  }
  .model-pill {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    height: 18px;
    padding: 0 6px;
    border-radius: 3px;
    font-family: var(--mono);
    font-size: 9px;
    font-weight: 500;
    background: var(--bg3);
    color: var(--t2);
    border: 1px solid var(--bd);
    white-space: nowrap;
  }
  .hdr-divider {
    width: 1px;
    height: 12px;
    background: var(--bd);
    flex-shrink: 0;
  }

  /* approval banner CSS removed — TODO: re-enable when auto-deny error is fixed */

  .feed-wrap {
    flex: 1;
    overflow: hidden;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: var(--radius-sm);
    border: none;
    background: transparent;
    color: var(--t3);
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.1s, color 0.1s;
  }

  .action-btn:hover {
    background: var(--bg3);
    color: var(--t1);
  }

  .mute-btn {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .mute-btn.muted {
    color: var(--t3);
  }

  .mute-btn.muted:hover {
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
