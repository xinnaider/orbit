<script lang="ts">
  import { onMount } from 'svelte';
  import {
    ChevronDown,
    ChevronRight,
    FileDiff,
    GitBranch,
    RefreshCw,
    Tag,
  } from 'lucide-svelte';
  import MonacoDiffViewer from './MonacoDiffViewer.svelte';
  import PanelHeader from './workspace/PanelHeader.svelte';
  import TreeNode from './GitTreeNode.svelte';
  import { gitDiffFile, gitOverview, type GitDiffFile, type GitFileChange, type GitOverview } from '../lib/tauri/git';
  import { buildGitTree, filterGitFiles } from '../lib/git-tree';
  import {
    applyTagToFiles,
    FIXED_GIT_TAGS,
    loadGitTags,
    saveGitTags,
    tagKey,
    tagsByFileId,
    type FixedGitTag,
  } from '../lib/git-tags';

  export let cwd: string;
  export let paneId = '';
  export let onClose: (() => void) | null = null;
  export let focused: boolean = true;

  let overview: GitOverview | null = null;
  let loading = false;
  let error = '';
  let query = '';
  let selectedFile: GitFileChange | null = null;
  let selectedIds = new Set<string>();
  let expanded = new Set<string>();
  let tags: Record<string, string[]> = {};
  let diff: GitDiffFile | null = null;
  let diffLoading = false;
  let diffError = '';

  $: files = overview?.files ?? [];
  $: fileTags = tagsByFileId(files, tags);
  $: filteredFiles = filterGitFiles(files, query, fileTags);
  $: tree = buildGitTree(filteredFiles);
  $: stagedCount = files.filter((file) => file.group === 'staged').length;

  async function refresh() {
    loading = true;
    error = '';
    try {
      overview = await gitOverview(cwd);
      tags = loadGitTags(cwd, overview.files);
      selectedFile = overview.files[0] ?? null;
      selectedIds = new Set();
      expandInitialGroups(overview.files);
      if (selectedFile) await loadDiff(selectedFile);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function expandInitialGroups(nextFiles: GitFileChange[]) {
    const ids = new Set<string>();
    for (const file of nextFiles) {
      ids.add(file.group);
    }
    expanded = ids;
  }

  async function loadDiff(file: GitFileChange) {
    selectedFile = file;
    diffLoading = true;
    diffError = '';
    try {
      diff = await gitDiffFile(cwd, file);
    } catch (e) {
      diffError = e instanceof Error ? e.message : String(e);
      diff = null;
    } finally {
      diffLoading = false;
    }
  }

  function toggleExpanded(id: string) {
    const next = new Set(expanded);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expanded = next;
  }

  function toggleSelected(file: GitFileChange) {
    const next = new Set(selectedIds);
    if (next.has(file.id)) next.delete(file.id);
    else next.add(file.id);
    selectedIds = next;
  }

  function tagSelected(tag: FixedGitTag) {
    const selectedFiles = files.filter((file) => selectedIds.has(file.id));
    const targetFiles = selectedFiles.length > 0 ? selectedFiles : selectedFile ? [selectedFile] : [];
    if (targetFiles.length === 0) return;
    tags = applyTagToFiles(tags, targetFiles, tag);
    saveGitTags(cwd, tags);
  }

  onMount(() => {
    refresh();
  });
</script>

<section class="git-panel">
  <PanelHeader
    title={overview?.branch ?? 'Git'}
    status={overview?.branch ? 'main' : null}
    dragPayload={JSON.stringify({ sourcePaneId: paneId, target: { kind: 'git', cwd } })}
    {onClose}
    {focused}
  >
    <div slot="leading" class="git-header-icon">
      <GitBranch size={12} />
    </div>
    <div slot="meta" class="git-header-right">
      <span class="hdr-stat">{files.length} files changed</span>
      <div class="hdr-divider"></div>
      <span class="hdr-stat add">+{files.filter(f => f.group === 'staged' || f.path.endsWith('.ts')).length}</span>
      <span class="hdr-stat del">-{files.filter(f => f.group === 'unstaged' && !f.path.endsWith('.ts')).length}</span>
    </div>
    <button slot="actions" type="button" class="hdr-action" aria-label="Refresh Git status" on:click={refresh}>
      <RefreshCw size={11} />
    </button>
  </PanelHeader>

  {#if loading}
    <div class="state">Loading Git status...</div>
  {:else if error}
    <div class="state error">{error}</div>
  {:else}
    <div class="git-body">
      <aside class="tree-pane">
        <div class="tree-tools">
          <input bind:value={query} placeholder="Search files or tags..." aria-label="Search Git files or tags" />
          <div class="tag-actions">
            <Tag size={12} />
            {#each FIXED_GIT_TAGS as tag}
              <button type="button" on:click={() => tagSelected(tag)}>{tag}</button>
            {/each}
          </div>
        </div>

        <div class="selection-bar">
          <span>{selectedIds.size} selected</span>
          <button type="button" on:click={() => (selectedIds = new Set())}>Clear</button>
        </div>

        <div class="tree" aria-label="Changed files">
          {#each tree as group (group.group)}
          <div
            class="group-row"
            role="button"
            tabindex="0"
            on:click={() => toggleExpanded(group.group)}
            on:keydown={(e) => e.key === 'Enter' && toggleExpanded(group.group)}
          >
              {#if expanded.has(group.group)}
                <ChevronDown size={13} />
              {:else}
                <ChevronRight size={13} />
              {/if}
              <span>{group.label}</span>
              <span class="count">{group.count}</span>
            </div>
            {#if expanded.has(group.group)}
              {#each group.children as node (node.id)}
                <TreeNode
                  {node}
                  depth={1}
                  {expanded}
                  {selectedIds}
                  {selectedFile}
                  {fileTags}
                  onToggleExpanded={toggleExpanded}
                  onToggleSelected={toggleSelected}
                  onSelectFile={loadDiff}
                />
              {/each}
            {/if}
          {/each}
        </div>
      </aside>

      <main class="diff-pane">
        <div class="diff-header">
          <FileDiff size={13} />
          <span class="diff-path">{selectedFile?.path ?? 'No file selected'}</span>
          {#if selectedFile}
            <span class="pill">{selectedFile.group}</span>
          {/if}
          {#each selectedFile ? tags[tagKey(selectedFile)] ?? [] : [] as tag}
            <span class="tag-pill">{tag}</span>
          {/each}
        </div>
        <div class="diff-body">
          {#if diffLoading}
            <div class="state">Loading diff...</div>
          {:else if diffError}
            <div class="state error">{diffError}</div>
          {:else if diff?.binary}
            <div class="state">Binary diff is not available.</div>
          {:else if diff}
            <MonacoDiffViewer original={diff.original} modified={diff.modified} language={diff.language} />
          {:else}
            <div class="state">Select a file to view its diff.</div>
          {/if}
        </div>
      </main>
    </div>
  {/if}
</section>

<style>
  .git-panel {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    background: var(--bg);
  }

  .diff-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    border-bottom: 1px solid var(--bd);
    background: var(--bg2);
    font-family: var(--mono);
    font-size: 10px;
    flex-shrink: 0;
  }

  .diff-path {
    overflow: hidden;
    color: var(--ac);
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .git-header-icon {
    display: flex;
    color: var(--t1);
    flex-shrink: 0;
  }

  .git-header-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .hdr-stat {
    font-family: var(--mono);
    font-size: 9.5px;
    color: var(--t2);
    font-variant-numeric: tabular-nums;
  }
  .hdr-stat.add { color: var(--ac); }
  .hdr-stat.del { color: var(--s-error); }

  .hdr-divider {
    width: 1px;
    height: 12px;
    background: var(--bd);
    flex-shrink: 0;
  }

  .hdr-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--t3);
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
  }
  .hdr-action:hover {
    background: var(--bg3);
    color: var(--t1);
  }

  .git-body {
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr);
    flex: 1;
    min-height: 0;
  }

  .tree-pane {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    border-right: 1px solid var(--bd);
  }

  .tree-tools {
    padding: 6px 8px;
    border-bottom: 1px solid var(--bd);
  }

  .tree-tools input {
    width: 100%;
    height: 24px;
    border: 1px solid var(--bd);
    border-radius: var(--radius-sm);
    background: var(--bg1);
    color: var(--t2);
    padding: 0 8px;
    font-family: var(--mono);
    font-size: 10px;
    outline: none;
    transition: border-color 0.15s;
  }

  .tree-tools input:focus {
    border-color: color-mix(in srgb, var(--ac), transparent 50%);
  }

  .tag-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 6px;
  }
  .tag-actions button {
    border: 1px solid var(--bd);
    border-radius: 3px;
    padding: 1px 5px;
    background: transparent;
    color: var(--t3);
    cursor: pointer;
    font-family: var(--mono);
    font-size: 8.5px;
    transition: all 0.1s;
  }
  .tag-actions button:hover {
    border-color: color-mix(in srgb, var(--ac), transparent 70%);
    background: var(--ac-d2);
    color: var(--t1);
  }

  .selection-bar {
    display: flex;
    align-items: center;
    height: 28px;
    padding: 0 10px;
    border-bottom: 1px solid var(--bd);
    color: var(--t2);
    font-size: 9.5px;
    font-family: var(--mono);
  }

  .selection-bar button {
    margin-left: auto;
    padding: var(--sp-1) var(--sp-3);
  }

  .tree {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 4px 0;
    color: var(--t2);
    font-family: var(--mono);
    font-size: 10px;
  }

  .group-row {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 10px;
    color: var(--t1);
    cursor: pointer;
    font-family: var(--mono);
    font-size: 8.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--t3);
  }

  .group-row:hover {
    color: var(--t2);
  }

  .count {
    margin-left: auto;
    font-family: var(--mono);
    font-size: 9px;
    font-variant-numeric: tabular-nums;
  }

  .selection-bar button {
    margin-left: auto;
    border: none;
    background: transparent;
    color: var(--t3);
    cursor: pointer;
    font-family: var(--mono);
    font-size: 9px;
  }
  .selection-bar button:hover {
    color: var(--t1);
  }

  .diff-pane {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
  }

  .diff-body {
    flex: 1;
    min-height: 0;
  }

  .state {
    display: grid;
    height: 100%;
    place-items: center;
    color: var(--t2);
    font-size: var(--sm);
  }

  .state.error {
    color: var(--s-error);
  }
</style>
