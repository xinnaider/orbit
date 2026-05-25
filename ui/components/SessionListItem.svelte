<script lang="ts">
  import Self from './SessionListItem.svelte';
  import type { Session } from '../lib/stores/sessions';
  import { sessions } from '../lib/stores/sessions';
  import { workspace } from '../lib/stores/workspace';
  import { statusColor } from '../lib/status';
  import { modelShortName } from '../lib/status';
  import { assignSession } from '../lib/stores/workspace';
  import { get } from 'svelte/store';
  import { clearAttention } from '../lib/tauri/attention';

  export let session: Session;
  export let depth = 0;
  export let pinned = false;
  export let expandedParents: Set<number>;
  export let onToggleExpand: (id: number) => void;
  export let onContextMenu: (e: MouseEvent, s: Session) => void;
  export let displayName: (s: Session) => string;
  export let fmtModel: (model: string | null) => string;
  export let attentionColor: (reason: string | null) => string;

  function getChildren(list: Session[], parentId: number) {
    return list.filter((s) => s.parentSessionId === parentId);
  }

  $: children = getChildren($sessions, session.id);
  $: hasChildren = children.length > 0;
  $: expanded = expandedParents.has(session.id);
  $: branchLabel = session.branchName ?? session.gitBranch ?? null;
  $: isMcpChild = (session.depth ?? 0) > 0 || session.parentSessionId != null;

  function openSession(s: Session, expandIfParent: boolean) {
    const ws = get(workspace);
    if (ws.focusedPaneId) assignSession(ws.focusedPaneId, s.id);
    if (s.attention?.requiresAttention) clearAttention(s.id);
    if (expandIfParent && getChildren($sessions, s.id).length > 0) {
      onToggleExpand(s.id);
    }
  }
</script>

<button
  type="button"
  class="session-item quiet-session"
  class:pinned
  class:session-child={depth > 0}
  class:active={$workspace.panes[$workspace.focusedPaneId ?? '']?.tabs.some(
    (tab) => tab.target.kind === 'agent' && tab.target.sessionId === session.id
  )}
  style="--tree-depth: {depth}"
  draggable="true"
  data-testid="session-item"
  data-session-depth={depth}
  on:dragstart={(e) => {
    e.dataTransfer?.setData('text/plain', JSON.stringify({ sessionId: session.id }));
  }}
  on:click={() => openSession(session, depth === 0)}
  on:contextmenu={(e) => onContextMenu(e, session)}
>
  <span class="session-topline">
    <span class="session-title-row">
      {#if hasChildren}
        <span class="tree-chevron" aria-hidden="true">{expanded ? '▾' : '▸'}</span>
      {:else if depth > 0}
        <span class="tree-chevron spacer" aria-hidden="true"></span>
      {/if}
      <span class="session-title">{displayName(session)}</span>
    </span>
    <span
      class="status-dot"
      style="background:{attentionColor(session.attention?.reason ?? null) ||
        statusColor(session.status)}"
    ></span>
  </span>
  <span class="session-subline">
    <span>{fmtModel(session.model)}</span>
    {#if isMcpChild}<span class="mcp-badge" title="MCP child session">mcp</span>{/if}
    {#if branchLabel}<span>{branchLabel}</span>{/if}
    {#if session.gitDirty}<span class="git-dirty" title="Uncommitted changes">●</span>{/if}
    {#if (session.contextPercent ?? 0) > 0}<span>{Math.round(session.contextPercent ?? 0)}% ctx</span
      >{/if}
  </span>
</button>

{#if hasChildren && expanded}
  {#each children as child (child.id)}
    <Self
      session={child}
      depth={depth + 1}
      {pinned}
      {expandedParents}
      {onToggleExpand}
      {onContextMenu}
      {displayName}
      {fmtModel}
      {attentionColor}
    />
  {/each}
{/if}

<style>
  .quiet-session {
    width: 100%;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    padding: 10px 11px;
    background: transparent;
    color: var(--t1);
    text-align: left;
    cursor: pointer;
  }
  .quiet-session:hover {
    background: color-mix(in srgb, var(--t0), transparent 97%);
  }
  .quiet-session.active {
    color: var(--t0);
    background: color-mix(in srgb, var(--t0), transparent 94%);
    border-color: color-mix(in srgb, var(--t0), transparent 92%);
  }
  .session-topline {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    font-size: 12px;
  }
  .session-title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .session-subline {
    margin-top: 4px;
    display: flex;
    gap: 7px;
    color: var(--t3);
    font-family: var(--mono);
    font-size: 10px;
    white-space: nowrap;
    overflow: hidden;
  }
  .git-dirty {
    color: var(--s-input);
    font-size: 9px;
    line-height: 1;
  }
  .status-dot {
    width: 8px;
    height: 8px;
    flex-shrink: 0;
    border-radius: 50%;
    box-shadow:
      0 0 0 3px color-mix(in srgb, currentColor, transparent 84%),
      0 0 14px currentColor;
  }
  .session-child {
    margin-left: calc(8px + var(--tree-depth, 0) * 10px);
    padding-left: 6px;
    border-left: 1px solid color-mix(in srgb, var(--t0), transparent 90%);
  }
  .session-title-row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: 1;
  }
  .tree-chevron {
    flex-shrink: 0;
    width: 10px;
    font-size: 9px;
    color: var(--t3);
    font-family: var(--mono);
  }
  .tree-chevron.spacer {
    visibility: hidden;
  }
  .mcp-badge {
    color: var(--ac);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
</style>
