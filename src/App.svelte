<script lang="ts">
  import { onMount } from 'svelte';
  import { agents, selectedAgentId } from './lib/stores/agents';
  import { onAgentsUpdate } from './lib/tauri';
  import { journal } from './lib/stores/journal';
  import Sidebar from './components/Sidebar.svelte';
  import CentralPanel from './components/CentralPanel.svelte';
  import RightPanel from './components/RightPanel.svelte';

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
    {#if currentAgent}
      <CentralPanel agent={currentAgent} />
    {:else}
      <div class="central-empty">Select an agent from the sidebar</div>
    {/if}
  </main>

  {#if currentAgent}
    <RightPanel agent={currentAgent} entries={$journal} />
  {:else}
    <aside class="right-panel">
      <div class="panel-tabs">
        <button class="tab active">Diff</button>
        <button class="tab">Files</button>
        <button class="tab">Tasks</button>
        <button class="tab">Stats</button>
      </div>
    </aside>
  {/if}
</div>

<style>
  .workspace { display: flex; height: 100vh; width: 100vw; }
  .central { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .central-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    font-size: 13px;
  }
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
</style>
