<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getMcpStatus } from '../lib/tauri/orchestration';
  import { mcpStatusLabel, mcpStatusLevel, type McpStatus } from '../lib/mcp';

  export let compact = false;

  let status: McpStatus | null = null;
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  async function refresh() {
    try {
      status = await getMcpStatus();
    } catch {
      status = null;
    }
  }

  onMount(() => {
    void refresh();
    pollTimer = setInterval(() => void refresh(), 8000);
  });

  onDestroy(() => {
    if (pollTimer) clearInterval(pollTimer);
  });
</script>

{#if status}
  {@const level = mcpStatusLevel(status)}
  <div
    class="mcp-status"
    class:compact
    class:ready={level === 'ready'}
    class:warn={level === 'warn'}
    class:error={level === 'error'}
    title={status.binaryPath
      ? `${mcpStatusLabel(status)}\n${status.binaryPath}${status.unifiedBinary ? ` ${status.stdioArg}` : ''}`
      : mcpStatusLabel(status)}
  >
    <span class="dot" aria-hidden="true"></span>
    {#if !compact}
      <span class="label">{mcpStatusLabel(status)}</span>
    {/if}
  </div>
{/if}

<style>
  .mcp-status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: var(--mono);
    font-size: 10px;
    color: var(--t2);
    letter-spacing: 0.04em;
  }
  .mcp-status.compact {
    gap: 0;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--t3);
  }
  .ready .dot {
    background: var(--s-working);
    box-shadow: 0 0 8px color-mix(in srgb, var(--s-working), transparent 50%);
  }
  .warn .dot {
    background: var(--s-input);
  }
  .error .dot {
    background: var(--s-error);
  }
  .label {
    white-space: nowrap;
  }
</style>
