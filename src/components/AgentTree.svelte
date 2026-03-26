<script lang="ts">
  import type { SubagentInfo, AgentStatus } from '../lib/types';

  export let status: AgentStatus;
  export let subagents: SubagentInfo[];
</script>

{#if subagents.length > 0}
<div class="tree mono">
  <span class="main">● main <span class="status-{status}">({status})</span></span>
  {#each subagents as agent, i}
    <span class="sep"> → </span>
    <span>{i < subagents.length - 1 ? '├' : '└'} {agent.agentType}
      <span class="agent-status" class:done={agent.status === 'done'} class:running={agent.status === 'running'}>
        {agent.status === 'done' ? '✓' : '●'}
      </span>
    </span>
  {/each}
</div>
{/if}

<style>
  .tree {
    padding: 5px 14px;
    border-bottom: 1px solid rgba(30, 41, 59, 0.07);
    background: rgba(255, 255, 255, 0.01);
    font-size: 10px;
    color: var(--text-secondary);
  }
  .main { color: var(--text-primary); }
  .sep { color: var(--text-dim); }
  .status-working { color: var(--green); }
  .status-input { color: var(--amber); }
  .status-idle { color: var(--text-muted); }
  .agent-status.done { color: var(--green); }
  .agent-status.running { color: var(--amber); }
</style>
