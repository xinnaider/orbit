<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { Plus } from 'lucide-svelte';

  export let visible: boolean = false;

  type DropPosition = 'center' | 'top' | 'bottom' | 'left' | 'right';

  const dispatch = createEventDispatcher<{
    drop: { position: DropPosition; data: string };
  }>();

  let hoverPosition: DropPosition | null = null;

  const edgeZones: DropPosition[] = ['top', 'bottom', 'left', 'right'];

  $: isEdge = hoverPosition !== null && edgeZones.includes(hoverPosition);
  $: isCenter = hoverPosition === 'center';

  function getPosition(e: DragEvent): DropPosition {
    const el = e.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const w = rect.width;
    const h = rect.height;

    const relX = x / w;
    const relY = y / h;

    if (relY < 0.25) return 'top';
    if (relY > 0.75) return 'bottom';
    if (relX < 0.25) return 'left';
    if (relX > 0.75) return 'right';
    return 'center';
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    hoverPosition = getPosition(e);
  }

  function handleDragLeave() {
    hoverPosition = null;
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    const pos = getPosition(e);
    const data = e.dataTransfer?.getData('text/plain') ?? '';
    hoverPosition = null;
    dispatch('drop', { position: pos, data });
  }
</script>

{#if visible}
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div
    class="split-drop-zone"
    on:dragover={handleDragOver}
    on:dragleave={handleDragLeave}
    on:drop={handleDrop}
  >
    {#if isEdge}
      <div class="preview preview-{hoverPosition}"></div>
    {:else if isCenter}
      <div class="tab-indicator">
        <Plus size={14} />
        <span>Add tab</span>
      </div>
    {/if}
  </div>
{/if}

<style>
  .split-drop-zone {
    position: absolute;
    inset: 0;
    z-index: 10;
  }

  /* ── Edge: split preview ── */
  .preview {
    position: absolute;
    background: rgba(0, 212, 126, 0.10);
    border: 2px solid var(--ac);
    border-radius: 4px;
    pointer-events: none;
  }

  .preview-top {
    left: 4px;
    right: 4px;
    top: 4px;
    height: 48%;
  }

  .preview-bottom {
    left: 4px;
    right: 4px;
    bottom: 4px;
    height: 48%;
  }

  .preview-left {
    top: 4px;
    bottom: 4px;
    left: 4px;
    width: 48%;
  }

  .preview-right {
    top: 4px;
    bottom: 4px;
    right: 4px;
    width: 48%;
  }

  /* ── Center: add tab indicator ── */
  .tab-indicator {
    position: absolute;
    inset: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--sp-3);
    border: 2px dashed var(--s-init);
    border-radius: var(--radius-md);
    background: rgba(72, 136, 224, 0.07);
    color: var(--s-init);
    font-size: var(--sm);
    pointer-events: none;
  }
</style>
