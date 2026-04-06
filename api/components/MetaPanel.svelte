<script lang="ts">
  import type { Session } from '../lib/stores/sessions';
  import { estimateCost, formatCost, formatTokens } from '../lib/cost';
  import { stopSession } from '../lib/tauri';
  import { isActive } from '../lib/status';
  import TasksList from './TasksList.svelte';

  export let session: Session;

  type Tab = 'stats' | 'tasks';
  let tab: Tab = 'stats';

  async function stop() {
    try { await stopSession(session.id); } catch (_e) { /* no-op */ }
  }

  $: tokens = session.tokens;
  $: total = (tokens?.input ?? 0) + (tokens?.output ?? 0);
  $: cost = tokens ? estimateCost(tokens, session.model) : 0;
  $: ctx = session.contextPercent ?? 0;
  $: active = isActive(session.status);
</script>

<aside class="meta">
  <div class="tabs">
    <button class="tab" class:active={tab === 'stats'} on:click={() => tab = 'stats'}>stats</button>
    <button class="tab" class:active={tab === 'tasks'} on:click={() => tab = 'tasks'}>tasks</button>
    {#if active}
      <button class="stop-btn" on:click={stop} title="Stop session">■</button>
    {/if}
  </div>

  <div class="content">
    {#if tab === 'stats'}
      <div class="stats">
        <div class="stat-group">
          <div class="stat-label">tokens</div>
          <div class="stat-value big">{formatTokens(total)}</div>
          {#if tokens}
            <div class="stat-row"><span>input</span><span>{formatTokens(tokens.input)}</span></div>
            <div class="stat-row"><span>output</span><span>{formatTokens(tokens.output)}</span></div>
            <div class="stat-row dim"><span>cache·r</span><span>{formatTokens(tokens.cacheRead)}</span></div>
            <div class="stat-row dim"><span>cache·w</span><span>{formatTokens(tokens.cacheWrite)}</span></div>
          {/if}
        </div>

        <div class="stat-group">
          <div class="stat-label">cost</div>
          <div class="stat-value big">{formatCost(cost)}</div>
        </div>

        {#if ctx > 0}
          <div class="stat-group">
            <div class="stat-label">context</div>
            <div class="ctx-row">
              <div class="ctx-bar">
                <div class="ctx-fill"
                  style="width:{Math.min(ctx,100)}%;
                    background:{ctx > 85 ? 'var(--s-error)' : ctx > 65 ? 'var(--s-input)' : 'var(--ac)'}">
                </div>
              </div>
              <span class="ctx-pct">{Math.round(ctx)}%</span>
            </div>
          </div>
        {/if}

        {#if session.miniLog && session.miniLog.length > 0}
          <div class="stat-group">
            <div class="stat-label">recent tools</div>
            {#each session.miniLog as log}
              <div class="log-row">
                <span class="log-tool">{log.tool}</span>
                {#if log.target}
                  <span class="log-target">{log.target.slice(0, 24)}</span>
                {/if}
                {#if log.success !== null}
                  <span class="log-status" class:ok={log.success} class:fail={!log.success}>
                    {log.success ? '✓' : '✗'}
                  </span>
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        <div class="stat-group meta-info">
          <div class="stat-row"><span>model</span><span class="mono-val">{session.model?.split('-').slice(-2).join('-') ?? '—'}</span></div>
          <div class="stat-row"><span>pid</span><span class="mono-val">{session.pid ?? '—'}</span></div>
          <div class="stat-row"><span>mode</span><span class="mono-val">{session.permissionMode}</span></div>
        </div>
      </div>
    {:else}
      <TasksList sessionId={String(session.id)} />
    {/if}
  </div>
</aside>

<style>
  .meta {
    width: 200px; flex-shrink: 0;
    display: flex; flex-direction: column;
    border-left: 1px solid var(--bd);
    background: var(--bg1);
  }

  .tabs {
    display: flex; align-items: center;
    border-bottom: 1px solid var(--bd);
    padding: 0 2px;
    flex-shrink: 0;
  }
  .tab {
    background: none; border: none;
    color: var(--t2); font-size: var(--xs);
    padding: 9px 10px 8px;
    letter-spacing: 0.06em;
    border-bottom: 1px solid transparent;
    margin-bottom: -1px;
    transition: color 0.15s;
  }
  .tab:hover { color: var(--t1); }
  .tab.active { color: var(--t0); border-bottom-color: var(--ac); }
  .stop-btn {
    margin-left: auto; margin-right: 4px;
    background: none; border: none;
    color: var(--t2); font-size: 10px;
    padding: 4px 6px;
  }
  .stop-btn:hover { color: var(--s-error); }

  .content { flex: 1; overflow-y: auto; }

  .stats { padding: 10px 0; display: flex; flex-direction: column; gap: 0; }
  .stat-group {
    padding: 8px 12px;
    border-bottom: 1px solid var(--bd);
  }
  .stat-group:last-child { border-bottom: none; }
  .stat-label {
    font-size: var(--xs); color: var(--t2);
    letter-spacing: 0.08em; margin-bottom: 4px;
  }
  .stat-value { font-size: var(--md); color: var(--t0); }
  .stat-value.big { font-size: 18px; font-weight: 300; color: var(--t0); margin-bottom: 6px; }

  .stat-row {
    display: flex; justify-content: space-between;
    font-size: var(--xs); color: var(--t1);
    padding: 1px 0;
  }
  .stat-row.dim { color: var(--t2); }
  .mono-val { color: var(--t1); font-size: var(--xs); }

  .ctx-row { display: flex; align-items: center; gap: 8px; }
  .ctx-bar { flex: 1; height: 3px; background: var(--bg3); border-radius: 2px; overflow: hidden; }
  .ctx-fill { height: 100%; border-radius: 2px; transition: width 0.3s; }
  .ctx-pct { font-size: var(--xs); color: var(--t2); flex-shrink: 0; }

  .log-row {
    display: flex; align-items: center; gap: 5px;
    padding: 2px 0;
    font-size: var(--xs);
  }
  .log-tool { color: var(--tool-fg); font-weight: 500; flex-shrink: 0; }
  .log-target { color: var(--t2); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .log-status { flex-shrink: 0; }
  .log-status.ok   { color: var(--s-working); }
  .log-status.fail { color: var(--s-error); }

  .meta-info .stat-row { color: var(--t2); }
</style>
