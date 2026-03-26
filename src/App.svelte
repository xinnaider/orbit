<script lang="ts">
  import { onMount } from 'svelte';
  import { agents, selectedAgentId } from './lib/stores/agents';
  import { onAgentsUpdate } from './lib/tauri';
  import Sidebar from './components/Sidebar.svelte';

  onMount(() => {
    const unlisten = onAgentsUpdate((update) => {
      agents.set(update);
      // Auto-select first agent if none selected
      if (!$selectedAgentId && update.length > 0) {
        selectedAgentId.set(update[0].sessionId);
      }
    });

    return () => { unlisten.then(fn => fn()); };
  });

  function handleSelect(id: string) {
    selectedAgentId.set(id);
  }

  $: currentAgent = $agents.find(a => a.sessionId === $selectedAgentId) ?? null;
</script>

<div class="workspace">
  <Sidebar
    agents={$agents}
    selectedId={$selectedAgentId}
    onSelect={handleSelect}
  />

  <main class="central">
    <div class="central-header">
      {#if currentAgent}
        <span class="agent-name">{currentAgent.project}</span>
        <span class="status {currentAgent.status}">{currentAgent.status.toUpperCase()}</span>
      {:else}
        <span class="placeholder">Select an agent</span>
      {/if}
    </div>
    <div class="central-content">
      <!-- CentralPanel will be added in Task 11 -->
    </div>
  </main>

  <aside class="right-panel">
    <div class="panel-tabs">
      <button class="tab active">Diff</button>
      <button class="tab">Files</button>
      <button class="tab">Tasks</button>
      <button class="tab">Stats</button>
    </div>
    <div class="panel-content"></div>
  </aside>
</div>

<style>
  .workspace { display: flex; height: 100vh; width: 100vw; }
  .central { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .central-header {
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .agent-name { font-weight: 600; font-size: 13px; }
  .status { padding: 1px 6px; border-radius: 8px; font-size: 9px; }
  .status.working { background: var(--green-dim); color: var(--green); }
  .status.input { background: var(--amber-dim); color: var(--amber); }
  .status.idle { background: rgba(71,85,105,0.2); color: var(--text-muted); }
  .placeholder { color: var(--text-muted); font-size: 13px; }
  .central-content { flex: 1; overflow-y: auto; }
  .right-panel {
    width: 300px;
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }
  .panel-tabs { display: flex; border-bottom: 1px solid var(--border); }
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
  .panel-content { flex: 1; overflow-y: auto; }
</style>
