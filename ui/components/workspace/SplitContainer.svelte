<script lang="ts">
  import type { SplitNode } from '../../lib/stores/workspace';
  import { resizeSplit } from '../../lib/stores/workspace';
  import PaneContainer from './PaneContainer.svelte';

  export let node: SplitNode & { type: 'split' };
  export let path: number[];

  let dragging = false;
  let container: HTMLDivElement;

  function onMousedown(e: MouseEvent) {
    e.preventDefault();
    dragging = true;

    function onMousemove(ev: MouseEvent) {
      if (!dragging || !container) return;
      const rect = container.getBoundingClientRect();
      const ratio =
        node.direction === 'horizontal'
          ? (ev.clientX - rect.left) / rect.width
          : (ev.clientY - rect.top) / rect.height;
      resizeSplit(path, ratio);
    }

    function onMouseup() {
      dragging = false;
      window.removeEventListener('mousemove', onMousemove);
      window.removeEventListener('mouseup', onMouseup);
    }

    window.addEventListener('mousemove', onMousemove);
    window.addEventListener('mouseup', onMouseup);
  }

  function onDblclick() {
    resizeSplit(path, 0.5);
  }

  $: isHorizontal = node.direction === 'horizontal';
  $: firstSize = `${node.ratio * 100}%`;
  $: secondSize = `${(1 - node.ratio) * 100}%`;
</script>

<div
  class="split-container"
  class:horizontal={isHorizontal}
  class:vertical={!isHorizontal}
  bind:this={container}
>
  <!-- First child -->
  <div class="split-child" style={isHorizontal ? `width:${firstSize}` : `height:${firstSize}`}>
    {#if node.children[0].type === 'leaf'}
      <PaneContainer paneId={node.children[0].paneId} />
    {:else}
      <svelte:self node={node.children[0]} path={[...path, 0]} />
    {/if}
  </div>

  <!-- Resize handle -->
  <div
    class="resize-handle"
    class:handle-horizontal={isHorizontal}
    class:handle-vertical={!isHorizontal}
    on:mousedown={onMousedown}
    on:dblclick={onDblclick}
    role="separator"
    aria-orientation={isHorizontal ? 'vertical' : 'horizontal'}
    aria-valuenow={Math.round(node.ratio * 100)}
    aria-valuemin={15}
    aria-valuemax={85}
    tabindex="-1"
  ></div>

  <!-- Second child -->
  <div class="split-child" style={isHorizontal ? `width:${secondSize}` : `height:${secondSize}`}>
    {#if node.children[1].type === 'leaf'}
      <PaneContainer paneId={node.children[1].paneId} />
    {:else}
      <svelte:self node={node.children[1]} path={[...path, 1]} />
    {/if}
  </div>
</div>

<style>
  .split-container {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .horizontal {
    flex-direction: row;
  }

  .vertical {
    flex-direction: column;
  }

  .split-child {
    display: flex;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    flex-shrink: 0;
  }

  .resize-handle {
    flex-shrink: 0;
    background: var(--border, #333);
    transition: background 0.15s;
    z-index: 1;
  }

  .resize-handle:hover,
  .resize-handle:active {
    background: var(--accent, #4e8ef7);
  }

  .handle-horizontal {
    width: 4px;
    cursor: col-resize;
  }

  .handle-vertical {
    height: 4px;
    cursor: row-resize;
  }
</style>
