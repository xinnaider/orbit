<script lang="ts">
  import type { JournalEntry } from '../lib/types';

  export let entries: JournalEntry[];

  interface FileImpact { path: string; added: number; removed: number; }

  $: files = computeImpact(entries);

  function computeImpact(entries: JournalEntry[]): FileImpact[] {
    const map = new Map<string, FileImpact>();
    for (const e of entries) {
      if (e.entryType === 'toolCall' && (e.tool === 'Edit' || e.tool === 'Write')) {
        const path = e.toolInput?.file_path as string | undefined;
        if (!path) continue;
        const short = path.split(/[/\\]/).slice(-2).join('/');
        const existing = map.get(short) ?? { path: short, added: 0, removed: 0 };
        if (e.linesChanged) {
          existing.added += e.linesChanged.added;
          existing.removed += e.linesChanged.removed;
        }
        map.set(short, existing);
      }
    }
    return [...map.values()].sort((a, b) => (b.added + b.removed) - (a.added + a.removed));
  }
</script>

<div class="files-impact">
  {#if files.length === 0}
    <p class="empty">No files modified yet</p>
  {:else}
    {#each files as file}
      <div class="file-row mono">
        <span class="path">{file.path}</span>
        <span class="stats">
          <span class="added">+{file.added}</span>
          <span class="removed">-{file.removed}</span>
        </span>
      </div>
    {/each}
  {/if}
</div>

<style>
  .files-impact { padding: 8px; }
  .file-row {
    display: flex;
    justify-content: space-between;
    padding: 3px 6px;
    border-radius: 4px;
    background: rgba(249, 115, 22, 0.03);
    font-size: 10px;
    margin-bottom: 2px;
  }
  .path { color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
  .stats { display: flex; gap: 4px; flex-shrink: 0; }
  .added { color: var(--green); }
  .removed { color: var(--red); }
  .empty { color: var(--text-dim); font-size: 11px; text-align: center; padding: 20px; }
</style>
