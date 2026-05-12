<script lang="ts">
  import { CheckSquare, FileText, Square } from 'lucide-svelte';
  import type { GitFileChange } from '../lib/tauri/git';
  import { STATUS_SYMBOLS, STATUS_COLORS } from '../lib/git-tree';
  import { TAG_COLORS } from '../lib/git-tags';

  export let files: GitFileChange[] = [];
  export let selectedFile: GitFileChange | null = null;
  export let selectedIds: Set<string> = new Set();
  export let fileTags: Record<string, string[]> = {};
  export let onSelectFile: (file: GitFileChange) => void = () => {};
  export let onToggleSelected: (file: GitFileChange) => void = () => {};

  $: sortedFiles = [...files].sort((a, b) => {
    const aPath = a.path.toLowerCase();
    const bPath = b.path.toLowerCase();
    return aPath.localeCompare(bPath);
  });

  function folderPath(path: string): string {
    const idx = path.lastIndexOf('/');
    return idx >= 0 ? path.slice(0, idx) : '';
  }
</script>

<div class="flat-list">
  {#each sortedFiles as file (file.id)}
    {@const fpath = folderPath(file.path)}
    <div
      class="flat-row"
      class:selected={selectedFile?.id === file.id}
      role="button"
      tabindex="0"
      on:click={() => onSelectFile(file)}
      on:keydown={(e) => e.key === 'Enter' && onSelectFile(file)}
    >
      <button
        class="check-btn"
        type="button"
        aria-label={selectedIds.has(file.id) ? 'Deselect file' : 'Select file'}
        on:click|stopPropagation={() => onToggleSelected(file)}
      >
        {#if selectedIds.has(file.id)}
          <CheckSquare size={12} />
        {:else}
          <Square size={12} />
        {/if}
      </button>

      {#if file.status}
        <span
          class="status-badge"
          style="--status-color: {STATUS_COLORS[file.status] ?? '#666'}"
          title={file.status}
          aria-label="Status: {file.status}"
        >
          {STATUS_SYMBOLS[file.status] ?? '?'}
        </span>
      {/if}

      <FileText size={13} />

      <span class="file-name">{file.fileName}</span>

      {#if fpath}
        <span class="folder-path" title={fpath}>{fpath}</span>
      {/if}

      {#if fileTags[file.id]?.length}
        <span class="tag-labels">
          {#each fileTags[file.id] as t}
            <span class="tag-label" style="--tag-color:{TAG_COLORS[t] ?? '#666'}">{t}</span>
          {/each}
        </span>
      {/if}
    </div>
  {/each}

  {#if sortedFiles.length === 0}
    <div class="empty">No changes</div>
  {/if}
</div>

<style>
  .flat-list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 2px 0;
  }

  .flat-row {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 8px;
    cursor: default;
    border-radius: 3px;
    color: var(--t2);
    font-size: var(--xs);
  }

  .flat-row:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .flat-row.selected {
    background: rgba(0, 212, 126, 0.06);
  }

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
    opacity: 0.4;
  }

  .flat-row:hover .check-btn {
    opacity: 1;
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

  .file-name {
    font-weight: 500;
    color: var(--t1);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex-shrink: 0;
    max-width: 40%;
  }

  .folder-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--t3);
    font-size: 9px;
    flex: 1;
    min-width: 30px;
  }

  .tag-labels {
    display: flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
    margin-left: auto;
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

  .empty {
    padding: 16px 10px;
    color: var(--t3);
    font-size: var(--xs);
    text-align: center;
  }
</style>
