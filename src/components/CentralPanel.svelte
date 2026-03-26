<script lang="ts">
  import type { AgentState } from '../lib/types';
  import { journal } from '../lib/stores/journal';
  import { detailLevel } from '../lib/stores/preferences';
  import { getJournal } from '../lib/tauri';
  import JournalEntry from './JournalEntry.svelte';
  import AgentTree from './AgentTree.svelte';
  import CommandInput from './CommandInput.svelte';
  import TypingIndicator from './TypingIndicator.svelte';

  export let agent: AgentState;

  let logContainer: HTMLDivElement;

  async function loadJournal(sessionId: string) {
    const entries = await getJournal(sessionId);
    journal.set(entries);
    requestAnimationFrame(() => {
      if (logContainer) logContainer.scrollTop = logContainer.scrollHeight;
    });
  }

  $: if (agent) {
    loadJournal(agent.sessionId);
  }

  $: filteredEntries = $detailLevel === 'compact'
    ? $journal.filter(e => e.entryType === 'toolCall' || e.entryType === 'toolResult')
    : $journal;

  // Determine typing label based on last entry
  $: typingLabel = (() => {
    const last = $journal[$journal.length - 1];
    if (!last) return 'Thinking';
    if (last.entryType === 'thinking') return 'Thinking';
    if (last.entryType === 'toolCall') return `Running ${last.tool ?? 'tool'}`;
    if (last.entryType === 'toolResult') return 'Processing result';
    return 'Thinking';
  })();
</script>

<div class="central-panel">
  <div class="header">
    <div class="left">
      <span class="name">{agent.project}</span>
      <span class="status {agent.status}">
        {#if agent.status === 'working'}
          <span class="status-dot-anim"></span>
        {/if}
        {agent.status.toUpperCase()}
      </span>
      <span class="meta">
        {agent.gitBranch ?? ''} · {agent.modelDisplay} · {Math.round((agent.tokens.input + agent.tokens.output) / 1000)}K
        {#if agent.contextPercent > 0}
          · {Math.round(agent.contextPercent)}% ctx
        {/if}
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
      {#each filteredEntries as entry, i (entry.timestamp + entry.entryType + i)}
        {@const prevEntry = filteredEntries[i - 1]}
        {@const isNewGroup = !prevEntry ||
          (entry.entryType === 'user' && prevEntry.entryType !== 'user') ||
          (entry.entryType === 'assistant' && prevEntry.entryType !== 'thinking' && prevEntry.entryType !== 'assistant') ||
          (entry.entryType === 'thinking' && prevEntry.entryType !== 'thinking')}
        {#if isNewGroup && i > 0}
          <div class="separator"></div>
        {/if}
        <div class="entry-wrapper">
          <JournalEntry {entry} />
        </div>
      {/each}

      {#if agent.status === 'working'}
        <TypingIndicator label={typingLabel} />
      {/if}

      {#if agent.status === 'input' && agent.pendingApproval}
        <div class="approval-banner">
          <span class="approval-icon">⏳</span>
          <span class="approval-text">{agent.pendingApproval}</span>
        </div>
      {/if}
    {/if}
  </div>

  <CommandInput sessionId={agent.sessionId} agentName={agent.project} />
</div>

<style>
  .central-panel { display: flex; flex-direction: column; height: 100%; }
  .header {
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .left { display: flex; align-items: center; gap: 8px; }
  .name { font-weight: 600; font-size: 14px; }
  .status {
    padding: 2px 8px;
    border-radius: 8px;
    font-size: 11px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .status.working { background: var(--green-dim); color: var(--green); }
  .status.input { background: var(--amber-dim); color: var(--amber); }
  .status.idle { background: var(--bg-idle); color: var(--text-muted); }
  .status-dot-anim {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--green);
    animation: blink 1.2s ease-in-out infinite;
  }
  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
  .meta { font-size: 12px; color: var(--text-dim); }
  .level-toggle {
    display: flex;
    gap: 2px;
    background: var(--bg-overlay);
    border-radius: 6px;
    padding: 2px;
  }
  .level-toggle button {
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 12px;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s;
  }
  .level-toggle button.active {
    background: var(--bg-active-toggle);
    color: var(--blue);
  }
  .log {
    flex: 1;
    overflow-y: auto;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .raw-log {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
    white-space: pre-wrap;
  }
  .separator {
    height: 1px;
    background: var(--border);
    margin: 8px 0;
    opacity: 0.5;
  }
  .entry-wrapper {
    animation: fadeIn 0.2s ease-out;
  }
  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .approval-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: var(--amber-dim);
    border: 1px solid var(--amber);
    border-radius: 8px;
    margin-top: 6px;
    animation: pulseGlow 2s ease-in-out infinite;
  }
  @keyframes pulseGlow {
    0%, 100% { box-shadow: none; }
    50% { box-shadow: 0 0 12px var(--pulse-glow); }
  }
  .approval-icon { font-size: 16px; }
  .approval-text { font-size: 12px; color: var(--amber); font-weight: 500; }
</style>
