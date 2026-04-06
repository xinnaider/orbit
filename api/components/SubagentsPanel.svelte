<script lang="ts">
  import type { SubagentInfo, JournalEntry } from '../lib/types';
  import { getSubagentJournal } from '../lib/tauri';
  import JournalEntryComponent from './JournalEntry.svelte';
  import ThinkingBlock from './ThinkingBlock.svelte';

  export let sessionId: string;
  export let subagents: SubagentInfo[];

  $: running = subagents.filter(a => a.status === 'running');
  $: done = subagents.filter(a => a.status === 'done');

  let modalAgent: SubagentInfo | null = null;
  let modalEntries: JournalEntry[] = [];
  let loading = false;

  async function openLog(agent: SubagentInfo) {
    modalAgent = agent;
    loading = true;
    modalEntries = [];
    try {
      modalEntries = await getSubagentJournal(sessionId, agent.id);
    } catch (e) {
      console.error('Failed to load subagent journal', e);
    }
    loading = false;
  }

  function closeModal() {
    modalAgent = null;
    modalEntries = [];
  }

  function handleBackdropClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('modal-backdrop')) {
      closeModal();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') closeModal();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="subagents">
  {#if subagents.length === 0}
    <p class="empty">No sub-agents spawned</p>
  {:else}
    {#if running.length > 0}
      <div class="section">
        <div class="section-header">
          <span class="dot running"></span> Running ({running.length})
        </div>
        {#each running as agent}
          <button class="agent-row running-row" onclick={() => openLog(agent)}>
            <div class="agent-type">{agent.agentType} <span class="status-dot">●</span></div>
            {#if agent.description}
              <div class="agent-desc">{agent.description}</div>
            {/if}
            <div class="view-hint">Click to view log</div>
          </button>
        {/each}
      </div>
    {/if}

    {#if done.length > 0}
      <div class="section">
        <div class="section-header">
          <span class="dot done"></span> Completed ({done.length})
        </div>
        {#each done as agent}
          <button class="agent-row" onclick={() => openLog(agent)}>
            <div class="agent-type">{agent.agentType} <span class="check">✓</span></div>
            {#if agent.description}
              <div class="agent-desc">{agent.description}</div>
            {/if}
            <div class="view-hint">Click to view log</div>
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>

{#if modalAgent}
  <div class="modal-backdrop" onclick={handleBackdropClick} onkeydown={(e) => e.key === 'Escape' && (modalAgent = null)} role="dialog" tabindex="-1">
    <div class="modal">
      <div class="modal-header">
        <div class="modal-title">
          <span class="modal-type">{modalAgent.agentType}</span>
          <span class="modal-status" class:done={modalAgent.status === 'done'} class:running-status={modalAgent.status === 'running'}>
            {modalAgent.status}
          </span>
        </div>
        {#if modalAgent.description}
          <div class="modal-desc">{modalAgent.description}</div>
        {/if}
        <button class="modal-close" onclick={closeModal}>✕</button>
      </div>
      <div class="modal-body">
        {#if loading}
          <p class="loading">Loading log...</p>
        {:else if modalEntries.length === 0}
          <p class="loading">No log entries found</p>
        {:else}
          {#each modalEntries as entry}
            <JournalEntryComponent {entry} />
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .subagents { padding: 10px; }
  .empty { color: var(--text-dim); font-size: 13px; text-align: center; padding: 20px; }
  .section { margin-bottom: 12px; }
  .section-header {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 6px;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    display: inline-block;
  }
  .dot.running { background: var(--amber); }
  .dot.done { background: var(--green); }
  .agent-row {
    display: block;
    width: 100%;
    text-align: left;
    padding: 8px 10px;
    border-radius: 6px;
    margin-bottom: 3px;
    background: var(--bg-overlay);
    border: 1px solid transparent;
    cursor: pointer;
    transition: border-color 0.15s;
  }
  .agent-row:hover {
    border-color: var(--border);
  }
  .agent-row.running-row {
    border-left: 2px solid var(--amber);
  }
  .agent-type {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
  }
  .check { color: var(--green); }
  .status-dot { color: var(--amber); font-size: 10px; }
  .agent-desc {
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 2px;
    line-height: 1.4;
  }
  .view-hint {
    font-size: 10px;
    color: var(--text-dim);
    margin-top: 3px;
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .modal {
    width: 70vw;
    max-width: 900px;
    max-height: 80vh;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .modal-header {
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
    position: relative;
  }
  .modal-title {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .modal-type {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .modal-status {
    font-size: 11px;
    padding: 1px 8px;
    border-radius: 8px;
    font-weight: 500;
  }
  .modal-status.done { background: var(--green-dim); color: var(--green); }
  .modal-status.running-status { background: var(--amber-dim); color: var(--amber); }
  .modal-desc {
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 4px;
  }
  .modal-close {
    position: absolute;
    top: 12px;
    right: 14px;
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
    padding: 4px;
  }
  .modal-close:hover { color: var(--text-primary); }
  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 12px 18px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .loading {
    color: var(--text-dim);
    font-size: 13px;
    text-align: center;
    padding: 30px;
  }
</style>
