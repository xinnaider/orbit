<script lang="ts">
  import type { Session } from '../lib/stores/sessions';
  import { journal, pendingMessages } from '../lib/stores/journal';
  import { backends as backendsStore } from '../lib/stores/providers';
  import { getSessionJournal } from '../lib/tauri/sessions';
  import { mergeJournalBySeq } from '../lib/journal-merge';
  import { invoke } from '../lib/tauri/invoke';
  import { updateSessionState, sessions } from '../lib/stores/sessions';
  import { statusColor, statusLabel, modelShortName } from '../lib/status';
  import { shortenPath } from '../lib/path';
  import { metaPanelVisible, compactDensity } from '../lib/stores/preferences';
  import { inspectorToggleHint } from '../lib/shortcuts';
  import Feed from './Feed.svelte';
  import InputBar from './InputBar.svelte';
  import PermissionDialog from './PermissionDialog.svelte';
  import PanelHeader from './workspace/PanelHeader.svelte';

  export let session: Session;
  export let onClose: (() => void) | null = null;
  export let focused: boolean = true;

  // The user's compact-density toggle is the single source of truth. Previously
  // this was OR-ed with a per-pane auto hint, which forced compact "on" whenever
  // a pane was split or had multiple tabs — so the toggle could never turn it off.
  $: effectiveCompact = $compactDensity;

  let feedComponent: Feed;
  let atBottom = true;

  // Load DB history once on mount
  async function loadHistory(id: number) {
    try {
      const entries = await getSessionJournal(id);
      if (entries.length === 0) return;
      journal.update((m) => {
        const existing = m.get(id) ?? [];
        const merged = mergeJournalBySeq(existing, entries);
        return new Map(m).set(id, merged);
      });
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

  function scrollToBottom() {
    feedComponent?.scrollToBottom();
    atBottom = true;
  }

  $: statusStr = statusLabel(session?.status ?? '');
  $: statusClr = statusColor(session?.status ?? '');

  $: topbarBranch = session?.branchName ?? session?.gitBranch ?? null;
  $: topbarPathFull = session?.cwd ?? null;
  $: topbarPath = topbarPathFull ? shortenPath(topbarPathFull) : null;

  function fmtModel(m: string | null): string {
    return modelShortName(m);
  }

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
    branch={topbarBranch}
    path={topbarPath}
    pathFull={topbarPathFull}
    status={statusStr}
    model={fmtModel(session.model)}
    contextPercent={session.contextPercent}
    statusColor={statusClr}
    {onClose}
    {focused}
  />

  <!-- Inspector badge (hidden when meta panel is visible) -->
  {#if !$metaPanelVisible}
    <div
      class="inspector-pop"
      role="button"
      tabindex="0"
      on:click={() => metaPanelVisible.set(true)}
      on:keydown={(e) => e.key === 'Enter' && metaPanelVisible.set(true)}
      title="Toggle inspector panel"
    >
      inspector hidden • {inspectorToggleHint()}
    </div>
  {/if}

  {#if session.pendingApproval}
    <PermissionDialog
      sessionId={session.id}
      toolName={parseToolName(session.pendingApproval)}
      description={parseToolDesc(session.pendingApproval)}
    />
  {/if}

  <!-- Feed -->
  <div class="feed-wrap" data-testid="session-feed">
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
          provider={session.provider ?? 'claude-code'}
          cwd={session.cwd}
          compact={effectiveCompact}
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

    {#if !atBottom}
      <button class="scroll-btn" on:click={scrollToBottom}>↓ scroll to bottom</button>
    {/if}
  </div>

  <!-- Input -->
  <InputBar
    sessionId={session.id}
    cwd={session.cwd ?? ''}
    sessionStatus={session.status}
    provider={session.provider}
    providerModels={providerModelIds}
    compact={effectiveCompact}
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

  /* ── Inspector pop badge ── */
  .inspector-pop {
    position: absolute;
    top: 44px;
    right: 24px;
    display: flex;
    gap: 8px;
    align-items: center;
    color: var(--t3);
    font-family: var(--mono);
    font-size: 10px;
    border: 1px solid var(--bd);
    background: color-mix(in srgb, var(--t0), transparent 97%);
    border-radius: 999px;
    padding: 5px 10px;
    cursor: pointer;
    z-index: 5;
    transition: all 0.15s;
  }
  .inspector-pop:hover {
    border-color: var(--t2);
    color: var(--t1);
    background: color-mix(in srgb, var(--t0), transparent 94%);
  }

  .feed-wrap {
    flex: 1;
    overflow: hidden;
    min-height: 0;
    display: flex;
    flex-direction: column;
    position: relative;
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
    bottom: 14px;
    right: 38px;
    z-index: 10;
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: 999px;
    color: var(--t1);
    font-size: var(--xs);
    padding: var(--sp-3) var(--sp-6);
    box-shadow: 0 14px 34px rgba(0, 0, 0, 0.28);
  }
  .scroll-btn:hover {
    border-color: var(--ac);
    color: var(--ac);
  }

  @media (max-width: 768px) {
    .scroll-btn {
      right: 18px;
      bottom: 10px;
    }
  }
</style>
