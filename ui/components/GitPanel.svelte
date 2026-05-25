<script lang="ts">
  import { onMount } from 'svelte';
  import { FileDiff, List, FolderTree, Pencil, Tag, Timer, TimerOff } from 'lucide-svelte';
  import MonacoDiffViewer from './MonacoDiffViewer.svelte';
  import PanelHeader from './workspace/PanelHeader.svelte';
  import TreeNode from './GitTreeNode.svelte';
  import GitFlatList from './GitFlatList.svelte';
  import {
    gitDiffFile,
    gitOverview,
    writeFileContent,
    type GitDiffFile,
    type GitFileChange,
    type GitOverview,
  } from '../lib/tauri/git';
  import { buildFlatTree, filterGitFiles } from '../lib/git-tree';
  import {
    applyTagToFiles,
    removeTagFromFiles,
    FIXED_GIT_TAGS,
    TAG_COLORS,
    loadGitTags,
    saveGitTags,
    tagKey,
    tagsByFileId,
    type FixedGitTag,
  } from '../lib/git-tags';

  export let cwd: string;
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
  let editMode = false;
  let treeCollapsed = false;
  let saving = false;
  let saveMessage = '';
  let dirtyAfterSave = false;
  let diffViewer: any = undefined;
  let autoSave = false;
  let viewMode: 'flat' | 'tree' = 'flat';

  $: files = overview?.files ?? [];
  $: fileTags = tagsByFileId(files, tags);
  $: filteredFiles = filterGitFiles(files, query, fileTags);
  $: tree = buildFlatTree(filteredFiles);
  $: totalChanged = files.length;
  $: activeDiffLoaded = !!selectedFile && !!diff && diff.id === selectedFile.id;

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
    // Expand all top-level folders
    const paths = new Set(nextFiles.map((f) => f.path.split('/').slice(0, -1).join('/')));
    const ids = new Set<string>();
    for (const p of paths) {
      if (p) ids.add(`all:${p}`);
    }
    expanded = ids;
  }

  async function loadDiff(file: GitFileChange) {
    selectedFile = file;
    editMode = false;
    diffLoading = true;
    diffError = '';
    try {
      const result = await gitDiffFile(cwd, file, overview?.statusOutput);
      if (selectedFile?.id === file.id) diff = result;
    } catch (e) {
      if (selectedFile?.id === file.id) {
        diffError = e instanceof Error ? e.message : String(e);
      }
    } finally {
      if (selectedFile?.id === file.id) diffLoading = false;
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

  async function handleSave(e: CustomEvent<{ content: string; auto: boolean }>) {
    if (!selectedFile || !diff) return;
    const filePath = `${cwd}/${selectedFile.path}`;
    saving = true;
    saveMessage = e.detail.auto ? 'Auto-saving...' : 'Saving...';
    try {
      await writeFileContent(filePath, e.detail.content);
      saveMessage = e.detail.auto ? 'Auto-saved' : 'Saved';
      dirtyAfterSave = false;
      diffViewer?.markSaved();
      // Update diff so original matches saved content — diff shows no changes
      diff = { ...diff, original: e.detail.content, modified: e.detail.content };
      setTimeout(() => (saveMessage = ''), 2000);
    } catch (err) {
      saveMessage = `Error: ${err instanceof Error ? err.message : String(err)}`;
      setTimeout(() => (saveMessage = ''), 4000);
    } finally {
      saving = false;
    }
  }

  function handleDirty(e: CustomEvent<{ dirty: boolean }>) {
    dirtyAfterSave = e.detail.dirty;
  }

  function toggleTree() {
    treeCollapsed = !treeCollapsed;
  }

  function setViewMode(mode: 'flat' | 'tree') {
    viewMode = mode;
    try {
      localStorage.setItem('orbit:gitViewMode', mode);
    } catch {
      /* ignore localStorage errors */
    }
  }

  function tagSelected(tag: FixedGitTag) {
    const selectedFiles = files.filter((file) => selectedIds.has(file.id));
    const targetFiles =
      selectedFiles.length > 0 ? selectedFiles : selectedFile ? [selectedFile] : [];
    if (targetFiles.length === 0) return;

    // Toggle: if ALL target files already have this tag, remove it instead
    const allHaveTag = targetFiles.every((f) => (tags[tagKey(f)] ?? []).includes(tag));
    if (allHaveTag && targetFiles.length > 0) {
      tags = removeTagFromFiles(tags, targetFiles, tag);
    } else {
      tags = applyTagToFiles(tags, targetFiles, tag);
    }
    saveGitTags(cwd, tags);
  }

  function clearSelection() {
    // Clear tags from selected (checked) files, or from the current file if none checked
    const checkedFiles = files.filter((file) => selectedIds.has(file.id));
    const targetFiles = checkedFiles.length > 0 ? checkedFiles : selectedFile ? [selectedFile] : [];
    if (targetFiles.length > 0) {
      let changed = false;
      for (const file of targetFiles) {
        const key = tagKey(file);
        if (tags[key]) {
          delete tags[key];
          changed = true;
        }
      }
      if (changed) {
        tags = { ...tags };
        saveGitTags(cwd, tags);
      }
    }
    // Clear checkbox selection
    selectedIds = new Set();
  }

  onMount(() => {
    try {
      const saved = localStorage.getItem('orbit:gitViewMode');
      if (saved === 'tree' || saved === 'flat') viewMode = saved;
    } catch {
      /* ignore localStorage errors */
    }
    refresh();
  });
</script>

<section class="git-panel" data-testid="git-panel">
  <PanelHeader
    title={overview?.branch ?? 'Git'}
    meta={totalChanged > 0 ? `${totalChanged} file${totalChanged !== 1 ? 's' : ''}` : null}
    {onClose}
    {focused}
  />

  {#if loading}
    <div class="state">Loading Git status...</div>
  {:else if error}
    <div class="state error">{error}</div>
  {:else}
    <div class="git-body" class:tree-hidden={treeCollapsed}>
      <aside class="tree-pane" class:hidden={treeCollapsed}>
        <div class="tree-tools">
          <input
            bind:value={query}
            placeholder="Search files or tags..."
            aria-label="Search Git files or tags"
            data-testid="git-search-input"
          />
          <div class="tag-actions">
            <Tag size={12} />
            {#each FIXED_GIT_TAGS as tag}
              {@const color = TAG_COLORS[tag] ?? '#666'}
              <button type="button" style="--tag-color:{color}" on:click={() => tagSelected(tag)}
                >{tag}</button
              >
            {/each}
          </div>
        </div>

        <div class="selection-bar">
          <span>{selectedIds.size} selected</span>
          <button type="button" on:click={clearSelection}>Clear</button>
        </div>

        <div class="tree" aria-label="Changed files">
          {#if viewMode === 'tree'}
            {#each tree as node (node.id)}
              <TreeNode
                {node}
                depth={0}
                {expanded}
                {selectedIds}
                {selectedFile}
                {fileTags}
                onToggleExpanded={toggleExpanded}
                onToggleSelected={toggleSelected}
                onSelectFile={loadDiff}
              />
            {/each}
          {:else}
            <GitFlatList
              files={filteredFiles}
              {selectedFile}
              {selectedIds}
              {fileTags}
              onSelectFile={loadDiff}
              onToggleSelected={toggleSelected}
            />
          {/if}
          {#if (viewMode === 'tree' && tree.length === 0) || (viewMode === 'flat' && filteredFiles.length === 0)}
            <div class="tree-empty">No changes</div>
          {/if}
        </div>

        <div class="view-mode-toggle">
          <button
            type="button"
            class="view-btn"
            class:active={viewMode === 'flat'}
            aria-pressed={viewMode === 'flat'}
            title="Flat view"
            on:click={() => setViewMode('flat')}
          >
            <List size={12} />
          </button>
          <button
            type="button"
            class="view-btn"
            class:active={viewMode === 'tree'}
            aria-pressed={viewMode === 'tree'}
            title="Tree view"
            on:click={() => setViewMode('tree')}
          >
            <FolderTree size={12} />
          </button>
        </div>
      </aside>

      <main class="diff-pane">
        <div class="diff-header">
          <FileDiff size={13} />
          <span class="diff-path">{selectedFile?.path ?? 'No file selected'}</span>
          {#if selectedFile}
            <span class="pill">{selectedFile.group}</span>
          {/if}
          {#each selectedFile ? (tags[tagKey(selectedFile)] ?? []) : [] as tag}
            {@const color = TAG_COLORS[tag] ?? '#666'}
            <span class="tag-pill" style="--tag-color:{color}">{tag}</span>
          {/each}
          {#if selectedFile && activeDiffLoaded && diff && !diff.binary}
            <div class="diff-header-actions">
              <button
                type="button"
                class="hdr-action"
                class:active={autoSave}
                title={autoSave
                  ? 'Auto-save is on'
                  : 'Enable auto-save (saves 1.5s after last change)'}
                on:click={() => (autoSave = !autoSave)}
              >
                {#if autoSave}
                  <Timer size={11} />
                {:else}
                  <TimerOff size={11} />
                {/if}
              </button>
              <button
                type="button"
                class="hdr-action"
                class:edit-toggle={true}
                class:active={editMode}
                title={editMode ? 'Disable editing (Ctrl+S to save)' : 'Enable editing'}
                on:click={() => (editMode = !editMode)}
              >
                <Pencil size={11} />
              </button>
              {#if editMode && dirtyAfterSave && !autoSave}
                <span class="save-status dirty">Unsaved changes</span>
              {:else if saving}
                <span class="save-status">{saveMessage}</span>
              {:else if saveMessage}
                <span class="save-status ok">{saveMessage}</span>
              {/if}
            </div>
          {/if}
        </div>
        <div class="diff-body" data-testid="git-diff-view">
          {#if !diff && !diffLoading}
            <div class="state">Select a file to view its diff.</div>
          {:else if diffError && !diffLoading}
            <div class="state error">{diffError}</div>
          {:else if diff?.binary}
            <div class="state">
              {activeDiffLoaded ? 'Binary diff is not available.' : 'Loading diff...'}
            </div>
          {:else}
            {#key diff?.id}
              {#if diff}
                <MonacoDiffViewer
                  bind:this={diffViewer}
                  original={diff.original}
                  modified={diff.modified}
                  language={diff.language}
                  editable={editMode}
                  {autoSave}
                  on:save={handleSave}
                  on:dirty={handleDirty}
                />
              {:else}
                <div class="state">Select a file to view its diff.</div>
              {/if}
            {/key}
          {/if}
          {#if diffLoading}
            <div class="diff-loading-overlay">
              <span>Loading diff...</span>
            </div>
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

  .tree-empty {
    padding: 16px 10px;
    color: var(--t3);
    font-size: var(--xs);
    text-align: center;
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
    transition:
      background 0.1s,
      color 0.1s;
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

  .git-body.tree-hidden {
    grid-template-columns: 0fr minmax(0, 1fr);
  }

  .git-body.tree-hidden .tree-pane {
    overflow: hidden;
    width: 0;
    min-width: 0;
    border-right: none;
  }

  /* Narrow screens: stack tree on top of diff */
  @media (max-width: 500px) {
    .git-body {
      grid-template-columns: 1fr;
      grid-template-rows: auto minmax(0, 1fr);
    }
    .git-body .tree-pane {
      max-height: 40%;
      border-right: none;
      border-bottom: 1px solid var(--bd);
    }
  }

  .tree-pane {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    border-right: 1px solid var(--bd);
    margin-right: 4px;
  }

  .tree-pane.hidden {
    margin-right: 0;
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
    border: 1px solid color-mix(in srgb, var(--tag-color, var(--bd)), transparent 70%);
    border-radius: 3px;
    padding: 1px 5px;
    background: transparent;
    color: var(--tag-color, var(--t3));
    cursor: pointer;
    font-family: var(--mono);
    font-size: 8.5px;
    transition: all 0.1s;
  }
  .tag-actions button:hover {
    border-color: var(--tag-color, var(--ac));
    background: color-mix(in srgb, var(--tag-color, var(--ac)), transparent 85%);
    color: var(--tag-color, var(--t1));
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
    padding: 2px 8px;
    border: none;
    background: transparent;
    color: var(--t3);
    cursor: pointer;
    font-family: var(--mono);
    font-size: 9px;
    border-radius: 3px;
    transition:
      color 0.1s,
      background 0.1s;
  }
  .selection-bar button:hover {
    color: var(--t1);
    background: var(--bg2);
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

  .diff-pane {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
  }

  .diff-body {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
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

  .tag-pill {
    display: inline-block;
    padding: 0 5px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--tag-color, #666), transparent 80%);
    color: var(--tag-color, #666);
    font-size: 8.5px;
    font-weight: 500;
    border: 1px solid color-mix(in srgb, var(--tag-color, #666), transparent 70%);
    line-height: 1.6;
  }

  .diff-header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
  }

  .diff-loading-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: flex-start;
    justify-content: flex-end;
    padding: 10px;
    pointer-events: none;
    background: color-mix(in srgb, var(--bg), transparent 82%);
  }

  .diff-loading-overlay span {
    border: 1px solid var(--bd);
    border-radius: var(--radius-sm);
    background: var(--bg2);
    color: var(--t2);
    padding: 4px 7px;
    font-family: var(--mono);
    font-size: 9px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.18);
  }

  .edit-toggle.active {
    color: var(--ac);
    background: var(--ac-d2);
  }

  .save-status {
    font-size: 9px;
    color: var(--t2);
  }
  .save-status.ok {
    color: var(--ac);
  }
  .save-status.dirty {
    color: var(--s-input);
    font-weight: 500;
  }

  .view-mode-toggle {
    display: flex;
    align-items: center;
    gap: 1px;
    padding: 4px 8px;
    border-top: 1px solid var(--bd);
    flex-shrink: 0;
  }

  .view-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 22px;
    border: none;
    background: transparent;
    color: var(--t3);
    cursor: pointer;
    border-radius: var(--radius-sm);
    transition:
      background 0.1s,
      color 0.1s;
  }

  .view-btn:hover {
    background: var(--bg3);
    color: var(--t1);
  }

  .view-btn.active {
    color: var(--ac);
    background: var(--ac-d2);
  }

  :global(html[data-glass-chrome='true']) .diff-header {
    border-bottom-color: var(--glass-border-subtle);
    background: var(--glass-bg-subtle);
    backdrop-filter: blur(var(--glass-blur));
    -webkit-backdrop-filter: blur(var(--glass-blur));
    font-size: 9px;
    color: color-mix(in srgb, var(--t0), transparent 55%);
  }

  :global(html[data-glass-chrome='true']) .tree-pane {
    border-right-color: var(--glass-border);
    background: var(--glass-bg-tree);
    backdrop-filter: blur(var(--glass-blur));
    -webkit-backdrop-filter: blur(var(--glass-blur));
  }

  :global(html[data-glass-chrome='true']) .tree-tools {
    border-bottom-color: var(--glass-border-subtle);
  }

  :global(html[data-glass-chrome='true']) .tree-tools input {
    border-color: var(--glass-border);
    background: var(--glass-bg);
  }

  :global(html[data-glass-chrome='true']) .selection-bar {
    border-bottom-color: var(--glass-border-subtle);
  }
</style>
