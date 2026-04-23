<script lang="ts">
  import { sessions, updateSessionState } from '../lib/stores/sessions';
  import { workspace, assignSession, splitPane } from '../lib/stores/workspace';
  import { get } from 'svelte/store';
  import { statusColor, statusLabel, isPulsing } from '../lib/status';
  import NewSessionModal from './NewSessionModal.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import RenameSessionModal from './RenameSessionModal.svelte';
  import { deleteSession, stopSession, getAppVersion } from '../lib/tauri';
  import { mutedSessions, sessionEffort } from '../lib/stores/ui';
  import { sidebarVisible } from '../lib/stores/preferences';
  import { modelShortName } from '../lib/status';
  import { onMount } from 'svelte';
  import { clearAttention } from '../lib/tauri/attention';
  import HttpApiSettingsModal from './HttpApiSettingsModal.svelte';

  let showHttpSettings = false;

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
  import { formatTokens } from '../lib/cost';
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
  const CTX_DELETE = ctxIcon(
    `<polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/>`,
    'Delete'
  );

  let expandedParents: Set<number> = new Set();

  function getChildren(list: typeof $sessions, parentId: number) {
    return list.filter((s) => s.parentSessionId === parentId);
  }

  function selectOrToggle(s: (typeof $sessions)[0], hasChildren: boolean) {
    const ws = get(workspace);
    if (ws.focusedPaneId) assignSession(ws.focusedPaneId, s.id);
    if (s.attention?.requiresAttention) clearAttention(s.id);
    if (hasChildren) {
      expandedParents = new Set(
        expandedParents.has(s.id)
          ? [...expandedParents].filter((id) => id !== s.id)
          : [...expandedParents, s.id]
      );
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
      } catch (_e) {
        /* no-op */
      }
    } else if (action === 'mute') {
      mutedSessions.toggle(String(sessionId));
    }
  }

  export let onOpenChangelog: () => void = () => {};

  let showModal = false;

  onMount(async () => {
    appVersion = await getAppVersion();
  });

  function fmtTokens(s: (typeof $sessions)[0]): string {
    if (!s.tokens) return '—';
    const total = s.tokens.input + s.tokens.output;
    return formatTokens(total);
  }

  function fmtModel(model: string | null): string {
    if (!model || model === 'auto') return 'auto';
    return modelShortName(model);
  }

  function displayName(s: (typeof $sessions)[0]): string {
    return s.name ?? s.projectName ?? s.cwd?.split(/[/\\]/).pop() ?? `#${s.id}`;
  }
</script>

{#if showModal}
  <NewSessionModal on:done={() => (showModal = false)} on:cancel={() => (showModal = false)} />
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
  <ContextMenu
    x={ctxMenu.x}
    y={ctxMenu.y}
    items={[
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

<aside class="sidebar">
  <header class="header">
    <div class="brand">
      <span class="brand-logo">{@html OrbitLogo}</span>
      <span class="brand-name">orbit</span>
      {#if appVersion}
        <button class="brand-version" on:click={onOpenChangelog} title="What's new"
          >v{appVersion}</button
        >
      {/if}
    </div>
    <div class="header-actions">
      <ThemePicker />
      <button class="new-btn" on:click={() => (showModal = true)} title="New session">+</button>
    </div>
  </header>

  <div class="list">
    {#if $sessions.length === 0}
      <p class="empty">no sessions</p>
    {:else}
      {#each $sessions.filter((s) => !s.parentSessionId) as s (s.id)}
        {@const active = Object.values($workspace.panes).some((p) => p.sessionId === s.id)}
        {@const color = statusColor(s.status)}
        {@const pulsing = isPulsing(s.status)}
        {@const children = getChildren($sessions, s.id)}
        {@const expanded = expandedParents.has(s.id)}
        {@const childActive = children.some((c) =>
          Object.values($workspace.panes).some((p) => p.sessionId === c.id)
        )}
        <button
          class="item"
          class:active
          class:child-active={childActive && !active}
          class:has-children={children.length > 0}
          class:expanded
          draggable="true"
          on:dragstart={(e) => {
            e.dataTransfer?.setData('text/plain', JSON.stringify({ sessionId: s.id }));
          }}
          on:click={() => selectOrToggle(s, children.length > 0)}
          on:dblclick={() => {
            const ws = get(workspace);
            if (ws.focusedPaneId) splitPane(ws.focusedPaneId, 'horizontal', s.id);
          }}
          on:contextmenu={(e) => onContextMenu(e, s)}
        >
          <div class="item-top">
            <span class="dot" style="color:{color}" class:pulse={pulsing}>●</span>
            <span class="name">{displayName(s)}</span>
            {#if mutedSessions.isMuted($mutedSessions, String(s.id))}
              <span class="muted-icon" title="Muted">
                <svg
                  width="9"
                  height="9"
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
              </span>
            {/if}
            <span class="status" style="color:{color}">{statusLabel(s.status)}</span>
          </div>
          <div class="item-meta">
            <span title={s.model ?? ''}>{fmtModel(s.model)}</span>
            {#if s.provider === 'claude-code'}
              <span class="sep">·</span>
              <span>{sessionEffort.get($sessionEffort, String(s.id))}</span>
            {/if}
            <span class="sep">·</span>
            <span>{fmtTokens(s)}</span>
            {#if s.pendingApproval}
              <span class="approval-dot" title={s.pendingApproval}>⚑</span>
            {/if}
            {#if s.attention?.requiresAttention}
              <span
                class="attention-dot"
                style="color:{attentionColor(s.attention.reason)}"
                title={s.attention.reason ?? 'needs attention'}>●</span
              >
            {/if}
          </div>
        </button>
        {#if expanded && children.length > 0}
          <div class="cards">
            {#each children as c (c.id)}
              {@const cActive = Object.values($workspace.panes).some((p) => p.sessionId === c.id)}
              {@const cColor = statusColor(c.status)}
              {@const cPulsing = isPulsing(c.status)}
              {@const pct = c.contextPercent ?? 0}
              <button
                class="card"
                class:active={cActive}
                style="--card-color:{cColor}"
                draggable="true"
                on:dragstart={(e) => {
                  e.dataTransfer?.setData('text/plain', JSON.stringify({ sessionId: c.id }));
                }}
                on:click={() => {
                  const ws = get(workspace);
                  if (ws.focusedPaneId) assignSession(ws.focusedPaneId, c.id);
                  if (c.attention?.requiresAttention) clearAttention(c.id);
                }}
                on:contextmenu={(e) => onContextMenu(e, c)}
              >
                <div class="card-top">
                  <span class="card-dot" class:pulse={cPulsing}>●</span>
                  <span class="card-name">{displayName(c)}</span>
                  <span class="card-status" style="color:{cColor}">{statusLabel(c.status)}</span>
                </div>
                <div class="card-meta">
                  <span>{fmtModel(c.model)}</span>
                  {#if c.provider === 'claude-code'}
                    <span class="sep">·</span>
                    <span>{sessionEffort.get($sessionEffort, String(c.id))}</span>
                  {/if}
                  <span class="sep">·</span>
                  <span>{fmtTokens(c)}</span>
                </div>
                <div class="card-bar">
                  <div class="card-fill" style="width:{Math.min(pct, 100)}%"></div>
                </div>
              </button>
            {/each}
          </div>
        {/if}
      {/each}
    {/if}
  </div>

  <footer class="footer">
    <span>{$sessions.length} session{$sessions.length !== 1 ? 's' : ''}</span>
    <div class="footer-actions">
      <button
        class="footer-btn"
        on:click={() => (showHttpSettings = true)}
        title="HTTP API settings"
      >
        API
      </button>
      <button class="collapse-btn" on:click={() => sidebarVisible.set(false)} title="Hide sidebar"
        >‹</button
      >
    </div>
  </footer>

  {#if showHttpSettings}
    <HttpApiSettingsModal on:close={() => (showHttpSettings = false)} />
  {/if}
</aside>

<style>
  .sidebar {
    width: 220px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--bd);
    background: var(--bg1);
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-5) var(--sp-6) 9px;
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }
  .brand-logo {
    display: flex;
    align-items: center;
    color: var(--ac);
    line-height: 0;
  }
  .brand-logo :global(svg) {
    width: 16px;
    height: 16px;
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
  .new-btn {
    background: none;
    border: 1px solid var(--bd1);
    color: var(--t1);
    width: 20px;
    height: 20px;
    border-radius: var(--radius-sm);
    font-size: 14px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    transition:
      border-color 0.15s,
      color 0.15s;
  }
  .new-btn:hover {
    border-color: var(--ac);
    color: var(--ac);
  }

  .list {
    flex: 1;
    overflow-y: auto;
  }

  .empty {
    padding: var(--sp-8) var(--sp-6);
    font-size: var(--sm);
    color: var(--t3);
  }

  .item {
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    border-bottom: 1px solid var(--bd);
    padding: var(--sp-4) var(--sp-6);
    cursor: pointer;
    transition: background 0.1s;
    position: relative;
  }
  .item:hover {
    background: var(--bg2);
  }
  .item.active {
    background: var(--ac-d2);
    border-left: 2px solid var(--ac);
    padding-left: var(--sp-5);
  }
  .item.child-active {
    background: color-mix(in srgb, var(--ac-d2), transparent 50%);
    border-left: 2px solid var(--ac);
    padding-left: var(--sp-5);
  }
  .item.has-children .item-meta::after {
    content: '▸';
    font-size: 9px;
    color: var(--t3);
    margin-left: auto;
    transition: transform 0.15s;
    display: inline-block;
  }
  .item.has-children.expanded .item-meta::after {
    transform: rotate(90deg);
  }

  .cards {
    padding: var(--sp-3) var(--sp-5);
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    border-bottom: 1px solid var(--bd);
    background: var(--bg0);
  }
  .card {
    background: var(--bg1);
    border: 1px solid var(--bd);
    border-radius: var(--radius-sm);
    padding: var(--sp-3) var(--sp-4);
    cursor: pointer;
    text-align: left;
    transition:
      background 0.1s,
      border-color 0.15s,
      box-shadow 0.15s;
  }
  .card:hover {
    background: var(--bg2);
    border-color: var(--bd1);
  }
  .card.active {
    background: var(--ac-d2);
    border-color: var(--ac);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--ac), transparent 70%);
  }
  .card-top {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .card-dot {
    font-size: 6px;
    color: var(--card-color);
    flex-shrink: 0;
    line-height: 1;
  }
  .card-dot.pulse {
    animation: pulse 2s ease-in-out infinite;
  }
  .card-name {
    font-size: var(--xs);
    color: var(--t0);
    font-weight: 500;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card-status {
    font-size: 9px;
    letter-spacing: 0.04em;
    flex-shrink: 0;
  }
  .card-meta {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    font-size: 10px;
    color: var(--t2);
    margin-top: 2px;
  }
  .card-bar {
    margin-top: var(--sp-2);
    height: 2px;
    background: var(--bg0);
    border-radius: 1px;
    overflow: hidden;
  }
  .card-fill {
    height: 100%;
    background: var(--card-color);
    border-radius: 1px;
    transition: width 0.3s ease;
  }

  .item-top {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    margin-bottom: var(--sp-2);
  }
  .dot {
    font-size: 8px;
    flex-shrink: 0;
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
      opacity: 0.3;
    }
  }

  .name {
    font-size: var(--md);
    color: var(--t0);
    font-weight: 500;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status {
    font-size: var(--xs);
    letter-spacing: 0.04em;
    flex-shrink: 0;
  }

  .item-meta {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    font-size: var(--xs);
    color: var(--t2);
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

  .approval-dot {
    color: var(--s-input);
    margin-left: var(--sp-2);
  }

  .attention-dot {
    font-size: 8px;
    margin-left: var(--sp-2);
    animation: pulse 2s ease-in-out infinite;
  }

  .muted-icon {
    display: flex;
    align-items: center;
    color: var(--t3);
    flex-shrink: 0;
  }

  .footer {
    padding: var(--sp-3) var(--sp-6);
    border-top: 1px solid var(--bd);
    font-size: var(--xs);
    color: var(--t2);
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
  }
  .footer-actions {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }
  .footer-btn {
    background: none;
    border: 1px solid var(--bd);
    border-radius: var(--radius-sm);
    color: var(--t2);
    font-size: 9px;
    font-family: var(--mono);
    letter-spacing: 0.05em;
    padding: 2px 6px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .footer-btn:hover {
    border-color: var(--ac);
    color: var(--ac);
  }
  .collapse-btn {
    background: none;
    border: none;
    color: var(--t2);
    font-size: 14px;
    cursor: pointer;
    padding: 0 var(--sp-1);
    line-height: 1;
    transition: color 0.15s;
  }
  .collapse-btn:hover {
    color: var(--t0);
  }
</style>
