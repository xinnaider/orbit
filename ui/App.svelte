<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    sessions,
    selectedSessionId,
    upsertSession,
    updateSessionState,
    getSelectedSession,
  } from './lib/stores/sessions';
  import { assignSession } from './lib/stores/layout';
  import { journal } from './lib/stores/journal';
  import {
    listSessions,
    checkClaude,
    onSessionCreated,
    onSessionOutput,
    onSessionState,
    onSessionStopped,
    onSessionRunning,
    onSessionError,
    onSessionRateLimit,
    getAppVersion,
    getChangelog,
  } from './lib/tauri';
  import type { ClaudeCheck } from './lib/tauri';
  import Banner from './components/Banner.svelte';
  import UpdateBanner from './components/UpdateBanner.svelte';
  import ChangelogModal from './components/ChangelogModal.svelte';
  import { checkUpdate } from './lib/tauri';
  import type { UpdateInfo } from './lib/types';
  import Sidebar from './components/Sidebar.svelte';
  import PaneGrid from './components/PaneGrid.svelte';
  import MetaPanel from './components/MetaPanel.svelte';

  let prevStatuses: Record<number, string> = {};
  let audioCtx: AudioContext | null = null;
  let claudeCheck: ClaudeCheck | null = null;
  let unlisteners: Array<() => void> = [];
  let spawnError: { sessionId: number; error: string } | null = null;
  let rateLimitError: { sessionId: number } | null = null;
  let rateLimitDismissTimer: ReturnType<typeof setTimeout> | null = null;
  let availableUpdate: UpdateInfo | null = null;
  let updateInterval: ReturnType<typeof setInterval> | null = null;
  let showChangelog = false;
  let changelogContent = '';
  let appVersion = '';

  const CHANGELOG_VERSION_KEY = 'orbit:lastSeenChangelogVersion';

  function openChangelog() {
    showChangelog = true;
  }

  function closeChangelog() {
    showChangelog = false;
    localStorage.setItem(CHANGELOG_VERSION_KEY, appVersion);
  }

  function beep() {
    try {
      if (!audioCtx || audioCtx.state === 'closed') audioCtx = new AudioContext();
      const osc = audioCtx.createOscillator();
      const gain = audioCtx.createGain();
      osc.connect(gain);
      gain.connect(audioCtx.destination);
      osc.frequency.value = 880;
      osc.type = 'sine';
      gain.gain.value = 0.15;
      gain.gain.exponentialRampToValueAtTime(0.001, audioCtx.currentTime + 0.15);
      osc.start();
      osc.stop(audioCtx.currentTime + 0.15);
    } catch (_e) {
      /* no-op */
    }
  }

  onMount(async () => {
    const [existing, check, version, changelog] = await Promise.all([
      listSessions(),
      checkClaude(),
      getAppVersion(),
      getChangelog(),
    ]);
    appVersion = version;
    changelogContent = changelog;
    const lastSeen = localStorage.getItem(CHANGELOG_VERSION_KEY);
    if (lastSeen !== version) {
      showChangelog = true;
    }

    claudeCheck = check;
    sessions.set(existing);
    if (existing.length > 0 && !$selectedSessionId) assignSession('tl', existing[0].id);

    const u1 = onSessionCreated((s) => {
      sessions.update((l) => upsertSession(l, s));
      if (!$selectedSessionId) assignSession('tl', s.id);
    });

    const u2 = onSessionOutput(({ sessionId, entry }) => {
      journal.update((map) => new Map(map).set(sessionId, [...(map.get(sessionId) ?? []), entry]));
    });

    const u3 = onSessionState((p) => {
      const prev = prevStatuses[p.sessionId];
      if (p.status === 'input' && prev && prev !== 'input') beep();
      prevStatuses[p.sessionId] = p.status;
      sessions.update((l) =>
        updateSessionState(l, p.sessionId, {
          status: p.status as any,
          tokens: p.tokens,
          contextPercent: p.contextPercent,
          pendingApproval: p.pendingApproval,
          miniLog: p.miniLog,
          costUsd: p.costUsd ?? null,
          gitBranch: p.gitBranch ?? null,
        })
      );
    });

    const u4 = onSessionStopped((id) => {
      sessions.update((l) => updateSessionState(l, id, { status: 'stopped' }));
    });

    const u5 = onSessionRunning((id, pid) => {
      sessions.update((l) => updateSessionState(l, id, { status: 'running', pid }));
    });

    const u6 = onSessionError((id, error) => {
      sessions.update((l) => updateSessionState(l, id, { status: 'error' }));
      spawnError = { sessionId: id, error };
      // Auto-dismiss after 15s
      setTimeout(() => (spawnError = null), 15000);
    });

    const u7 = onSessionRateLimit((id) => {
      rateLimitError = { sessionId: id };
      if (rateLimitDismissTimer) clearTimeout(rateLimitDismissTimer);
      rateLimitDismissTimer = setTimeout(() => (rateLimitError = null), 30000);
    });

    // Resolve all unlisten functions and store for cleanup
    Promise.all([u1, u2, u3, u4, u5, u6, u7]).then((fns) => {
      unlisteners = fns;
    });

    async function tryCheckUpdate() {
      try {
        const info = await checkUpdate();
        if (info) availableUpdate = info;
      } catch (_e) {
        // silencioso — falha de rede não afeta o uso do app
      }
    }

    setTimeout(tryCheckUpdate, 3000);
    updateInterval = setInterval(tryCheckUpdate, 30 * 60 * 1000);
  });

  onDestroy(() => {
    unlisteners.forEach((fn) => fn());
    if (updateInterval) clearInterval(updateInterval);
  });

  $: selected = getSelectedSession($sessions, $selectedSessionId);
</script>

{#if rateLimitError}
  <Banner
    variant="warning"
    icon="⏳"
    title="rate limit reached"
    message="Please wait a moment and try again."
    onDismiss={() => (rateLimitError = null)}
  />
{/if}

{#if spawnError}
  <Banner
    variant="error"
    icon="⚠"
    title={`session #${spawnError.sessionId} failed to spawn`}
    message={spawnError.error}
    zIndex={499}
    onDismiss={() => (spawnError = null)}
  />
{/if}

{#if availableUpdate}
  <UpdateBanner update={availableUpdate} onDismiss={() => (availableUpdate = null)} />
{/if}

{#if showChangelog}
  <ChangelogModal {changelogContent} currentVersion={appVersion} onClose={closeChangelog} />
{/if}

<div class="layout">
  <Sidebar onOpenChangelog={openChangelog} />
  {#if claudeCheck && !claudeCheck.found}
    <div class="empty">
      <div class="claude-warn">
        <span class="warn-icon">⚠</span>
        <div>
          <div class="warn-title">claude CLI not found</div>
          <div class="warn-hint">
            {claudeCheck.hint ?? 'npm install -g @anthropic-ai/claude-code'}
          </div>
        </div>
      </div>
    </div>
  {:else}
    <PaneGrid />
  {/if}
  {#if selected}
    <MetaPanel session={selected} />
  {/if}
</div>

<style>
  .layout {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
  }
  .claude-warn {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    background: rgba(224, 72, 72, 0.07);
    border: 1px solid rgba(224, 72, 72, 0.25);
    border-radius: 4px;
    padding: 14px 18px;
    max-width: 360px;
  }
  .warn-icon {
    color: var(--s-error);
    font-size: 16px;
    flex-shrink: 0;
    margin-top: 1px;
  }
  .warn-title {
    font-size: var(--md);
    color: var(--s-error);
    margin-bottom: 4px;
  }
  .warn-hint {
    font-size: var(--xs);
    color: var(--t1);
    font-style: italic;
  }
</style>
