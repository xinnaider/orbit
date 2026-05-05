<script lang="ts">
  import {
    CheckSquare,
    ChevronDown,
    ChevronRight,
    FileText,
    Folder,
    FolderOpen,
    Square,
  } from 'lucide-svelte';
  import type { GitFileChange } from '../lib/tauri/git';
  import type { GitTreeNode as TreeNodeType } from '../lib/git-tree';
  import TreeNode from './GitTreeNode.svelte';

  export let node: TreeNodeType;
  export let depth: number;
  export let expanded: Set<string>;
  export let selectedIds: Set<string>;
  export let selectedFile: GitFileChange | null;
  export let fileTags: Record<string, string[]>;
  export let onToggleExpanded: (id: string) => void;
  export let onToggleSelected: (file: GitFileChange) => void;
  export let onSelectFile: (file: GitFileChange) => void;
</script>

{#if node.kind === 'folder'}
  <div class="tree-row folder-row" style="padding-left: {depth * 16 + 8}px">
    <button
      class="expand-btn"
      type="button"
      aria-label={expanded.has(node.id) ? 'Collapse folder' : 'Expand folder'}
      on:click|stopPropagation={() => onToggleExpanded(node.id)}
    >
      {#if expanded.has(node.id)}
        <ChevronDown size={12} />
      {:else}
        <ChevronRight size={12} />
      {/if}
    </button>
    {#if expanded.has(node.id)}
      <FolderOpen size={14} />
    {:else}
      <Folder size={14} />
    {/if}
    <span class="node-name">{node.name}</span>
    <span class="count">{node.children.filter((c) => c.kind === 'file').length}</span>
  </div>
  {#if expanded.has(node.id)}
    {#each node.children as child (child.id)}
      <TreeNode
        node={child}
        depth={depth + 1}
        {expanded}
        {selectedIds}
        {selectedFile}
        {fileTags}
        {onToggleExpanded}
        {onToggleSelected}
        {onSelectFile}
      />
    {/each}
  {/if}
{:else if node.kind === 'file'}
  <div
    class="tree-row file-row"
    class:selected={selectedFile?.id === node.change.id}
    role="button"
    tabindex="0"
    style="padding-left: {depth * 16 + 8}px"
    on:click={() => onSelectFile(node.change)}
    on:keydown={(e) => e.key === 'Enter' && onSelectFile(node.change)}
  >
    <button
      class="check-btn"
      type="button"
      aria-label={selectedIds.has(node.change.id) ? 'Deselect file' : 'Select file'}
      on:click|stopPropagation={() => onToggleSelected(node.change)}
    >
      {#if selectedIds.has(node.change.id)}
        <CheckSquare size={12} />
      {:else}
        <Square size={12} />
      {/if}
    </button>
    <FileText size={14} />
    <span class="node-name">{node.name}</span>
    {#if fileTags[node.change.id]?.length}
      <span class="tag-dot" title={fileTags[node.change.id].join(', ')}></span>
    {/if}
  </div>
{/if}

<style>
  .tree-row {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    cursor: default;
    border-radius: 4px;
    color: #6b7f75;
    font-size: var(--xs);
  }

  .tree-row:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .folder-row {
    color: #d9f7e8;
  }

  .file-row.selected {
    background: rgba(0, 212, 126, 0.06);
    color: #d9f7e8;
  }

  .expand-btn,
  .check-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border: none;
    background: transparent;
    color: inherit;
    padding: 0;
    flex-shrink: 0;
  }

  .check-btn {
    opacity: 0.4;
  }

  .tree-row:hover .check-btn {
    opacity: 1;
  }

  .node-name {
    overflow: hidden;
    flex: 1;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    margin-left: auto;
    color: #4a5a52;
    font-size: 10px;
    padding-right: 4px;
  }

  .tag-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ac, #00d47e);
    flex-shrink: 0;
    margin-right: 4px;
  }
</style>
