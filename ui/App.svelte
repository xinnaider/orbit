<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { IS_WEB, HAS_TAURI } from './lib/tauri/invoke';
  import { getStoredToken } from './lib/tauri/web-adapter';
  import WebLoginScreen from './components/WebLoginScreen.svelte';

  let webAuthenticated = !IS_WEB || !!getStoredToken();
  const isMobile = IS_WEB && window.innerWidth < 768;
  import {
    sessions,
    selectedSessionId,
    upsertSession,
    updateSessionState,
    getSelectedSession,
    type Session,
  } from './lib/stores/sessions';
  import { get } from 'svelte/store';
  import { assignSession, restoreWorkspace, workspace } from './lib/stores/workspace';
  import { journal } from './lib/stores/journal';
  import { taskUpdateTrigger } from './lib/stores/tasks';
  import { addToast } from './lib/stores/toasts';
  import {
    listSessions,
    checkClaude,
    getProviders,
    onSessionCreated,
    onSessionOutput,
    onSessionState,
    onSessionStopped,
    onSessionRunning,
    onSessionError,
    onSessionRateLimit,
    onSessionTaskUpdate,
    getAppVersion,
    getChangelog,
  } from './lib/tauri';
  import { listen } from './lib/tauri/invoke';
  import type { ClaudeCheck } from './lib/tauri';
  import ChangelogModal from './components/ChangelogModal.svelte';
  import ToastContainer from './components/ToastContainer.svelte';
  import { checkUpdate } from './lib/tauri';
  import { installUpdate } from './lib/tauri';
  import type { UpdateInfo } from './lib/types';
  import Sidebar from './components/Sidebar.svelte';
  import WorkspaceContainer from './components/workspace/WorkspaceContainer.svelte';
  import NewSessionModal from './components/NewSessionModal.svelte';
  import MetaPanel from './components/MetaPanel.svelte';
  import { metaPanelVisible, sidebarVisible } from './lib/stores/preferences';
  import { mutedSessions } from './lib/stores/ui';
  import { backends } from './lib/stores/providers';

  let prevStatuses: Record<number, string> = {};
  let audioCtx: AudioContext | null = null;
  let claudeCheck: ClaudeCheck | null = null;
  let unlisteners: Array<() => void> = [];
  let updateInterval: ReturnType<typeof setInterval> | null = null;
  let showChangelog = false;
  let changelogContent = '';
  let appVersion = '';
  let pendingUpdate: UpdateInfo | null = null;
  let updateToastId: string | null = null;

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
    if (IS_WEB && !webAuthenticated) return;

    if (isMobile) {
      sidebarVisible.set(false);
      metaPanelVisible.set(false);
    }

    const [existing, check, version, changelog, providerList] = await Promise.all([
      listSessions(),
      checkClaude(),
      getAppVersion(),
      getChangelog(),
      getProviders().catch(() => []),
    ]);
    backends.set(providerList);
    appVersion = version;
    changelogContent = changelog;
    const lastSeen = localStorage.getItem(CHANGELOG_VERSION_KEY);
    if (lastSeen !== version) {
      showChangelog = true;
    }

    claudeCheck = check;
    sessions.set(existing);
    restoreWorkspace(new Set(existing.map((s) => s.id)));
    if (existing.length > 0 && !$selectedSessionId) {
      const ws = get(workspace);
      if (ws.focusedPaneId) assignSession(ws.focusedPaneId, existing[0].id);
    }

    const u1 = onSessionCreated((s) => {
      sessions.update((l) => upsertSession(l, s));
      if (!$selectedSessionId) {
        const ws = get(workspace);
        if (ws.focusedPaneId) assignSession(ws.focusedPaneId, s.id);
      }
    });

    const u2 = onSessionOutput(({ sessionId, entry }) => {
      journal.update((map) => new Map(map).set(sessionId, [...(map.get(sessionId) ?? []), entry]));
    });

    const u3 = onSessionState((p) => {
      const prev = prevStatuses[p.sessionId];
      if (p.status === 'input' && prev && prev !== 'input') {
        if (!mutedSessions.isMuted($mutedSessions, String(p.sessionId))) beep();
      }
      prevStatuses[p.sessionId] = p.status;
      // 'idle' and 'new' are agent-level pauses emitted while the process is still running.
      // Map them to 'running' so the working indicator stays visible until session:stopped fires.
      const sessionStatus = p.status === 'idle' || p.status === 'new' ? 'running' : p.status;
      sessions.update((l) =>
        updateSessionState(l, p.sessionId, {
          status: sessionStatus as any,
          tokens: p.tokens,
          contextPercent: p.contextPercent,
          pendingApproval: p.pendingApproval,
          miniLog: p.miniLog,
          gitBranch: p.gitBranch ?? null,
          subagents: p.subagents,
          // Only overwrite model/contextWindow when the stream provides them
          // (Codex/OpenCode don't emit model — preserve the one set at creation)
          ...(p.model != null ? { model: p.model } : {}),
          ...(p.contextWindow != null ? { contextWindow: p.contextWindow } : {}),
          ...(p.attention != null ? { attention: p.attention } : {}),
          ...(p.rateLimit?.length ? { rateLimit: p.rateLimit } : {}),
          ...(p.costUsd != null ? { costUsd: p.costUsd } : {}),
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
      addToast({
        type: 'error',
        message: `session #${id} failed to spawn: ${error}`,
        autoDismiss: false,
      });
    });

    const u7 = onSessionRateLimit((_id) => {
      // Rate limit info is shown inline in the chat feed as a System entry
    });

    const u8 = onSessionTaskUpdate((id) => {
      taskUpdateTrigger.set(id);
    });

    // Resolve all unlisten functions and store for cleanup
    Promise.all([u1, u2, u3, u4, u5, u6, u7, u8]).then((fns) => {
      unlisteners = fns;
    });

    async function tryCheckUpdate() {
      try {
        const info = await checkUpdate();
        if (info && !updateToastId) {
          pendingUpdate = info;
          updateToastId = addToast({
            type: 'update',
            message: `new version available — ${info.version}`,
            autoDismiss: false,
            action: {
              label: 'update now',
              onClick: () => {
                installUpdate();
                updateToastId = null;
              },
            },
          });
        }
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        addToast({ type: 'error', message: `update check failed: ${msg}`, autoDismiss: false });
      }
    }

    setTimeout(tryCheckUpdate, 3000);
    updateInterval = setInterval(tryCheckUpdate, 30 * 60 * 1000);

    window.addEventListener('orbit:new-session', handleOrbitNewSession);
  });

  onDestroy(() => {
    unlisteners.forEach((fn) => fn());
    if (updateInterval) clearInterval(updateInterval);
    window.removeEventListener('orbit:new-session', handleOrbitNewSession);
  });

  // Derive selected session from workspace focused pane
  $: selected = (() => {
    const ws = $workspace;
    const focusedPane = ws.focusedPaneId ? ws.panes[ws.focusedPaneId] : null;
    if (focusedPane?.sessionId) {
      return getSelectedSession($sessions, focusedPane.sessionId);
    }
    return null;
  })();

  let showNewSessionModal = false;

  function handleOrbitNewSession() {
    showNewSessionModal = true;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'F12' && HAS_TAURI) {
      import('@tauri-apps/api/webviewWindow').then(({ getCurrentWebviewWindow }) => {
        (getCurrentWebviewWindow() as unknown as { openDevtools(): void }).openDevtools();
      });
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if IS_WEB && !webAuthenticated}
  <WebLoginScreen
    on:authenticated={() => {
      webAuthenticated = true;
      window.location.reload();
    }}
  />
{:else}
  {#if showChangelog}
    <ChangelogModal {changelogContent} currentVersion={appVersion} onClose={closeChangelog} />
  {/if}

  {#if showNewSessionModal}
    <NewSessionModal
      on:done={() => (showNewSessionModal = false)}
      on:cancel={() => (showNewSessionModal = false)}
    />
  {/if}

  {#if isMobile}
    <div class="mobile-beta-banner">
      <span class="mobile-beta-pill">mobile beta</span>
      <p>Phone access is in testing. Some screens and actions may not work as expected yet.</p>
    </div>
  {/if}

  <div class="layout" class:mobile={isMobile}>
    {#if isMobile}
      <div class="mobile-topbar">
        <button class="hamburger-btn" on:click={() => sidebarVisible.set(true)} aria-label="Open sidebar">
          <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
            <rect x="2" y="4" width="16" height="2" rx="1" fill="currentColor"/>
            <rect x="2" y="9" width="16" height="2" rx="1" fill="currentColor"/>
            <rect x="2" y="14" width="16" height="2" rx="1" fill="currentColor"/>
          </svg>
        </button>
        <span class="mobile-title">orbit</span>
      </div>
      {#if $sidebarVisible}
        <button class="sidebar-overlay" on:click={() => sidebarVisible.set(false)} aria-label="Close sidebar"></button>
        <Sidebar onOpenChangelog={openChangelog} />
      {/if}
    {:else if $sidebarVisible}
      <Sidebar onOpenChangelog={openChangelog} />
    {:else}
      <button class="sidebar-reopen" on:click={() => sidebarVisible.set(true)} title="Show sidebar"
        >›</button
      >
    {/if}
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
      <WorkspaceContainer />
    {/if}
    {#if selected && $metaPanelVisible}
      <MetaPanel session={selected} />
    {:else if selected && !$metaPanelVisible}
      <button class="meta-reopen" on:click={() => metaPanelVisible.set(true)} title="Show panel"
        >‹</button
      >
    {/if}
  </div>
{/if}

<ToastContainer />

<style>
  .mobile-beta-banner {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    margin: 10px 10px 0;
    padding: 12px 14px;
    border-radius: 12px;
    border: 1px solid rgba(245, 166, 35, 0.28);
    background: rgba(245, 166, 35, 0.08);
  }
  .mobile-beta-banner p {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--t1);
  }
  .mobile-beta-pill {
    flex-shrink: 0;
    padding: 4px 7px;
    border-radius: 999px;
    background: rgba(245, 166, 35, 0.14);
    color: var(--warning, #f5a623);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
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
  .sidebar-reopen {
    flex-shrink: 0;
    width: 20px;
    background: var(--bg1);
    border: none;
    border-right: 1px solid var(--bd);
    color: var(--t2);
    font-size: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    transition:
      color 0.15s,
      background 0.15s;
  }
  .sidebar-reopen:hover {
    color: var(--t0);
    background: var(--bg2);
  }
  .meta-reopen {
    flex-shrink: 0;
    width: 20px;
    background: var(--bg1);
    border: none;
    border-left: 1px solid var(--bd);
    color: var(--t2);
    font-size: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    transition:
      color 0.15s,
      background 0.15s;
  }
  .meta-reopen:hover {
    color: var(--t0);
    background: var(--bg2);
  }

  /* ── Mobile ──────────────────────────────────────────────── */
  .sidebar-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 90;
    border: none;
    cursor: default;
  }

  .layout.mobile :global(.sidebar) {
    position: fixed;
    top: 0;
    left: 0;
    bottom: 0;
    z-index: 100;
    width: 260px;
    box-shadow: 4px 0 24px rgba(0, 0, 0, 0.4);
  }

  .mobile-topbar {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-3) var(--sp-5);
    background: var(--bg1);
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
    position: sticky;
    top: 0;
    z-index: 50;
  }

  .hamburger-btn {
    background: none;
    border: none;
    color: var(--t1);
    padding: var(--sp-2);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .hamburger-btn:hover {
    color: var(--t0);
  }

  .mobile-title {
    font-size: var(--md);
    color: var(--t2);
    letter-spacing: 0.08em;
    font-weight: 500;
  }

  .layout.mobile {
    flex-direction: column;
  }

  .layout.mobile .sidebar-reopen {
    display: none;
  }

  .layout.mobile .meta-reopen {
    display: none;
  }

  .layout.mobile :global(.copy-btn) {
    display: none !important;
  }
</style>
