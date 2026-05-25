<script lang="ts">
  import { sessions, updateSessionState } from '../lib/stores/sessions';
  import { upsertAndOpenSession } from '../lib/stores/session-actions';
  import NewSessionModal from './NewSessionModal.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import RenameSessionModal from './RenameSessionModal.svelte';
  import { deleteSession, stopSession, getAppVersion } from '../lib/tauri';
  import { appendSessionFeedMessage } from '../lib/session-feed';
  import { mutedSessions, pinnedSessions, togglePin } from '../lib/stores/ui';
  import { modelShortName } from '../lib/status';
  import { onMount } from 'svelte';
  import SessionListItem from './SessionListItem.svelte';
  import McpStatusBadge from './McpStatusBadge.svelte';
  import DesktopNotifyToggle from './DesktopNotifyToggle.svelte';
  import SidebarFooterHints from './SidebarFooterHints.svelte';
  import { expandedParentSessions } from '../lib/stores/mcp-ui';

  function attentionColor(reason: string | null): string {
    switch (reason) {
      case 'permission':
        return 'var(--s-input)';
      case 'completed':
        return 'var(--s-idle)';
      case 'error':
        return 'var(--s-error)';
      case 'rateLimit':
        return 'var(--s-input)';
      default:
        return 'var(--ac)';
    }
  }

  let appVersion = '';
  import OrbitLogo from '../lib/assets/orbit.svg?raw';
  import ThemePicker from './ThemePicker.svelte';

  function ctxIcon(paths: string, label: string) {
    return `<span class="ctx-icon-label"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${paths}</svg>${label}</span>`;
  }

  const CTX_RENAME = ctxIcon(
    `<path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>`,
    'Rename'
  );
  const CTX_MUTE = ctxIcon(
    `<polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/>`,
    'Mute'
  );
  const CTX_UNMUTE = ctxIcon(
    `<polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><line x1="23" y1="9" x2="17" y2="15"/><line x1="17" y1="9" x2="23" y2="15"/>`,
    'Unmute'
  );
  const CTX_STOP = ctxIcon(
    `<rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>`,
    'Force Stop'
  );
  const CTX_PIN = ctxIcon(
    `<path d="M12 2L15.09 8.26L22 9.27L17 14.14L18.18 21.02L12 17.77L5.82 21.02L7 14.14L2 9.27L8.91 8.26L12 2z"/>`,
    'Pin'
  );
  const CTX_UNPIN = ctxIcon(
    `<path d="M12 2L15.09 8.26L22 9.27L17 14.14L18.18 21.02L12 17.77L5.82 21.02L7 14.14L2 9.27L8.91 8.26L12 2z"/><line x1="2" y1="2" x2="22" y2="22"/>`,
    'Unpin'
  );
  const CTX_DELETE = ctxIcon(
    `<polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/>`,
    'Delete'
  );

  function toggleExpand(parentId: number) {
    expandedParentSessions.update((set) => {
      if (set.has(parentId)) {
        return new Set([...set].filter((id) => id !== parentId));
      }
      return new Set([...set, parentId]);
    });
  }

  // Auto-expand parents when MCP child sessions appear
  $: {
    const parentIds = $sessions
      .map((s) => s.parentSessionId)
      .filter((id): id is number => id != null);
    if (parentIds.length > 0) {
      expandedParentSessions.update((set) => {
        const next = new Set([...set, ...parentIds]);
        return next.size === set.size ? set : next;
      });
    }
  }

  // Context menu state
  let ctxMenu: { x: number; y: number; sessionId: number; sessionName: string } | null = null;
  let renameTarget: { id: number; name: string } | null = null;
  let confirmDelete: { id: number; name: string } | null = null;

  function onContextMenu(e: MouseEvent, s: (typeof $sessions)[0]) {
    e.preventDefault();
    ctxMenu = {
      x: e.clientX,
      y: e.clientY,
      sessionId: s.id,
      sessionName: s.name ?? s.projectName ?? `#${s.id}`,
    };
  }

  async function handleCtxAction(action: string) {
    if (!ctxMenu) return;
    const { sessionId, sessionName } = ctxMenu;
    ctxMenu = null;

    if (action === 'rename') {
      renameTarget = { id: sessionId, name: sessionName };
    } else if (action === 'delete') {
      confirmDelete = { id: sessionId, name: sessionName };
    } else if (action === 'stop') {
      try {
        await stopSession(sessionId);
        appendSessionFeedMessage(sessionId, 'Session stopped.');
      } catch (e: unknown) {
        const message = e instanceof Error ? e.message : String(e);
        appendSessionFeedMessage(sessionId, `Failed to stop session: ${message}`, { error: true });
      }
    } else if (action === 'mute') {
      mutedSessions.toggle(String(sessionId));
    } else if (action === 'pin') {
      togglePin(String(sessionId));
    }
  }

  export let onOpenChangelog: () => void = () => {};

  let showModal = false;

  onMount(async () => {
    appVersion = await getAppVersion();
  });

  function fmtModel(model: string | null): string {
    if (!model || model === 'auto') return 'auto';
    return modelShortName(model);
  }

  function displayName(s: (typeof $sessions)[0]): string {
    return s.name ?? s.projectName ?? s.cwd?.split(/[/\\]/).pop() ?? `#${s.id}`;
  }

  let searchQuery = '';

  function sessionSearchHaystack(s: (typeof $sessions)[0]): string {
    const branch = s.branchName ?? s.gitBranch ?? '';
    const cwdLeaf = s.cwd?.split(/[/\\]/).pop() ?? '';
    return [
      displayName(s),
      s.name,
      s.projectName,
      cwdLeaf,
      s.cwd,
      branch,
      s.model,
      s.provider,
      s.status,
    ]
      .filter(Boolean)
      .join(' ')
      .toLowerCase();
  }

  function matchesSessionSearch(s: (typeof $sessions)[0], query: string): boolean {
    const q = query.trim().toLowerCase();
    if (!q) return true;
    return sessionSearchHaystack(s).includes(q);
  }

  // Derived lists for session sections
  $: rootSessions = $sessions.filter((s) => !s.parentSessionId);
  $: searchActive = searchQuery.trim().length > 0;
  $: filteredRoots = searchActive
    ? rootSessions.filter((s) => matchesSessionSearch(s, searchQuery))
    : rootSessions;
  $: pinnedList = filteredRoots.filter((s) =>
    pinnedSessions.isPinned($pinnedSessions, String(s.id))
  );
  $: recentList = filteredRoots.filter(
    (s) => !pinnedSessions.isPinned($pinnedSessions, String(s.id))
  );
</script>

{#if showModal}
  <NewSessionModal
    on:done={(e) => {
      if (e.detail?.session) upsertAndOpenSession(e.detail.session);
      showModal = false;
    }}
    on:cancel={() => (showModal = false)}
  />
{/if}

{#if confirmDelete}
  <div class="confirm-overlay" role="dialog" tabindex="-1">
    <div class="confirm-box">
      <p>Delete <strong>{confirmDelete.name}</strong>?</p>
      <div class="confirm-actions">
        <button class="confirm-btn" on:click={() => (confirmDelete = null)}>cancel</button>
        <button
          class="confirm-btn danger"
          on:click={async () => {
            const { id } = confirmDelete!;
            confirmDelete = null;
            await deleteSession(id);
            sessions.update((l) => l.filter((s) => s.id !== id));
          }}>delete</button
        >
      </div>
    </div>
  </div>
{/if}

{#if renameTarget}
  <RenameSessionModal
    sessionId={renameTarget.id}
    sessionName={renameTarget.name}
    on:done={(e) => {
      const { id, name } = e.detail;
      sessions.update((l) => updateSessionState(l, id, { name }));
      renameTarget = null;
    }}
    on:cancel={() => (renameTarget = null)}
  />
{/if}

{#if ctxMenu}
  {@const isMuted = mutedSessions.isMuted($mutedSessions, String(ctxMenu.sessionId))}
  {@const isPinned = pinnedSessions.isPinned($pinnedSessions, String(ctxMenu.sessionId))}
  <ContextMenu
    x={ctxMenu.x}
    y={ctxMenu.y}
    items={[
      { label: isPinned ? CTX_UNPIN : CTX_PIN, action: 'pin', danger: false, html: true },
      { label: CTX_RENAME, action: 'rename', danger: false, html: true },
      { label: isMuted ? CTX_UNMUTE : CTX_MUTE, action: 'mute', danger: false, html: true },
      { label: CTX_STOP, action: 'stop', danger: false, html: true },
      { label: '—', action: 'divider', divider: true },
      { label: CTX_DELETE, action: 'delete', danger: true, html: true },
    ]}
    on:select={(e) => handleCtxAction(e.detail)}
    on:close={() => (ctxMenu = null)}
  />
{/if}

<aside class="sidebar" data-testid="quiet-sidebar">
  <header class="header quiet-header">
    <div class="brand">
      <span class="brand-logo" data-testid="orbit-brand-icon">{@html OrbitLogo}</span>
      <span class="brand-name">orbit</span>
      {#if appVersion}
        <button class="brand-version" on:click={onOpenChangelog} title="What's new">
          v{appVersion}
        </button>
      {/if}
    </div>
    <div class="header-actions">
      <McpStatusBadge compact />
      <ThemePicker />
    </div>
  </header>

  <input
    type="search"
    class="quiet-search"
    bind:value={searchQuery}
    placeholder="Search sessions…"
    aria-label="Search sessions"
    data-testid="session-search-input"
    autocomplete="off"
    spellcheck="false"
  />

  <button
    type="button"
    class="new-session-btn"
    aria-label="New session"
    data-testid="new-session-button"
    on:click={() => (showModal = true)}>+ New session</button
  >

  {#if pinnedList.length > 0}
    <section class="session-section" aria-label="Pinned sessions">
      <div class="section-label">Pinned</div>
      <div class="session-list">
        {#each pinnedList as s (s.id)}
          <SessionListItem
            session={s}
            pinned
            expandedParents={$expandedParentSessions}
            onToggleExpand={toggleExpand}
            onContextMenu={onContextMenu}
            {displayName}
            {fmtModel}
            {attentionColor}
          />
        {/each}
      </div>
    </section>
  {/if}

  <section class="session-section" aria-label="Recent sessions">
    <div class="section-label">Recent sessions</div>
    <div class="session-list">
      {#if rootSessions.length === 0}
        <div class="empty quiet-empty">No sessions yet</div>
      {:else if searchActive && filteredRoots.length === 0}
        <div class="empty quiet-empty">No matching sessions</div>
      {:else if recentList.length === 0}
        <div class="empty quiet-empty">
          {searchActive ? 'No matching sessions' : 'All sessions pinned'}
        </div>
      {:else}
        {#each recentList as s (s.id)}
          <SessionListItem
            session={s}
            expandedParents={$expandedParentSessions}
            onToggleExpand={toggleExpand}
            onContextMenu={onContextMenu}
            {displayName}
            {fmtModel}
            {attentionColor}
          />
        {/each}
      {/if}
    </div>
  </section>

  <footer class="footer quiet-footer">
    <SidebarFooterHints />
    <DesktopNotifyToggle />
  </footer>
</aside>

<style>
  .sidebar {
    width: 282px;
    flex: 0 0 282px;
    background: var(--bg-sidebar, #0c0d0d);
    border-right: 1px solid var(--bd);
    padding: 18px 14px;
    gap: 16px;
    display: flex;
    flex-direction: column;
  }

  .quiet-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: auto;
    padding: 0 6px 10px;
    border-bottom: 0;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .brand-logo {
    width: 28px;
    height: 28px;
    color: var(--ac);
    filter: drop-shadow(0 0 14px color-mix(in srgb, var(--ac), transparent 48%));
  }
  .brand-logo :global(svg) {
    width: 26px;
    height: 26px;
  }
  .brand-name {
    font-size: var(--md);
    font-weight: 600;
    color: var(--t0);
    letter-spacing: 0.12em;
    text-transform: lowercase;
  }
  .brand-version {
    font-size: 10px;
    color: var(--t2);
    letter-spacing: 0.04em;
    margin-top: 1px;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    font-family: var(--mono);
    text-decoration: underline;
    text-decoration-color: var(--t3);
    text-underline-offset: 2px;
    transition:
      color 0.15s,
      text-decoration-color 0.15s;
  }
  .brand-version:hover {
    color: var(--t0);
    text-decoration-color: var(--t1);
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }
  .new-session-btn {
    display: block;
    width: 100%;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    font-size: 11px;
    font-weight: 500;
    background: color-mix(in srgb, var(--t0), transparent 96%);
    color: var(--t1);
    cursor: pointer;
    text-align: center;
    font-family: var(--mono);
    border: none;
    transition: all 0.15s;
  }
  .new-session-btn:hover {
    background: color-mix(in srgb, var(--t0), transparent 93%);
    color: var(--t0);
  }
  .quiet-search {
    width: 100%;
    height: 34px;
    box-sizing: border-box;
    padding: 0 12px;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--t0), transparent 95%);
    color: var(--t0);
    font-size: 12px;
    font-family: inherit;
    outline: none;
  }
  .quiet-search::placeholder {
    color: var(--t3);
  }
  .quiet-search:focus {
    border-color: color-mix(in srgb, var(--t0), transparent 88%);
    background: color-mix(in srgb, var(--t0), transparent 93%);
  }
  .session-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .session-section:last-child {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }
  .section-label {
    padding: 0 8px;
    color: var(--t3);
    font-family: var(--mono);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .session-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .empty {
    padding: var(--sp-8) var(--sp-6);
    font-size: var(--sm);
    color: var(--t3);
  }

  .confirm-overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .confirm-box {
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-md);
    padding: var(--sp-8) var(--sp-9);
    min-width: 200px;
  }
  .confirm-box p {
    font-size: var(--sm);
    color: var(--t0);
    margin-bottom: var(--sp-6);
  }
  .confirm-box strong {
    color: var(--t0);
  }
  .confirm-actions {
    display: flex;
    gap: var(--sp-4);
    justify-content: flex-end;
  }
  .confirm-btn {
    background: none;
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    color: var(--t1);
    font-size: var(--xs);
    padding: var(--sp-2) var(--sp-6);
    cursor: pointer;
    font-family: var(--mono);
  }
  .confirm-btn:hover {
    border-color: var(--bd2);
    color: var(--t0);
  }
  .confirm-btn.danger {
    color: var(--s-error);
  }
  .confirm-btn.danger:hover {
    border-color: var(--s-error);
  }

  .quiet-footer {
    margin-top: auto;
    padding: 10px 8px 0;
    border-top: 1px solid var(--bd);
    color: var(--t3);
    font-family: var(--mono);
    font-size: 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.3;
    }
  }
</style>
