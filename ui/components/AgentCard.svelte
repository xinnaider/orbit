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
  class:working={agent.status === 'working'}
  onclick={onClick}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === 'Enter' && onClick()}
>
  <div class="header">
    <span class="name">{agent.project}</span>
    <span class="status {agent.status}">
      {#if agent.status === 'working'}
        <span class="working-dot"></span>
      {:else if agent.status === 'input'}
        <span class="input-dot"></span>
      {/if}
      {agent.status.toUpperCase()}
    </span>
  </div>
  <div class="meta">
    <span>{agent.gitBranch ?? 'no branch'}</span>
    <span class="sep">·</span>
    <span>{agent.modelDisplay}</span>
    <span class="sep">·</span>
    <span>{Math.round((agent.tokens.input + agent.tokens.output) / 1000)}K</span>
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
    border-left: 3px solid transparent;
    cursor: pointer;
    border-bottom: 1px solid var(--border-subtle);
    transition: all 0.2s ease;
  }
  .card:hover {
    background: var(--bg-hover);
  }
  .card.selected {
    background: var(--bg-selected);
    border-left-color: var(--blue);
  }
  .card.working {
    border-left-color: var(--green);
  }
  .card.input {
    border-left-color: var(--amber);
    animation: inputPulse 2s ease-in-out infinite;
  }
  @keyframes inputPulse {
    0%,
    100% {
      box-shadow: none;
    }
    50% {
      box-shadow: inset 0 0 12px var(--pulse-glow);
    }
  }
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .status {
    padding: 2px 7px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .status.working {
    background: var(--green-dim);
    color: var(--green);
  }
  .status.input {
    background: var(--amber-dim);
    color: var(--amber);
  }
  .status.idle {
    background: var(--bg-idle);
    color: var(--text-muted);
  }
  .status.new {
    background: var(--blue-dim);
    color: var(--blue);
  }
  .working-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--green);
    animation: blink 1.2s ease-in-out infinite;
  }
  .input-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--amber);
    animation: blink 0.8s ease-in-out infinite;
  }
  @keyframes blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.2;
    }
  }
  .meta {
    font-size: 11px;
    color: var(--text-dim);
    margin-top: 4px;
    display: flex;
    gap: 4px;
  }
  .sep {
    opacity: 0.5;
  }
  .context-bar {
    margin-top: 6px;
    height: 3px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
  }
  .context-fill {
    height: 100%;
    background: var(--green);
    border-radius: 2px;
    transition: width 0.5s ease;
  }
  .context-fill.warn {
    background: var(--amber);
  }
  .context-fill.danger {
    background: var(--red);
  }
</style>
