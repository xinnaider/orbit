<script lang="ts">
  import type { AgentState } from '../lib/types';
  import AgentCard from './AgentCard.svelte';

  export let agents: AgentState[];
  export let selectedId: string | null;
  export let onSelect: (id: string) => void;

  $: totalTokens = agents.reduce((sum, a) => sum + a.tokens.input + a.tokens.output, 0);
  $: awaitingCount = agents.filter(a => a.status === 'input').length;
</script>

<aside class="sidebar">
  <div class="sidebar-header">
    <span class="title">Agents</span>
    <span class="badge">{agents.length}</span>
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
    font-size: 12px;
  }
  .badge {
    background: var(--green-dim);
    color: var(--green);
    padding: 1px 6px;
    border-radius: 8px;
    font-size: 9px;
  }
  .sidebar-content { flex: 1; overflow-y: auto; }
  .sidebar-footer {
    padding: 8px 12px;
    border-top: 1px solid var(--border);
    font-size: 9px;
    color: var(--text-dim);
  }
  .empty {
    padding: 20px 12px;
    color: var(--text-muted);
    font-size: 11px;
    text-align: center;
  }
</style>
