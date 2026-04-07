<script lang="ts">
  import { splitLayout } from '../lib/stores/layout';
  import { sessions } from '../lib/stores/sessions';
  import type { Session } from '../lib/stores/sessions';
  import Pane from './Pane.svelte';

  function getSession(sessionId: number | null): Session | null {
    if (sessionId === null) return null;
    return $sessions.find((s) => s.id === sessionId) ?? null;
  }

  $: hasRight = $splitLayout.visible.includes('tr') || $splitLayout.visible.includes('br');
  $: hasBottom = $splitLayout.visible.includes('bl') || $splitLayout.visible.includes('br');
  $: cols = hasRight ? '1fr 1fr' : '1fr';
  $: rows = hasBottom ? '1fr 1fr' : '1fr';
</script>

<div class="grid" style="grid-template-columns:{cols};grid-template-rows:{rows}">
  {#each $splitLayout.visible as paneId (paneId)}
    <Pane
      {paneId}
      session={getSession($splitLayout.panes[paneId])}
      focused={$splitLayout.focused === paneId}
      canClose={$splitLayout.visible.length > 1}
      atMaxPanes={$splitLayout.visible.length >= 4}
    />
  {/each}
</div>

<style>
  .grid {
    display: grid;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    gap: 1px;
    background: var(--bd);
  }
</style>
