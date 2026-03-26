<script lang="ts">
  import type { AgentState, JournalEntry } from '../lib/types';
  import { rightPanelTab } from '../lib/stores/preferences';
  import DiffView from './DiffView.svelte';
  import FilesImpact from './FilesImpact.svelte';
  import TasksProgress from './TasksProgress.svelte';
  import StatsPanel from './StatsPanel.svelte';

  export let agent: AgentState;
  export let entries: JournalEntry[];

  const tabs = ['diff', 'files', 'tasks', 'stats'] as const;
</script>

<aside class="right-panel">
  <div class="tabs">
    {#each tabs as tab}
      <button
        class="tab"
        class:active={$rightPanelTab === tab}
        onclick={() => rightPanelTab.set(tab)}
      >
        {tab.charAt(0).toUpperCase() + tab.slice(1)}
      </button>
    {/each}
  </div>
  <div class="content">
    {#if $rightPanelTab === 'diff'}
      <DiffView sessionId={agent.sessionId} />
    {:else if $rightPanelTab === 'files'}
      <FilesImpact {entries} />
    {:else if $rightPanelTab === 'tasks'}
      <TasksProgress />
    {:else if $rightPanelTab === 'stats'}
      <StatsPanel {agent} />
    {/if}
  </div>
</aside>

<style>
  .right-panel {
    width: 300px;
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }
  .tabs { display: flex; border-bottom: 1px solid var(--border); }
  .tab {
    padding: 8px 12px;
    font-size: 10px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-muted);
    cursor: pointer;
  }
  .tab.active { color: var(--blue); border-bottom-color: var(--blue); }
  .content { flex: 1; overflow-y: auto; }
</style>
