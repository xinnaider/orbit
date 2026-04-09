<script lang="ts">
  import { splitLayout } from '../lib/stores/layout';
  import type { PaneId } from '../lib/stores/layout';
  import { sessions } from '../lib/stores/sessions';
  import type { Session } from '../lib/stores/sessions';
  import Pane from './Pane.svelte';

  function getSession(sessionId: number | null): Session | null {
    if (sessionId === null) return null;
    return $sessions.find((s) => s.id === sessionId) ?? null;
  }

  // Need 2 columns only when panes exist on BOTH left and right sides
  $: hasLeftCol = $splitLayout.visible.some((p) => p === 'tl' || p === 'bl');
  $: hasRightCol = $splitLayout.visible.some((p) => p === 'tr' || p === 'br');
  // Need 2 rows only when panes exist on BOTH top and bottom sides
  $: hasTopRow = $splitLayout.visible.some((p) => p === 'tl' || p === 'tr');
  $: hasBotRow = $splitLayout.visible.some((p) => p === 'bl' || p === 'br');

  $: cols = hasLeftCol && hasRightCol ? '1fr 1fr' : '1fr';
  $: rows = hasTopRow && hasBotRow ? '1fr 1fr' : '1fr';

  // Reactive map so grid positions update immediately when layout changes.
  // With 3 panes, a pane alone in its row spans both columns (full width).
  $: gridAreas = (() => {
    const v = $splitLayout.visible;
    const twoCol = hasLeftCol && hasRightCol;
    const twoRow = hasTopRow && hasBotRow;
    const result = {} as Record<PaneId, string>;

    for (const p of ['tl', 'tr', 'bl', 'br'] as PaneId[]) {
      const isRight = p === 'tr' || p === 'br';
      const isBottom = p === 'bl' || p === 'br';

      const rowStart = twoRow && isBottom ? 2 : 1;

      let colStart: number, colEnd: number;
      if (twoCol && twoRow) {
        // Lone pane in its row → span full width
        const rowPeers: PaneId[] = isBottom ? ['bl', 'br'] : ['tl', 'tr'];
        const aloneInRow = rowPeers.filter((x) => x !== p).every((x) => !v.includes(x));
        if (aloneInRow) {
          colStart = 1;
          colEnd = 3;
        } else {
          colStart = isRight ? 2 : 1;
          colEnd = colStart + 1;
        }
      } else if (twoCol) {
        colStart = isRight ? 2 : 1;
        colEnd = colStart + 1;
      } else {
        colStart = 1;
        colEnd = 2;
      }

      result[p] = `${rowStart} / ${colStart} / ${rowStart + 1} / ${colEnd}`;
    }

    return result;
  })();
</script>

<div class="grid" style="grid-template-columns:{cols};grid-template-rows:{rows}">
  {#each $splitLayout.visible as paneId (paneId)}
    <Pane
      {paneId}
      gridArea={gridAreas[paneId]}
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
