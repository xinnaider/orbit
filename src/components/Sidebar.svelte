<script lang="ts">
  import type { AgentState } from '../lib/types';
  import AgentCard from './AgentCard.svelte';
  import { theme } from '../lib/stores/preferences';
  import CreateSessionDialog from './CreateSessionDialog.svelte';

  export let agents: AgentState[];
  export let selectedId: string | null;
  export let onSelect: (id: string) => void;

  let showCreateDialog = false;

  $: totalTokens = agents.reduce((sum, a) => sum + a.tokens.input + a.tokens.output, 0);
  $: awaitingCount = agents.filter(a => a.status === 'input').length;
</script>

{#if showCreateDialog}
  <CreateSessionDialog
    on:created={() => showCreateDialog = false}
    on:cancel={() => showCreateDialog = false}
  />
{/if}

<aside class="sidebar">
  <div class="sidebar-header">
    <span class="title">Agents</span>
    <div class="header-right">
      <button class="new-session-btn" on:click={() => showCreateDialog = true} title="New Session">+ New Session</button>
      <button class="theme-toggle" onclick={() => theme.toggle()} title="Toggle theme">
        {$theme === 'dark' ? '☀' : '☾'}
      </button>
      <span class="badge">{agents.length}</span>
    </div>
  </div>
  <div class="sidebar-content">
    {#if agents.length === 0}
      <p class="empty">No agents detected</p>
    {:else}
      {#each agents as agent (agent.sessionId)}
        <AgentCard
          {agent}
          selected={agent.sessionId === selectedId}
          onClick={() => onSelect(agent.sessionId)}
        />
      {/each}
    {/if}
  </div>
  <div class="sidebar-footer">
    {Math.round(totalTokens / 1000)}K tokens total
    {#if awaitingCount > 0}
      • {awaitingCount} awaiting
    {/if}
  </div>
</aside>

<style>
  .sidebar {
    width: 260px;
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }
  .sidebar-header {
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-weight: 600;
    font-size: 14px;
  }
  .header-right { display: flex; align-items: center; gap: 8px; }
  .new-session-btn {
    background: #3b82f6;
    border: none;
    border-radius: 4px;
    color: white;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    padding: 3px 8px;
    line-height: 1.4;
  }
  .new-session-btn:hover { background: #2563eb; }
  .theme-toggle {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
    padding: 2px;
    line-height: 1;
  }
  .theme-toggle:hover { color: var(--text-primary); }
  .badge {
    background: var(--green-dim);
    color: var(--green);
    padding: 1px 6px;
    border-radius: 8px;
    font-size: 11px;
  }
  .sidebar-content { flex: 1; overflow-y: auto; }
  .sidebar-footer {
    padding: 8px 12px;
    border-top: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-dim);
  }
  .empty {
    padding: 20px 12px;
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
  }
</style>
