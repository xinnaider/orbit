<script lang="ts">
  import { workspace } from '../../lib/stores/workspace';
  import type { SplitNode } from '../../lib/stores/workspace';
  import SplitContainer from './SplitContainer.svelte';
  import PaneContainer from './PaneContainer.svelte';

  $: node = $workspace.root;
</script>

<div class="workspace">
  {#if node.type === 'leaf'}
    <PaneContainer paneId={node.paneId} />
  {:else}
    <SplitContainer {node} path={[]} />
  {/if}
</div>

<style>
  .workspace {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
</style>
