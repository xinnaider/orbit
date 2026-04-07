<script lang="ts">
  import { sessions, updateSessionState } from '../lib/stores/sessions';
  import { splitLayout, assignSession, clearSession } from '../lib/stores/layout';
  import { statusColor, statusLabel, isPulsing } from '../lib/status';
  import NewSessionModal from './NewSessionModal.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import { renameSession, deleteSession, stopSession, getAppVersion } from '../lib/tauri';
  import { onMount } from 'svelte';

  let appVersion = '';
  import { estimateCost, formatCost, formatTokens } from '../lib/cost';
  import OrbitLogo from '../lib/assets/orbit.svg?raw';

  // Context menu state
  let ctxMenu: { x: number; y: number; sessionId: number; sessionName: string } | null = null;
  let renaming: { id: number; value: string } | null = null;
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
      renaming = { id: sessionId, value: sessionName };
    } else if (action === 'delete') {
      confirmDelete = { id: sessionId, name: sessionName };
    } else if (action === 'stop') {
      try {
        await stopSession(sessionId);
      } catch (_e) {
        /* no-op */
      }
    }
  }

  async function submitRename() {
    if (!renaming) return;
    const { id, value } = renaming;
    renaming = null;
    if (!value.trim()) return;
    await renameSession(id, value.trim());
    sessions.update((l) => updateSessionState(l, id, { name: value.trim() }));
  }

  let showModal = false;

  // Svelte action: auto-focus and select when mounted
  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    node.select();
    return { destroy() {} };
  }

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
    if (model.includes('opus')) return 'opus';
    if (model.includes('sonnet')) return 'sonnet';
    if (model.includes('haiku')) return 'haiku';
    return model.split('-')[2] ?? model;
  }

  function displayName(s: (typeof $sessions)[0]): string {
    return s.name ?? s.projectName ?? s.cwd?.split(/[\\/]/).pop() ?? `#${s.id}`;
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
            clearSession(id);
          }}>delete</button
        >
      </div>
    </div>
  </div>
{/if}

{#if ctxMenu}
  <ContextMenu
    x={ctxMenu.x}
    y={ctxMenu.y}
    items={[
      { label: 'Rename', action: 'rename', danger: false },
      { label: 'Stop', action: 'stop', danger: false },
      { label: '—', action: 'divider', divider: true },
      { label: 'Delete', action: 'delete', danger: true },
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
        <span class="brand-version">v{appVersion}</span>
      {/if}
    </div>
    <button class="new-btn" on:click={() => (showModal = true)} title="New session">+</button>
  </header>

  <div class="list">
    {#if $sessions.length === 0}
      <p class="empty">no sessions</p>
    {:else}
      {#each $sessions as s (s.id)}
        {@const active = Object.values($splitLayout.panes).includes(s.id)}
        {@const color = statusColor(s.status)}
        {@const pulsing = isPulsing(s.status)}
        <button
          class="item"
          class:active
          draggable="true"
          on:click={() => assignSession($splitLayout.focused, s.id)}
          on:contextmenu={(e) => onContextMenu(e, s)}
          on:dragstart={(e) => {
            if (e.dataTransfer) {
              e.dataTransfer.setData('text/plain', String(s.id));
              e.dataTransfer.effectAllowed = 'move';
            }
          }}
        >
          <div class="item-top">
            <span class="dot" style="color:{color}" class:pulse={pulsing}>●</span>
            {#if renaming?.id === s.id}
              <input
                class="rename-input"
                bind:value={renaming.value}
                on:keydown={(e) => {
                  if (e.key === 'Enter') submitRename();
                  if (e.key === 'Escape') renaming = null;
                }}
                on:blur={submitRename}
                use:focusOnMount
              />
            {:else}
              <span class="name">{displayName(s)}</span>
            {/if}
            <span class="status" style="color:{color}">{statusLabel(s.status)}</span>
          </div>
          <div class="item-meta">
            <span>{fmtModel(s.model)}</span>
            <span class="sep">·</span>
            <span>{fmtTokens(s)}</span>
            {#if s.pendingApproval}
              <span class="approval-dot" title={s.pendingApproval}>⚑</span>
            {/if}
          </div>
        </button>
      {/each}
    {/if}
  </div>

  <footer class="footer">
    <span>{$sessions.length} session{$sessions.length !== 1 ? 's' : ''}</span>
    <span class="sep">·</span>
    <span>
      {formatCost(
        $sessions.reduce((sum, s) => {
          if (!s.tokens) return sum;
          return sum + estimateCost(s.tokens, s.model);
        }, 0)
      )} total
    </span>
  </footer>
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
    padding: 10px 12px 9px;
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 7px;
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
    color: var(--t3);
    letter-spacing: 0.04em;
    margin-top: 1px;
  }
  .new-btn {
    background: none;
    border: 1px solid var(--bd1);
    color: var(--t1);
    width: 20px;
    height: 20px;
    border-radius: 3px;
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
    padding: 16px 12px;
    font-size: var(--sm);
    color: var(--t3);
  }

  .item {
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    border-bottom: 1px solid var(--bd);
    padding: 8px 12px;
    cursor: pointer;
    transition: background 0.1s;
  }
  .item:hover {
    background: var(--bg2);
  }
  .item.active {
    background: var(--ac-d2);
    border-left: 2px solid var(--ac);
    padding-left: 10px;
  }

  .item-top {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 3px;
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
    gap: 4px;
    font-size: var(--xs);
    color: var(--t2);
    padding-left: 14px;
  }
  .sep {
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
    border-radius: 4px;
    padding: 16px 20px;
    min-width: 200px;
  }
  .confirm-box p {
    font-size: var(--sm);
    color: var(--t0);
    margin-bottom: 12px;
  }
  .confirm-box strong {
    color: var(--t0);
  }
  .confirm-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .confirm-btn {
    background: none;
    border: 1px solid var(--bd1);
    border-radius: 3px;
    color: var(--t1);
    font-size: var(--xs);
    padding: 4px 12px;
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

  .rename-input {
    flex: 1;
    min-width: 0;
    background: var(--bg3);
    border: 1px solid var(--ac);
    border-radius: 2px;
    color: var(--t0);
    font-size: var(--md);
    font-family: var(--mono);
    padding: 1px 5px;
    outline: none;
  }
  .approval-dot {
    color: var(--s-input);
    margin-left: 4px;
  }

  .footer {
    padding: 7px 12px;
    border-top: 1px solid var(--bd);
    font-size: var(--xs);
    color: var(--t2);
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }
</style>
