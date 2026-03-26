<script lang="ts">
  import type { SubagentInfo } from '../lib/types';

  export let subagents: SubagentInfo[];

  $: running = subagents.filter(a => a.status === 'running');
  $: done = subagents.filter(a => a.status === 'done');
</script>

<div class="subagents">
  {#if subagents.length === 0}
    <p class="empty">No subagents spawned</p>
  {:else}
    {#if running.length > 0}
      <div class="section">
        <div class="section-header">
          <span class="dot running"></span> Running ({running.length})
        </div>
        {#each running as agent}
          <div class="agent-row running-row">
            <div class="agent-type">{agent.agentType}</div>
            {#if agent.description}
              <div class="agent-desc">{agent.description}</div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    {#if done.length > 0}
      <div class="section">
        <div class="section-header">
          <span class="dot done"></span> Completed ({done.length})
        </div>
        {#each done as agent}
          <div class="agent-row">
            <div class="agent-type">{agent.agentType} <span class="check">✓</span></div>
            {#if agent.description}
              <div class="agent-desc">{agent.description}</div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .subagents { padding: 10px; }
  .empty { color: var(--text-dim); font-size: 13px; text-align: center; padding: 20px; }
  .section { margin-bottom: 12px; }
  .section-header {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 6px;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    display: inline-block;
  }
  .dot.running { background: var(--amber); }
  .dot.done { background: var(--green); }
  .agent-row {
    padding: 6px 8px;
    border-radius: 6px;
    margin-bottom: 3px;
    background: var(--bg-overlay);
  }
  .agent-row.running-row {
    border-left: 2px solid var(--amber);
    background: var(--bg-thinking);
  }
  .agent-type {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
  }
  .check { color: var(--green); }
  .agent-desc {
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 2px;
    line-height: 1.4;
  }
</style>
