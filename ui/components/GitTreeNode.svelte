<script lang="ts">
  import {
    CheckSquare,
    ChevronDown,
    ChevronRight,
    FileText,
    Folder,
    FolderOpen,
    Minus,
    Plus,
    Square,
  } from 'lucide-svelte';
  import type { GitFileChange } from '../lib/tauri/git';
  import type { GitTreeNode as TreeNodeType } from '../lib/git-tree';
  import { TAG_COLORS } from '../lib/git-tags';
  import { STATUS_SYMBOLS, STATUS_COLORS } from '../lib/git-tree';
  import TreeNode from './GitTreeNode.svelte';

  export let node: TreeNodeType;
  export let depth: number;
  export let expanded: Set<string>;
  export let selectedIds: Set<string>;
  export let selectedFile: GitFileChange | null;
  export let fileTags: Record<string, string[]>;
  export let actionBusy = false;
  export let onToggleExpanded: (id: string) => void;
  export let onToggleSelected: (file: GitFileChange) => void;
  export let onSelectFile: (file: GitFileChange) => void;
  export let onStageFile: (file: GitFileChange) => void = () => {};
  export let onUnstageFile: (file: GitFileChange) => void = () => {};

  function hasStats(file: GitFileChange): boolean {
    return file.additions != null || file.deletions != null;
  }
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
        {actionBusy}
        {onToggleExpanded}
        {onToggleSelected}
        {onSelectFile}
        {onStageFile}
        {onUnstageFile}
      />
    {/each}
  {/if}
{:else if node.kind === 'file'}
  <div
    class="tree-row file-row"
    class:selected={selectedFile?.id === node.change.id}
    data-testid="git-file-row"
    role="button"
    tabindex="0"
    style="padding-left: {depth * 16 + 8}px"
    on:click={() => onSelectFile(node.change)}
    on:keydown={(e) => e.key === 'Enter' && onSelectFile(node.change)}
  >
    <button
      class="stage-btn"
      type="button"
      disabled={actionBusy}
      aria-label={node.change.group === 'staged' ? 'Unstage file' : 'Stage file'}
      data-testid={node.change.group === 'staged' ? 'git-unstage-file' : 'git-stage-file'}
      on:click|stopPropagation={() =>
        node.change.group === 'staged'
          ? onUnstageFile(node.change)
          : onStageFile(node.change)}
    >
      {#if node.change.group === 'staged'}
        <Minus size={11} />
      {:else}
        <Plus size={11} />
      {/if}
    </button>
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
    {#if node.change.status}
      <span
        class="status-badge"
        style="--status-color: {STATUS_COLORS[node.change.status] ?? '#666'}"
        title={node.change.status}
        aria-label="Status: {node.change.status}"
      >
        {STATUS_SYMBOLS[node.change.status] ?? '?'}
      </span>
    {/if}
    <FileText size={14} />
    <span class="node-name">{node.name}</span>
    {#if hasStats(node.change)}
      <span class="line-stats" data-testid="git-file-stats">
        {#if node.change.additions != null}
          <span class="add">+{node.change.additions}</span>
        {/if}
        {#if node.change.deletions != null}
          <span class="del">-{node.change.deletions}</span>
        {/if}
      </span>
    {/if}
    {#if fileTags[node.change.id]?.length}
      <span class="tag-labels">
        {#each fileTags[node.change.id] as t}
          <span class="tag-label" style="--tag-color:{TAG_COLORS[t] ?? '#666'}">{t}</span>
        {/each}
      </span>
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
  }

  .expand-btn,
  .check-btn,
  .stage-btn {
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
    cursor: pointer;
  }

  .stage-btn {
    color: var(--t3);
    opacity: 0.55;
  }

  .tree-row:hover .stage-btn {
    opacity: 1;
    color: var(--ac);
  }

  .stage-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
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

  .line-stats {
    display: inline-flex;
    gap: 4px;
    flex-shrink: 0;
    font-size: 9px;
    font-variant-numeric: tabular-nums;
  }

  .line-stats .add {
    color: #3ecf8e;
  }

  .line-stats .del {
    color: #f07178;
  }

  .count {
    margin-left: auto;
    color: #4a5a52;
    font-size: 10px;
    padding-right: 4px;
  }

  .tag-labels {
    display: flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
    margin-left: auto;
    padding-right: 2px;
  }
  .tag-label {
    display: inline-block;
    padding: 0 4px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--tag-color, #666), transparent 82%);
    color: var(--tag-color, #666);
    font-size: 7.5px;
    font-weight: 500;
    line-height: 1.7;
    border: 1px solid color-mix(in srgb, var(--tag-color, #666), transparent 75%);
  }

  .status-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 14px;
    font-size: 8.5px;
    font-weight: 700;
    color: var(--status-color);
    flex-shrink: 0;
    border-radius: 2px;
    background: color-mix(in srgb, var(--status-color), transparent 85%);
  }
</style>
