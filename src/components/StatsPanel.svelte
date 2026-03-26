<script lang="ts">
  import type { AgentState } from '../lib/types';

  export let agent: AgentState;

  $: totalTokens = agent.tokens.input + agent.tokens.output;
</script>

<div class="stats">
  <div class="stat-row">
    <span class="label">Input tokens</span>
    <span class="value">{agent.tokens.input.toLocaleString()}</span>
  </div>
  <div class="stat-row">
    <span class="label">Output tokens</span>
    <span class="value">{agent.tokens.output.toLocaleString()}</span>
  </div>
  <div class="stat-row">
    <span class="label">Cache read</span>
    <span class="value">{agent.tokens.cacheRead.toLocaleString()}</span>
  </div>
  <div class="stat-row">
    <span class="label">Cache write</span>
    <span class="value">{agent.tokens.cacheWrite.toLocaleString()}</span>
  </div>
  <div class="stat-row total">
    <span class="label">Total</span>
    <span class="value">{totalTokens.toLocaleString()}</span>
  </div>
  <div class="context-section">
    <div class="context-label">
      Context window: {agent.contextPercent.toFixed(1)}%
    </div>
    <div class="context-bar">
      <div
        class="fill"
        style="width: {Math.min(agent.contextPercent, 100)}%"
        class:warn={agent.contextPercent > 70}
        class:danger={agent.contextPercent > 90}
      ></div>
    </div>
  </div>
  <div class="stat-row">
    <span class="label">Model</span>
    <span class="value">{agent.modelDisplay}</span>
  </div>
  <div class="stat-row">
    <span class="label">Subagents</span>
    <span class="value">{agent.subagents.length}</span>
  </div>
</div>

<style>
  .stats { padding: 12px; display: flex; flex-direction: column; gap: 6px; }
  .stat-row {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
    padding: 4px 0;
    border-bottom: 1px solid rgba(30, 41, 59, 0.3);
  }
  .stat-row.total { font-weight: 600; border-bottom: none; margin-top: 4px; }
  .label { color: var(--text-muted); }
  .value { color: var(--text-primary); }
  .context-section { margin-top: 12px; }
  .context-label { font-size: 12px; color: var(--text-secondary); margin-bottom: 4px; }
  .context-bar { height: 6px; background: var(--border); border-radius: 3px; overflow: hidden; }
  .fill {
    height: 100%;
    background: var(--green);
    border-radius: 3px;
    transition: width 0.3s;
  }
  .fill.warn { background: var(--amber); }
  .fill.danger { background: var(--red); }
</style>
