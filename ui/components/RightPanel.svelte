<script lang="ts">
  import { sessions, selectedSessionId, getSelectedSession } from '../lib/stores/sessions';
  import { rightPanelTab } from '../lib/stores/preferences';
  import TasksProgress from './TasksProgress.svelte';
  import StatsPanel from './StatsPanel.svelte';
  import SubagentsPanel from './SubagentsPanel.svelte';

  $: session = getSelectedSession($sessions, $selectedSessionId);

  const tabs: { key: typeof $rightPanelTab; label: string }[] = [
    { key: 'agents', label: 'Sub-agents' },
    { key: 'tasks', label: 'Tasks' },
    { key: 'stats', label: 'Stats' },
  ];
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
        <SubagentsPanel sessionId={session.id.toString()} subagents={[]} />
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
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }
  .tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
  }
  .tab {
    padding: 8px 12px;
    font-size: 12px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-muted);
    cursor: pointer;
  }
  .tab.active {
    color: var(--blue);
    border-bottom-color: var(--blue);
  }
  .content {
    flex: 1;
    overflow-y: auto;
  }
</style>
