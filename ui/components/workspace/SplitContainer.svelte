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
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
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
    position: relative;
    background: var(--bd);
    z-index: 2;
  }

  /* Thin visible line */
  .handle-horizontal {
    width: 1px;
    cursor: col-resize;
  }

  .handle-vertical {
    height: 1px;
    cursor: row-resize;
  }

  /* Wider invisible hit area via pseudo-element */
  .resize-handle::after {
    content: '';
    position: absolute;
  }

  .handle-horizontal::after {
    top: 0;
    bottom: 0;
    left: -3px;
    right: -3px;
  }

  .handle-vertical::after {
    left: 0;
    right: 0;
    top: -3px;
    bottom: -3px;
  }

  .resize-handle:hover {
    background: var(--bd1);
  }
</style>
