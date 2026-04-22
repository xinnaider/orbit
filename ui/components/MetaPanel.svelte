<script lang="ts">
  import type { Session } from '../lib/stores/sessions';
  import { formatTokens } from '../lib/cost';
  import { stopSession, getSubagents } from '../lib/tauri';
  import { isActive, modelShortName } from '../lib/status';
  import { sessionEffort } from '../lib/stores/ui';
  import { metaPanelVisible } from '../lib/stores/preferences';
  import { sessions, updateSessionState } from '../lib/stores/sessions';
  import TasksList from './TasksList.svelte';
  import SubagentsPanel from './SubagentsPanel.svelte';
  import { providerCaps, getCaps } from '../lib/stores/providers';

  export let session: Session;
  $: caps = getCaps($providerCaps, session.provider);

  type Tab = 'stats' | 'tasks' | 'agents';
  let tab: Tab = 'stats';

  let refreshing = false;

  async function stop() {
    try {
      await stopSession(session.id);
    } catch (_e) {
      /* no-op */
    }
  }

  async function refreshAgents() {
    if (refreshing) return;
    refreshing = true;
    try {
      const subagents = await getSubagents(session.id);
      sessions.update((l) => updateSessionState(l, session.id, { subagents }));
    } catch (_e) {
      /* no-op */
    } finally {
      refreshing = false;
    }
  }

  $: tokens = session.tokens;
  $: total = (tokens?.input ?? 0) + (tokens?.output ?? 0);
  $: ctx = session.contextPercent ?? 0;
  $: active = isActive(session.status);
  $: stopped = session.status === 'stopped';
</script>

<aside class="meta">
  <div class="tabs">
    <button class="tab" class:active={tab === 'stats'} on:click={() => (tab = 'stats')}
      >stats</button
    >
    <button class="tab" class:active={tab === 'tasks'} on:click={() => (tab = 'tasks')}
      >tasks</button
    >
    <button class="tab" class:active={tab === 'agents'} on:click={() => (tab = 'agents')}
      >agents</button
    >
    <span class="tabs-spacer"></span>
    <button class="collapse-btn" on:click={() => metaPanelVisible.set(false)} title="Hide panel"
      >›</button
    >
  </div>

  <div class="content">
    {#if tab === 'stats'}
      <div class="stats">
        <div class="stat-group">
          <div class="stat-label">tokens</div>
          <div class="stat-value big">{formatTokens(total)}</div>
          {#if tokens}
            <div class="stat-row"><span>input</span><span>{formatTokens(tokens.input)}</span></div>
            <div class="stat-row">
              <span>output</span><span>{formatTokens(tokens.output)}</span>
            </div>
            <div class="stat-row dim">
              <span>cache·r</span><span>{formatTokens(tokens.cacheRead)}</span>
            </div>
            <div class="stat-row dim">
              <span>cache·w</span><span>{formatTokens(tokens.cacheWrite)}</span>
            </div>
          {/if}
          {#if session.costUsd != null}
            <div class="stat-row">
              <span>cost</span><span class="cost-val">${session.costUsd.toFixed(4)}</span>
            </div>
          {/if}
        </div>

        {#if ctx > 0}
          {@const maxCtx = session.contextWindow ?? 200_000}
          {@const usedTokens = Math.round((ctx / 100) * maxCtx)}
          <div class="stat-group">
            <div class="stat-label">context</div>
            <div class="ctx-row">
              <div class="ctx-bar">
                <div
                  class="ctx-fill"
                  style="width:{Math.min(ctx, 100)}%;
                    background:{ctx > 85
                    ? 'var(--s-error)'
                    : ctx > 65
                      ? 'var(--s-input)'
                      : 'var(--ac)'}"
                ></div>
              </div>
              <span class="ctx-pct">{Math.round(ctx)}%</span>
            </div>
            <div class="stat-row dim">
              <span>used</span><span>{formatTokens(usedTokens)}</span>
            </div>
            <div class="stat-row dim">
              <span>max</span><span>{formatTokens(maxCtx)}</span>
            </div>
          </div>
        {/if}

        {#if session.rateLimit && session.rateLimit.filter((rl) => rl.status === 'allowed_warning' || rl.status === 'exceeded' || rl.status === 'blocked').length > 0}
          {#each session.rateLimit.filter((rl) => rl.status === 'allowed_warning' || rl.status === 'exceeded' || rl.status === 'blocked') as rl}
            <div class="stat-group">
              <div class="stat-label rate-limit-label">
                rate limit ({rl.rateLimitType === 'five_hour'
                  ? '5h'
                  : rl.rateLimitType === 'seven_day'
                    ? '7d'
                    : rl.rateLimitType})
              </div>
              <div class="ctx-row">
                <div class="ctx-bar">
                  <div
                    class="ctx-fill rate-limit-fill"
                    style="width:{Math.min(Math.round(rl.utilization * 100), 100)}%"
                  ></div>
                </div>
                <span class="ctx-pct rate-limit-pct">{Math.round(rl.utilization * 100)}%</span>
              </div>
              {#if rl.resetsAt}
                <div class="stat-row dim">
                  <span>resets</span><span
                    >{new Date(rl.resetsAt * 1000).toLocaleTimeString([], {
                      hour: '2-digit',
                      minute: '2-digit',
                    })}</span
                  >
                </div>
              {/if}
              <div class="stat-row dim">
                <span>status</span><span
                  >{rl.status === 'allowed_warning'
                    ? 'warning'
                    : rl.status === 'exceeded'
                      ? 'exceeded'
                      : rl.status}</span
                >
              </div>
              {#if rl.isUsingOverage}
                <div class="stat-row dim">
                  <span>overage</span><span>yes</span>
                </div>
              {/if}
            </div>
          {/each}
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
          <div class="stat-row">
            <span>model</span><span class="mono-val" title={session.model ?? ''}
              >{modelShortName(session.model)}</span
            >
          </div>
          {#if caps.supportsEffort}
            <div class="stat-row">
              <span>effort</span><span class="mono-val"
                >{sessionEffort.get($sessionEffort, String(session.id))}</span
              >
            </div>
          {/if}
          <div class="stat-row">
            <span>pid</span><span class="mono-val">{session.pid ?? '—'}</span>
          </div>
          <div class="stat-row">
            <span>mode</span><span class="mono-val">{session.permissionMode}</span>
          </div>
        </div>
      </div>
    {:else if tab === 'tasks'}
      <TasksList sessionId={String(session.id)} />
    {:else}
      <SubagentsPanel
        sessionId={session.id}
        subagents={session.subagents ?? []}
        {refreshing}
        onRefresh={refreshAgents}
      />
    {/if}
  </div>
</aside>

<style>
  .meta {
    width: 200px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--bd);
    background: var(--bg1);
  }

  .tabs {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--bd);
    padding: 0 var(--sp-1);
    flex-shrink: 0;
  }
  .tab {
    background: none;
    border: none;
    color: var(--t2);
    font-size: var(--xs);
    padding: var(--sp-5) var(--sp-5) var(--sp-4);
    letter-spacing: 0.06em;
    border-bottom: 1px solid transparent;
    margin-bottom: -1px;
    transition: color 0.15s;
  }
  .tab:hover {
    color: var(--t1);
  }
  .tab.active {
    color: var(--t0);
    border-bottom-color: var(--ac);
  }
  .tabs-spacer {
    flex: 1;
  }
  .collapse-btn {
    margin-left: auto;
    background: none;
    border: none;
    color: var(--t2);
    font-size: 14px;
    padding: var(--sp-2) var(--sp-3);
    line-height: 1;
    cursor: pointer;
    transition: color 0.15s;
  }
  .collapse-btn:hover {
    color: var(--t0);
  }

  .content {
    flex: 1;
    overflow-y: auto;
  }

  .stats {
    padding: var(--sp-5) 0;
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .stat-group {
    padding: var(--sp-4) var(--sp-6);
    border-bottom: 1px solid var(--bd);
  }
  .stat-group:last-child {
    border-bottom: none;
  }
  .stat-label {
    font-size: var(--xs);
    color: var(--t2);
    letter-spacing: 0.08em;
    margin-bottom: var(--sp-2);
  }
  .stat-value {
    font-size: var(--md);
    color: var(--t0);
  }
  .stat-value.big {
    font-size: 18px;
    font-weight: 300;
    color: var(--t0);
    margin-bottom: 6px;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    font-size: var(--xs);
    color: var(--t1);
    padding: 1px 0;
  }
  .stat-row.dim {
    color: var(--t2);
  }
  .mono-val {
    color: var(--t1);
    font-size: var(--xs);
  }

  .ctx-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .ctx-bar {
    flex: 1;
    height: 3px;
    background: var(--bg3);
    border-radius: 2px;
    overflow: hidden;
  }
  .ctx-fill {
    height: 100%;
    border-radius: 2px;
    transition: width 0.3s;
  }
  .ctx-pct {
    font-size: var(--xs);
    color: var(--t2);
    flex-shrink: 0;
  }

  .rate-limit-label {
    color: var(--s-input);
  }
  .rate-limit-fill {
    background: var(--s-input);
  }
  .rate-limit-pct {
    color: var(--s-input);
  }
  .cost-val {
    color: var(--t0);
    font-size: var(--xs);
  }

  .log-row {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 2px 0;
    font-size: var(--xs);
  }
  .log-tool {
    color: var(--tool-fg);
    font-weight: 500;
    flex-shrink: 0;
  }
  .log-target {
    color: var(--t2);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .log-status {
    flex-shrink: 0;
  }
  .log-status.ok {
    color: var(--s-working);
  }
  .log-status.fail {
    color: var(--s-error);
  }

  .meta-info .stat-row {
    color: var(--t2);
  }
</style>
