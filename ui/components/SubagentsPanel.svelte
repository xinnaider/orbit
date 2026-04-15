<script lang="ts">
  import type { SubagentInfo, JournalEntry } from '../lib/types';
  import { getSubagentJournal } from '../lib/tauri';
  import Feed from './Feed.svelte';

  export let sessionId: number;
  export let subagents: SubagentInfo[];
  export let refreshing = false;
  export let onRefresh: (() => void) | null = null;

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
    if ((e.target as HTMLElement).classList.contains('modal-backdrop')) closeModal();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') closeModal();
  }

  function icon(status: string) {
    if (status === 'done') return '✓';
    if (status === 'running') return '▸';
    return '○';
  }

  function cls(status: string) {
    if (status === 'done') return 'done';
    if (status === 'running') return 'active';
    return 'pending';
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="agents">
  <div class="agents-header">
    <span class="count">{subagents.length} agent{subagents.length !== 1 ? 's' : ''}</span>
    {#if onRefresh}
      <button class="refresh-btn" on:click={onRefresh} disabled={refreshing} title="Refresh">
        {refreshing ? '…' : '↺'}
      </button>
    {/if}
  </div>

  {#if subagents.length === 0}
    <p class="empty">no sub-agents</p>
  {:else}
    {#each subagents as agent}
      <button class="agent {cls(agent.status)}" on:click={() => openLog(agent)}>
        <span class="agent-icon">{icon(agent.status)}</span>
        <span class="agent-name">{agent.description || agent.agentType}</span>
      </button>
    {/each}
  {/if}
</div>

{#if modalAgent}
  <div
    class="modal-backdrop"
    on:click={handleBackdropClick}
    on:keydown={handleKeydown}
    role="dialog"
    tabindex="-1"
  >
    <div class="modal">
      <div class="modal-header">
        <div class="header-left">
          <span
            class="dot-status"
            class:dot-done={modalAgent.status === 'done'}
            class:dot-running={modalAgent.status === 'running'}>●</span
          >
          <span class="modal-type">{modalAgent.agentType}</span>
          <span
            class="modal-status"
            class:status-done={modalAgent.status === 'done'}
            class:status-running={modalAgent.status === 'running'}
          >
            {modalAgent.status}
          </span>
        </div>
        <button class="modal-close" on:click={closeModal}>×</button>
      </div>
      {#if modalAgent.description}
        <div class="desc-strip">
          <span class="desc-icon">⊙</span>
          <span class="desc-text">{modalAgent.description}</span>
        </div>
      {/if}
      <div class="modal-body">
        {#if loading}
          <p class="loading">loading log…</p>
        {:else if modalEntries.length === 0}
          <p class="loading">no log entries</p>
        {:else}
          <Feed entries={modalEntries} status={modalAgent.status} />
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .agents {
    padding: var(--sp-5) var(--sp-6);
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  .agents-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--sp-1);
  }

  .count {
    font-size: var(--xs);
    color: var(--t3);
    letter-spacing: 0.06em;
  }

  .refresh-btn {
    background: none;
    border: none;
    color: var(--t2);
    font-size: 11px;
    padding: var(--sp-1) var(--sp-2);
    cursor: pointer;
    line-height: 1;
    transition: color 0.15s;
  }
  .refresh-btn:hover {
    color: var(--t0);
  }
  .refresh-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .empty {
    font-size: var(--xs);
    color: var(--t3);
    padding: var(--sp-2) 0;
  }

  .agent {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-3);
    padding: var(--sp-2) 0;
    background: none;
    border: none;
    width: 100%;
    text-align: left;
    cursor: pointer;
    font-family: inherit;
  }
  .agent:hover .agent-name {
    color: var(--t0);
  }

  .agent-icon {
    font-size: var(--xs);
    flex-shrink: 0;
    margin-top: 1px;
  }
  .done .agent-icon {
    color: var(--s-working);
  }
  .active .agent-icon {
    color: var(--s-input);
  }
  .pending .agent-icon {
    color: var(--t3);
  }

  .agent-name {
    font-size: var(--xs);
    color: var(--t1);
    line-height: 1.4;
    transition: color 0.1s;
  }
  .done .agent-name {
    color: var(--t2);
  }
  .active .agent-name {
    color: var(--t0);
  }

  /* ── modal ────────────────────────────────────────────────────────────── */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .modal {
    width: 70vw;
    max-width: 900px;
    height: 75vh;
    background: var(--bg);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-4) var(--sp-7);
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
    background: var(--bg1);
  }
  .header-left {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    min-width: 0;
    flex: 1;
    overflow: hidden;
  }
  .dot-status {
    font-size: 8px;
    line-height: 1;
    flex-shrink: 0;
    color: var(--t3);
  }
  .dot-done {
    color: var(--s-working);
  }
  .dot-running {
    color: var(--s-input);
  }

  .modal-type {
    font-size: 12px;
    font-weight: 500;
    color: var(--t0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .modal-status {
    font-size: 10px;
    color: var(--t2);
    letter-spacing: 0.04em;
    flex-shrink: 0;
  }
  .status-done {
    color: var(--s-working);
  }
  .status-running {
    color: var(--s-input);
  }

  .modal-close {
    background: var(--bg3);
    border: 1px solid var(--bd1);
    color: var(--t2);
    width: 18px;
    height: 18px;
    border-radius: var(--radius-sm);
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-family: var(--mono);
    line-height: 1;
    transition:
      border-color 0.15s,
      color 0.15s;
    flex-shrink: 0;
  }
  .modal-close:hover {
    border-color: var(--s-error);
    color: var(--s-error);
  }

  .desc-strip {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-1) var(--sp-7);
    border-bottom: 1px solid var(--bd);
    background: var(--bg1);
    flex-shrink: 0;
    overflow: hidden;
  }
  .desc-icon {
    font-size: 10px;
    color: var(--t3);
    flex-shrink: 0;
  }
  .desc-text {
    font-size: 10px;
    color: var(--t3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .modal-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .loading {
    color: var(--t2);
    font-size: 12px;
    text-align: center;
    padding: 30px;
  }
</style>
