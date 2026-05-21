<script lang="ts">
  import type { Session } from '../lib/stores/sessions';
  import { GitBranch, Volume2, VolumeX } from 'lucide-svelte';
  import { journal, pendingMessages } from '../lib/stores/journal';
  import { rawJournal } from '../lib/stores/rawJournal';
  import { backends as backendsStore } from '../lib/stores/providers';
  import { getSessionJournal, getSessionRawOutputs } from '../lib/tauri/sessions';
  import { invoke } from '../lib/tauri/invoke';
  import { updateSessionState, sessions } from '../lib/stores/sessions';
  import { statusColor, statusLabel, isPulsing, modelShortName } from '../lib/status';
  import { formatTokensLong } from '../lib/cost';
  import { mutedSessions, toggleMute } from '../lib/stores/ui';
  import Feed from './Feed.svelte';
  import RawFeed from './RawFeed.svelte';
  import InputBar from './InputBar.svelte';
  import PanelHeader from './workspace/PanelHeader.svelte';
  import PermissionDialog from './PermissionDialog.svelte';

  export let session: Session;
  export let onClose: (() => void) | null = null;
  export let paneId: string = '';
  export let focused: boolean = true;
  export let compact: boolean = false;

  let feedComponent: Feed;
  let atBottom = true;
  let viewMode: 'chat' | 'raw' = 'chat';
  let rawLoading = false;

  // Load DB history once on mount
  let loadedRaw = false;
  async function loadRawHistory(id: number) {
    if (loadedRaw) return;
    loadedRaw = true;
    rawLoading = true;
    try {
      const lines = await getSessionRawOutputs(id);
      rawJournal.update((m) => {
        const existing = m.get(id) ?? [];
        if (lines.length > existing.length) {
          return new Map(m).set(id, lines);
        }
        return m;
      });
    } catch {
      /* no-op */
    } finally {
      rawLoading = false;
    }
  }

  $: if (session?.id != null) {
    loadedRaw = false;
    loadRawHistory(session.id);
  }

  $: rawLines = $rawJournal.get(session?.id) ?? [];

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

  // Auto-detect git branch if not set yet
  async function fetchBranch() {
    if (!session.cwd) return;
    try {
      const branch = await invoke<string | null>('git_branch', { cwd: session.cwd });
      if (branch && (session.gitBranch ?? null) !== branch) {
        sessions.update((l) => updateSessionState(l, session.id, { gitBranch: branch }));
      }
    } catch {
      /* not a git repo — no-op */
    }
  }

  let loadedId: number | null = null;
  $: if (session?.id != null && session.id !== loadedId) {
    loadedId = session.id;
    loadHistory(session.id);
    fetchBranch();
  }

  $: entries = $journal.get(session?.id) ?? [];

  // Clear pending messages only when a NEW entry arrives (entries grew).
  // This avoids clearing pendingMessages on spurious re-evaluations
  // triggered by loadHistory or unrelated store updates.
  let prevEntryCount = 0;
  $: {
    const count = entries.length;
    if (count > prevEntryCount) {
      const last = entries[count - 1];
      if (
        last &&
        (last.entryType === 'user' ||
          last.entryType === 'assistant' ||
          last.entryType === 'toolCall')
      ) {
        pendingMessages.clear();
      }
    }
    prevEntryCount = count;
  }

  function onFeedBottomChange(event: CustomEvent<{ atBottom: boolean }>) {
    atBottom = event.detail.atBottom;
  }

  function scrollToBottom() {
    feedComponent?.scrollToBottom();
    atBottom = true;
  }

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
    title={session.name ??
      session.projectName ??
      session.cwd?.split(/[\\/]/).pop() ??
      `#${session.id}`}
    status={statusStr}
    dragPayload={JSON.stringify({ sessionId: session.id, sourcePaneId: paneId })}
    {onClose}
    {focused}
  >
    <span slot="leading" class="dot" style="color:{statusClr}" class:pulse={pulsing}></span>
    <div slot="meta" class="header-right">
      {#if session.tokens}
        <span class="meta tokens-meta">
          <span class="meta-item"
            >{formatTokensLong(session.tokens.input)}<span class="meta-arrow-up">↑</span></span
          >
          <span class="meta-sep">·</span>
          <span class="meta-item"
            >{formatTokensLong(session.tokens.output)}<span class="meta-arrow-down">↓</span></span
          >
        </span>
        {#if (session.contextPercent ?? 0) > 0}
          <span class="meta-sep">·</span>
          <span class="ctx-pct">{Math.round(session.contextPercent ?? 0)}% ctx</span>
        {/if}
        <span class="meta-sep">·</span>
      {/if}
      <span class="model-pill" title={session.model ?? ''}>
        {fmtModel(session.model)}
      </span>
    </div>
    <div slot="actions" class="header-actions">
      <div class="view-toggle">
        <button
          type="button"
          class="toggle-btn"
          class:active={viewMode === 'chat'}
          on:click={() => (viewMode = 'chat')}
          title="Chat view">chat</button
        >
        <button
          type="button"
          class="toggle-btn"
          class:active={viewMode === 'raw'}
          on:click={() => (viewMode = 'raw')}
          title="Raw JSONL view">raw</button
        >
      </div>
      <button
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
    </div>
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
    {#if viewMode === 'raw'}
      {#if rawLoading && rawLines.length === 0}
        <div class="feed-empty"><span>loading raw output...</span></div>
      {:else}
        <RawFeed lines={rawLines} />
      {/if}
    {:else if entries.length === 0 && $pendingMessages.length === 0}
      <div class="feed-empty">
        <span>session #{session.id} · {statusStr}</span>
      </div>
    {:else}
      {#key session.id}
        <Feed
          bind:this={feedComponent}
          {entries}
          status={session.status}
          provider={session.provider ?? 'claude-code'}
          cwd={session.cwd}
          {compact}
          on:bottomchange={(e) => (atBottom = e.detail.atBottom)}
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
    {compact}
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
    border-radius: 50%;
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
    font-size: 9.5px;
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
  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    flex-shrink: 0;
  }
  .meta {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--t2);
    font-variant-numeric: tabular-nums;
  }
  .tokens-meta {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .meta-item {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .meta-arrow-up {
    color: var(--tool-fg);
    font-size: 9px;
  }
  .meta-arrow-down {
    color: var(--ac);
    font-size: 9px;
  }
  .meta-sep {
    color: var(--t3);
  }
  .ctx-pct {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--t2);
    font-variant-numeric: tabular-nums;
  }
  .ctx-pct {
    font-family: var(--mono);
    font-size: 10px;
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
    font-size: 9.5px;
    font-weight: 500;
    background: var(--bg3);
    color: var(--t2);
    border: 1px solid var(--bd);
    white-space: nowrap;
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
    transition:
      background 0.1s,
      color 0.1s;
  }

  .action-btn:hover {
    background: var(--bg3);
    color: var(--t1);
  }

  .view-toggle {
    display: flex;
    gap: 0;
    flex-shrink: 0;
    margin-right: var(--sp-2);
  }
  .toggle-btn {
    background: none;
    border: 1px solid var(--bd1);
    color: var(--t2);
    font-size: 9px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 2px 7px;
    font-family: var(--mono);
    cursor: pointer;
    line-height: 1;
    height: 20px;
  }
  .toggle-btn:first-child {
    border-radius: 3px 0 0 3px;
  }
  .toggle-btn:last-child {
    border-radius: 0 3px 3px 0;
    border-left: none;
  }
  .toggle-btn.active {
    background: var(--ac-d);
    border-color: var(--ac-border);
    color: var(--ac);
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
