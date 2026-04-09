<script lang="ts">
  import {
    sessions,
    selectedSessionId,
    getSelectedSession,
    updateSessionState,
  } from '../lib/stores/sessions';
  import { rightPanelTab } from '../lib/stores/preferences';
  import { getSubagents } from '../lib/tauri';
  import TasksProgress from './TasksProgress.svelte';
  import StatsPanel from './StatsPanel.svelte';
  import SubagentsPanel from './SubagentsPanel.svelte';

  $: session = getSelectedSession($sessions, $selectedSessionId);

  const tabs: { key: typeof $rightPanelTab; label: string }[] = [
    { key: 'agents', label: 'Sub-agents' },
    { key: 'tasks', label: 'Tasks' },
    { key: 'stats', label: 'Stats' },
  ];

  let refreshing = false;

  async function refreshSubagents() {
    if (!session || refreshing) return;
    refreshing = true;
    try {
      const subagents = await getSubagents(session.id);
      sessions.update((l) => updateSessionState(l, session!.id, { subagents }));
    } finally {
      refreshing = false;
    }
  }
</script>

<aside class="right-panel">
  <div class="tabs">
    {#each tabs as tab}
      <button
        class="tab"
        class:active={$rightPanelTab === tab.key}
        onclick={() => rightPanelTab.set(tab.key)}
      >
        {tab.label}
      </button>
    {/each}
  </div>
  <div class="content">
    {#if session}
      {#if $rightPanelTab === 'agents'}
        <div class="agents-header">
          <button
            class="refresh-btn"
            onclick={refreshSubagents}
            disabled={refreshing}
            title="Refresh sub-agents"
          >
            {refreshing ? '…' : '↺'}
          </button>
        </div>
        <SubagentsPanel sessionId={session.id.toString()} subagents={session.subagents ?? []} />
      {:else if $rightPanelTab === 'tasks'}
        <TasksProgress sessionId={session.id.toString()} />
      {:else if $rightPanelTab === 'stats'}
        <StatsPanel {session} />
      {/if}
    {/if}
  </div>
</aside>

<style>
  .right-panel {
    width: 300px;
    border-left: 1px solid var(--bd);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }
  .tabs {
    display: flex;
    border-bottom: 1px solid var(--bd);
  }
  .tab {
    padding: 8px 12px;
    font-size: 12px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--t2);
    cursor: pointer;
    font-family: var(--mono);
  }
  .tab.active {
    color: var(--ac);
    border-bottom-color: var(--ac);
  }
  .content {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
  .agents-header {
    display: flex;
    justify-content: flex-end;
    padding: 4px 8px 0;
  }
  .refresh-btn {
    background: none;
    border: none;
    color: var(--t2);
    font-size: 13px;
    font-family: var(--mono);
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    line-height: 1;
  }
  .refresh-btn:hover:not(:disabled) {
    color: var(--t0);
    background: var(--bg3);
  }
  .refresh-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
