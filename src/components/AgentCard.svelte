<script lang="ts">
  import type { AgentState } from '../lib/types';
  import MiniLog from './MiniLog.svelte';

  export let agent: AgentState;
  export let selected: boolean = false;
  export let onClick: () => void = () => {};
</script>

<div
  class="card"
  class:selected
  class:input={agent.status === 'input'}
  onclick={onClick}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === 'Enter' && onClick()}
>
  <div class="header">
    <span class="name">{agent.project}</span>
    <span class="status {agent.status}">{agent.status.toUpperCase()}</span>
  </div>
  <div class="meta">
    {agent.gitBranch ?? 'no branch'} • {agent.modelDisplay} • {Math.round((agent.tokens.input + agent.tokens.output) / 1000)}K
  </div>
  <div class="context-bar">
    <div
      class="context-fill"
      style="width: {Math.min(agent.contextPercent, 100)}%"
      class:warn={agent.contextPercent > 70}
      class:danger={agent.contextPercent > 90}
    ></div>
  </div>
  <MiniLog entries={agent.miniLog} pendingApproval={agent.pendingApproval} />
</div>

<style>
  .card {
    padding: 10px 12px;
    border-left: 2px solid transparent;
    cursor: pointer;
    border-bottom: 1px solid rgba(30, 41, 59, 0.13);
    transition: background 0.15s;
  }
  .card:hover { background: rgba(255, 255, 255, 0.02); }
  .card.selected {
    background: rgba(96, 165, 250, 0.08);
    border-left-color: var(--blue);
  }
  .card.input { animation: pulse 2s ease-in-out infinite; }
  @keyframes pulse {
    0%, 100% { box-shadow: none; }
    50% { box-shadow: inset 0 0 12px rgba(245, 158, 11, 0.1); }
  }
  .header { display: flex; justify-content: space-between; align-items: center; }
  .name { font-size: 13px; font-weight: 600; color: var(--text-primary); }
  .status {
    padding: 0 5px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 600;
  }
  .status.working { background: var(--green-dim); color: var(--green); }
  .status.input { background: var(--amber-dim); color: var(--amber); }
  .status.idle { background: rgba(71, 85, 105, 0.2); color: var(--text-muted); }
  .status.new { background: var(--blue-dim); color: var(--blue); }
  .meta { font-size: 11px; color: var(--text-dim); margin-top: 2px; }
  .context-bar {
    margin-top: 4px;
    height: 2px;
    background: var(--border);
    border-radius: 1px;
    overflow: hidden;
  }
  .context-fill {
    height: 100%;
    background: var(--green);
    border-radius: 1px;
    transition: width 0.3s;
  }
  .context-fill.warn { background: var(--amber); }
  .context-fill.danger { background: var(--red); }
</style>
