<script lang="ts">
  import type { SubagentInfo } from '../lib/types';

  export let subagents: SubagentInfo[];

  $: running = subagents.filter(a => a.status === 'running');
  $: doneCount = subagents.filter(a => a.status === 'done').length;
</script>

{#if running.length > 0 || doneCount > 0}
<div class="tree mono">
  {#if running.length > 0}
    {#each running as agent}
      <span class="running-indicator">▸ {agent.agentType}</span>
      {#if agent.description}
        <span class="desc">{agent.description}</span>
      {/if}
    {/each}
  {/if}
  {#if doneCount > 0}
    <span class="done-count">{doneCount} subagent{doneCount > 1 ? 's' : ''} completed</span>
  {/if}
</div>
{/if}

<style>
  .tree {
    padding: 5px 14px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-subtle);
    font-size: 12px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .running-indicator { color: var(--amber); font-weight: 500; }
  .desc { color: var(--text-dim); font-size: 11px; }
  .done-count { color: var(--text-dim); font-size: 11px; }
</style>
