<script lang="ts">
  import { onMount } from 'svelte';
  import type { Session } from '../lib/stores/sessions';
  import { formatTokens } from '../lib/cost';
  import { getClaudeUsageStats, type ClaudeUsageStats } from '../lib/tauri';
  import { modelDisplayName } from '../lib/status';
  import { sessionEffort } from '../lib/stores/ui';

  export let session: Session;

  $: tokens = session.tokens;
  $: totalTokens = (tokens?.input ?? 0) + (tokens?.output ?? 0);
  $: ctx = session.contextPercent ?? 0;

  let usage: ClaudeUsageStats | null = null;

  onMount(async () => {
    try {
      usage = await getClaudeUsageStats();
    } catch {
      // not in Tauri or no stats file
    }
  });
</script>

<div class="stats">
  <div class="stat-row">
    <span class="label">Input tokens</span>
    <span class="value">{(tokens?.input ?? 0).toLocaleString()}</span>
  </div>
  <div class="stat-row">
    <span class="label">Output tokens</span>
    <span class="value">{(tokens?.output ?? 0).toLocaleString()}</span>
  </div>
  <div class="stat-row">
    <span class="label">Cache read</span>
    <span class="value">{(tokens?.cacheRead ?? 0).toLocaleString()}</span>
  </div>
  <div class="stat-row">
    <span class="label">Cache write</span>
    <span class="value">{(tokens?.cacheWrite ?? 0).toLocaleString()}</span>
  </div>
  <div class="stat-row total">
    <span class="label">Total</span>
    <span class="value">{totalTokens.toLocaleString()}</span>
  </div>
  <div class="context-section">
    <div class="context-label">
      Context window: {ctx.toFixed(1)}%
    </div>
    <div class="context-bar">
      <div
        class="fill"
        style="width: {Math.min(ctx, 100)}%"
        class:warn={ctx > 70}
        class:danger={ctx > 90}
      ></div>
    </div>
  </div>
  <div class="stat-row">
    <span class="label">Model</span>
    <span class="value">{modelDisplayName(session.model)}</span>
  </div>
  <div class="stat-row">
    <span class="label">Effort</span>
    <span class="value">{sessionEffort.get($sessionEffort, String(session.id))}</span>
  </div>
  {#if usage}
    <div class="usage-section">
      <div class="usage-title">Weekly usage (7d)</div>
      <div class="stat-row">
        <span class="label">Tokens</span>
        <span class="value">{formatTokens(usage.weeklyTokens)}</span>
      </div>
      <div class="stat-row">
        <span class="label">Today</span>
        <span class="value">{formatTokens(usage.todayTokens)}</span>
      </div>
      <div class="stat-row">
        <span class="label">Messages</span>
        <span class="value">{usage.weeklyMessages.toLocaleString()}</span>
      </div>
    </div>
  {/if}
</div>

<style>
  .stats {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .stat-row {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
    padding: 4px 0;
    border-bottom: 1px solid var(--border-subtle);
  }
  .stat-row.total {
    font-weight: 600;
    border-bottom: 1px solid var(--border-subtle);
    margin-top: 4px;
  }
  .label {
    color: var(--text-muted);
  }
  .value {
    color: var(--text-primary);
  }
  .context-section {
    margin-top: 12px;
  }
  .context-label {
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }
  .context-bar {
    height: 6px;
    background: var(--border);
    border-radius: 3px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--green);
    border-radius: 3px;
    transition: width 0.3s;
  }
  .fill.warn {
    background: var(--amber);
  }
  .fill.danger {
    background: var(--red);
  }
  .usage-section {
    margin-top: 12px;
    border-top: 1px solid var(--border);
    padding-top: 8px;
  }
  .usage-title {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin-bottom: 4px;
  }
</style>
