<script lang="ts">
  import type { AgentState } from '../lib/types';
  import { journal } from '../lib/stores/journal';
  import { detailLevel } from '../lib/stores/preferences';
  import { getJournal } from '../lib/tauri';
  import JournalEntry from './JournalEntry.svelte';
  import AgentTree from './AgentTree.svelte';
  import CommandInput from './CommandInput.svelte';

  export let agent: AgentState;

  let logContainer: HTMLDivElement;

  async function loadJournal(sessionId: string) {
    const entries = await getJournal(sessionId);
    journal.set(entries);
    // Scroll to bottom
    requestAnimationFrame(() => {
      if (logContainer) logContainer.scrollTop = logContainer.scrollHeight;
    });
  }

  // Reload journal on each agents-update cycle
  $: if (agent) {
    loadJournal(agent.sessionId);
  }

  $: filteredEntries = $detailLevel === 'compact'
    ? $journal.filter(e => e.entryType === 'toolCall' || e.entryType === 'toolResult')
    : $journal;
</script>

<div class="central-panel">
  <div class="header">
    <div class="left">
      <span class="name">{agent.project}</span>
      <span class="status {agent.status}">{agent.status.toUpperCase()}</span>
      <span class="meta">
        {agent.gitBranch ?? ''} • {agent.modelDisplay} • {Math.round((agent.tokens.input + agent.tokens.output) / 1000)}K/{agent.contextPercent > 0 ? Math.round(agent.contextPercent) + '%' : ''}
      </span>
    </div>
    <div class="level-toggle">
      <button class:active={$detailLevel === 'compact'} onclick={() => detailLevel.set('compact')}>Compact</button>
      <button class:active={$detailLevel === 'full'} onclick={() => detailLevel.set('full')}>Full</button>
      <button class:active={$detailLevel === 'raw'} onclick={() => detailLevel.set('raw')}>Raw</button>
    </div>
  </div>

  <AgentTree subagents={agent.subagents} />

  <div class="log" bind:this={logContainer}>
    {#if $detailLevel === 'raw'}
      <pre class="raw-log mono">{JSON.stringify($journal, null, 2)}</pre>
    {:else}
      {#each filteredEntries as entry (entry.timestamp + entry.entryType)}
        <JournalEntry {entry} />
      {/each}
    {/if}
  </div>

  <CommandInput sessionId={agent.sessionId} agentName={agent.project} />
</div>

<style>
  .central-panel { display: flex; flex-direction: column; height: 100%; }
  .header {
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .left { display: flex; align-items: center; gap: 8px; }
  .name { font-weight: 600; font-size: 14px; }
  .status { padding: 1px 6px; border-radius: 8px; font-size: 11px; }
  .status.working { background: var(--green-dim); color: var(--green); }
  .status.input { background: var(--amber-dim); color: var(--amber); }
  .status.idle { background: var(--bg-idle); color: var(--text-muted); }
  .meta { font-size: 12px; color: var(--text-dim); }
  .level-toggle {
    display: flex;
    gap: 2px;
    background: var(--bg-overlay);
    border-radius: 6px;
    padding: 2px;
  }
  .level-toggle button {
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 12px;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .level-toggle button.active {
    background: var(--bg-active-toggle);
    color: var(--blue);
  }
  .log {
    flex: 1;
    overflow-y: auto;
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .raw-log {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
    white-space: pre-wrap;
  }
</style>
