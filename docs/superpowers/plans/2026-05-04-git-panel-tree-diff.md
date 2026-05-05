# Git Panel Tree Diff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Orbit Git panel as a normal workspace panel with a status-grouped file tree, local review tags, and an inline Monaco diff viewer.

**Architecture:** Add typed Git IPC wrappers and Rust Git commands that return a flat list of changed files plus file diff pairs. The frontend derives a status/folder tree from that flat list, persists local review tags in `localStorage`, and renders a two-column `GitPanel` with the tree on the left and the selected file diff on the right. The panel uses the shared Compact Stack panel header design from `docs/superpowers/specs/2026-05-04-orbit-compact-panel-header-design.md`.

**Tech Stack:** Svelte 5, TypeScript 5.6, Tauri 2, Rust 1.85, Git CLI, Monaco Editor, lucide-svelte, localStorage, svelte-check, cargo check/clippy.

---

## Source Specs

- `docs/superpowers/specs/2026-05-04-git-panel-tree-diff-design.md`
- `docs/superpowers/specs/2026-05-04-orbit-compact-panel-header-design.md`

---

## File Structure

- Create: `ui/lib/tauri/git.ts`
- Responsibility: typed frontend wrapper for `git_overview` and `git_diff_file` Tauri commands.
- Create: `ui/lib/git-tree.ts`
- Responsibility: pure TypeScript helpers to derive a grouped folder tree from flat `GitFileChange[]` and filter by path/status/tag.
- Create: `ui/lib/git-tags.ts`
- Responsibility: localStorage-backed tag persistence scoped by repository path and file group.
- Create: `ui/components/MonacoDiffViewer.svelte`
- Responsibility: lazy Monaco diff editor wrapper sized to fill its container.
- Create: `ui/components/GitPanel.svelte`
- Responsibility: panel UI, refresh state, tree selection, file tagging, search/filter, and inline diff loading.
- Modify: `ui/components/workspace/PaneContainer.svelte`
- Responsibility: render `GitPanel` when the active tab target is Git, or add the minimal integration point if the workspace tab migration is implemented separately.
- Modify: `ui/components/workspace/TabAddMenu.svelte`
- Responsibility: expose `Git overview` action if it is not already present.
- Modify: `ui/lib/mock/tauri-mock.ts`
- Responsibility: mock Git overview and diff data for `npm run dev:mock`.
- Create: `tauri/src/commands/git.rs`
- Responsibility: run Git CLI commands, parse changed files/branches/stashes, and return correct diff pairs.
- Modify: `tauri/src/commands/mod.rs`
- Responsibility: expose the Git command module.
- Modify: `tauri/src/lib.rs`
- Responsibility: register Git commands in `tauri::generate_handler!`.
- Modify: `package.json`, `package-lock.json`
- Responsibility: add `monaco-editor` if missing.
- Modify: `CHANGELOG.md`
- Responsibility: user-facing entry for the new Git review panel.

---

## Commit Strategy

Do not commit step-by-step. Commit by implementation block after checks pass:

1. `feat: add git backend commands` after Tasks 1-2.
2. `feat: add git tree diff panel` after Tasks 3-7.
3. `docs: add git panel design and plan` only if docs are committed separately; otherwise include specs/plans in the relevant feature commit.

Before every commit, run checks manually. Do not use `--no-verify`.

---

## Task 1: Frontend Git Contract And Tree Helpers

**Files:**

- Create: `ui/lib/tauri/git.ts`
- Create: `ui/lib/git-tree.ts`
- Create: `ui/lib/git-tags.ts`

- [ ] **Step 1: Create typed Git IPC wrapper**

Create `ui/lib/tauri/git.ts`:

```ts
import { invoke } from './invoke';

export type GitChangeGroup = 'staged' | 'unstaged' | 'untracked';

export type GitFileStatus = 'modified' | 'added' | 'deleted' | 'renamed' | 'copied' | 'untracked';

export interface GitBranchInfo {
  name: string;
  fullName: string;
  kind: 'local' | 'remote';
  current: boolean;
  upstream: string | null;
  ahead: number;
  behind: number;
}

export interface GitFileChange {
  id: string;
  path: string;
  fileName: string;
  group: GitChangeGroup;
  status: GitFileStatus;
  staged: boolean;
  untracked: boolean;
  oldPath: string | null;
  additions: number | null;
  deletions: number | null;
}

export interface GitOverview {
  cwd: string;
  branch: string | null;
  upstream: string | null;
  ahead: number;
  behind: number;
  files: GitFileChange[];
  branches: GitBranchInfo[];
}

export interface GitDiffFile {
  id: string;
  path: string;
  group: GitChangeGroup;
  language: string;
  binary: boolean;
  original: string;
  modified: string;
}

export function gitOverview(cwd: string): Promise<GitOverview> {
  return invoke<GitOverview>('git_overview', { cwd });
}

export function gitDiffFile(cwd: string, file: GitFileChange): Promise<GitDiffFile> {
  return invoke<GitDiffFile>('git_diff_file', { cwd, path: file.path, group: file.group });
}
```

- [ ] **Step 2: Export Git wrappers from Tauri index**

Append to `ui/lib/tauri/index.ts`:

```ts
export * from './git';
```

- [ ] **Step 3: Create tree helper types and builder**

Create `ui/lib/git-tree.ts`:

```ts
import type { GitChangeGroup, GitFileChange } from './tauri/git';

export type GitTreeNode = GitTreeFolderNode | GitTreeFileNode;

export interface GitTreeFolderNode {
  kind: 'folder';
  id: string;
  name: string;
  path: string;
  children: GitTreeNode[];
}

export interface GitTreeFileNode {
  kind: 'file';
  id: string;
  name: string;
  path: string;
  change: GitFileChange;
}

export interface GitTreeGroup {
  group: GitChangeGroup;
  label: string;
  children: GitTreeNode[];
  count: number;
}

const GROUP_LABELS: Record<GitChangeGroup, string> = {
  staged: 'Staged',
  unstaged: 'Unstaged',
  untracked: 'Untracked',
};

export function buildGitTree(files: GitFileChange[]): GitTreeGroup[] {
  const groups: GitTreeGroup[] = (['staged', 'unstaged', 'untracked'] as const).map((group) => {
    const groupFiles = files.filter((file) => file.group === group);
    return {
      group,
      label: GROUP_LABELS[group],
      count: groupFiles.length,
      children: buildFolderTree(group, groupFiles),
    };
  });

  return groups.filter((group) => group.count > 0);
}

function buildFolderTree(group: GitChangeGroup, files: GitFileChange[]): GitTreeNode[] {
  const root: GitTreeFolderNode = { kind: 'folder', id: `${group}:`, name: '', path: '', children: [] };

  for (const file of files) {
    const parts = file.path.split('/');
    let current = root;

    for (const part of parts.slice(0, -1)) {
      const nextPath = current.path ? `${current.path}/${part}` : part;
      let folder = current.children.find(
        (child): child is GitTreeFolderNode => child.kind === 'folder' && child.path === nextPath
      );
      if (!folder) {
        folder = { kind: 'folder', id: `${group}:${nextPath}`, name: part, path: nextPath, children: [] };
        current.children.push(folder);
      }
      current = folder;
    }

    current.children.push({
      kind: 'file',
      id: file.id,
      name: file.fileName,
      path: file.path,
      change: file,
    });
  }

  sortNodes(root.children);
  return root.children;
}

function sortNodes(nodes: GitTreeNode[]): void {
  nodes.sort((a, b) => {
    if (a.kind !== b.kind) return a.kind === 'folder' ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
  for (const node of nodes) {
    if (node.kind === 'folder') sortNodes(node.children);
  }
}

export function filterGitFiles(
  files: GitFileChange[],
  query: string,
  tagsByFile: Record<string, string[]>
): GitFileChange[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return files;

  return files.filter((file) => {
    const tags = tagsByFile[file.id] ?? [];
    return (
      file.path.toLowerCase().includes(normalized) ||
      file.group.includes(normalized) ||
      file.status.includes(normalized) ||
      tags.some((tag) => tag.toLowerCase().includes(normalized))
    );
  });
}
```

- [ ] **Step 4: Create local tag persistence helper**

Create `ui/lib/git-tags.ts`:

```ts
import type { GitFileChange } from './tauri/git';

export const FIXED_GIT_TAGS = ['ready', 'needs review', 'docs', 'risky', 'generated'] as const;

export type FixedGitTag = (typeof FIXED_GIT_TAGS)[number];

function storageKey(repoPath: string): string {
  return `orbit:git-file-tags:${repoPath}`;
}

export function tagKey(file: Pick<GitFileChange, 'path' | 'group'>): string {
  return `${file.group}:${file.path}`;
}

export function loadGitTags(repoPath: string, files: GitFileChange[]): Record<string, string[]> {
  try {
    const raw = localStorage.getItem(storageKey(repoPath));
    const parsed = raw ? (JSON.parse(raw) as Record<string, string[]>) : {};
    const validKeys = new Set(files.map(tagKey));
    const tags: Record<string, string[]> = {};

    for (const [key, value] of Object.entries(parsed)) {
      if (validKeys.has(key)) tags[key] = value.filter((tag) => FIXED_GIT_TAGS.includes(tag as FixedGitTag));
    }

    saveGitTags(repoPath, tags);
    return tags;
  } catch {
    return {};
  }
}

export function saveGitTags(repoPath: string, tags: Record<string, string[]>): void {
  localStorage.setItem(storageKey(repoPath), JSON.stringify(tags));
}

export function applyTagToFiles(
  tags: Record<string, string[]>,
  files: GitFileChange[],
  tag: FixedGitTag
): Record<string, string[]> {
  const next = { ...tags };
  for (const file of files) {
    const key = tagKey(file);
    const current = next[key] ?? [];
    if (!current.includes(tag)) next[key] = [...current, tag];
  }
  return next;
}

export function tagsByFileId(files: GitFileChange[], tags: Record<string, string[]>): Record<string, string[]> {
  return Object.fromEntries(files.map((file) => [file.id, tags[tagKey(file)] ?? []]));
}
```

- [ ] **Step 5: Typecheck helper files**

Run:

```bash
npm run check
```

Expected: no errors from `ui/lib/tauri/git.ts`, `ui/lib/git-tree.ts`, or `ui/lib/git-tags.ts`.

---

## Task 2: Rust Git Commands

**Files:**

- Create: `tauri/src/commands/git.rs`
- Modify: `tauri/src/commands/mod.rs`
- Modify: `tauri/src/lib.rs`

- [ ] **Step 1: Create Rust response types and Git runner**

Create `tauri/src/commands/git.rs`:

```rust
use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchInfo {
    name: String,
    full_name: String,
    kind: String,
    current: bool,
    upstream: Option<String>,
    ahead: u32,
    behind: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileChange {
    id: String,
    path: String,
    file_name: String,
    group: String,
    status: String,
    staged: bool,
    untracked: bool,
    old_path: Option<String>,
    additions: Option<u32>,
    deletions: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitOverview {
    cwd: String,
    branch: Option<String>,
    upstream: Option<String>,
    ahead: u32,
    behind: u32,
    files: Vec<GitFileChange>,
    branches: Vec<GitBranchInfo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiffFile {
    id: String,
    path: String,
    group: String,
    language: String,
    binary: bool,
    original: String,
    modified: String,
}

fn run_git(cwd: &str, args: &[&str]) -> Result<String, String> {
    if !Path::new(cwd).exists() {
        return Err(format!("Directory does not exist: {cwd}"));
    }

    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("failed to run git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() { "git command failed".to_string() } else { stderr });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

- [ ] **Step 2: Add branch parser helpers**

Append to `tauri/src/commands/git.rs`:

```rust
fn parse_ahead_behind(line: &str) -> (u32, u32) {
    let mut ahead = 0;
    let mut behind = 0;

    let Some(start) = line.find('[') else { return (ahead, behind) };
    let Some(end) = line[start..].find(']') else { return (ahead, behind) };

    for part in line[start + 1..start + end].split(',').map(str::trim) {
        if let Some(value) = part.strip_prefix("ahead ") {
            ahead = value.parse().unwrap_or(0);
        } else if let Some(value) = part.strip_prefix("behind ") {
            behind = value.parse().unwrap_or(0);
        }
    }

    (ahead, behind)
}

fn current_branch(cwd: &str) -> Result<(Option<String>, Option<String>, u32, u32), String> {
    let branch = run_git(cwd, &["branch", "--show-current"])?;
    let branch = branch.trim();
    let branch = if branch.is_empty() { None } else { Some(branch.to_string()) };

    let upstream = run_git(cwd, &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let status = run_git(cwd, &["status", "--short", "--branch"])?;
    let first_line = status.lines().next().unwrap_or_default();
    let (ahead, behind) = parse_ahead_behind(first_line);

    Ok((branch, upstream, ahead, behind))
}

fn branches(cwd: &str) -> Result<Vec<GitBranchInfo>, String> {
    let output = run_git(
        cwd,
        &[
            "for-each-ref",
            "--format=%(refname)%00%(refname:short)%00%(HEAD)%00%(upstream:short)%00%(ahead-behind:HEAD)",
            "refs/heads",
            "refs/remotes",
        ],
    )?;

    Ok(output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\0').collect();
            if parts.len() < 5 {
                return None;
            }

            let full_name = parts[0].to_string();
            let kind = if full_name.starts_with("refs/remotes/") { "remote" } else { "local" };
            let upstream = if parts[3].is_empty() { None } else { Some(parts[3].to_string()) };
            let mut counts = parts[4].split_whitespace();
            let ahead = counts.next().and_then(|value| value.parse().ok()).unwrap_or(0);
            let behind = counts.next().and_then(|value| value.parse().ok()).unwrap_or(0);

            Some(GitBranchInfo {
                name: parts[1].to_string(),
                full_name,
                kind: kind.to_string(),
                current: parts[2] == "*",
                upstream,
                ahead,
                behind,
            })
        })
        .collect())
}
```

- [ ] **Step 3: Add changed file parser**

Append:

```rust
fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn status_name(code: char, untracked: bool) -> String {
    if untracked {
        return "untracked".to_string();
    }
    match code {
        'A' => "added",
        'D' => "deleted",
        'R' => "renamed",
        'C' => "copied",
        _ => "modified",
    }
    .to_string()
}

fn parse_status_line(line: &str) -> Vec<GitFileChange> {
    if line.len() < 4 {
        return Vec::new();
    }

    let staged_code = line.chars().next().unwrap_or(' ');
    let worktree_code = line.chars().nth(1).unwrap_or(' ');
    let raw_path = normalize_path(&line[3..]);
    let untracked = staged_code == '?' && worktree_code == '?';
    let mut changes = Vec::new();

    if untracked {
        changes.push(make_change("untracked", '?', &raw_path, true, None));
        return changes;
    }

    if staged_code != ' ' {
        changes.push(make_change("staged", staged_code, &raw_path, false, None));
    }
    if worktree_code != ' ' {
        changes.push(make_change("unstaged", worktree_code, &raw_path, false, None));
    }

    changes
}

fn make_change(group: &str, code: char, path: &str, untracked: bool, old_path: Option<String>) -> GitFileChange {
    GitFileChange {
        id: format!("{group}:{path}"),
        path: path.to_string(),
        file_name: file_name(path),
        group: group.to_string(),
        status: status_name(code, untracked),
        staged: group == "staged",
        untracked,
        old_path,
        additions: None,
        deletions: None,
    }
}

fn changed_files(cwd: &str) -> Result<Vec<GitFileChange>, String> {
    let output = run_git(cwd, &["status", "--porcelain=v1"])?;
    Ok(output.lines().flat_map(parse_status_line).collect())
}
```

- [ ] **Step 4: Add diff helpers and commands**

Append:

```rust
fn read_git_object(cwd: &str, spec: &str) -> String {
    run_git(cwd, &["show", spec]).unwrap_or_default()
}

fn read_worktree_file(cwd: &str, path: &str) -> String {
    std::fs::read_to_string(Path::new(cwd).join(path)).unwrap_or_default()
}

fn language_for(path: &str) -> String {
    match Path::new(path).extension().and_then(|ext| ext.to_str()).unwrap_or_default() {
        "svelte" => "svelte",
        "ts" => "typescript",
        "js" => "javascript",
        "rs" => "rust",
        "json" => "json",
        "md" => "markdown",
        "css" => "css",
        _ => "plaintext",
    }
    .to_string()
}

#[tauri::command]
pub fn git_overview(cwd: String) -> Result<GitOverview, String> {
    run_git(&cwd, &["rev-parse", "--is-inside-work-tree"])?;
    let (branch, upstream, ahead, behind) = current_branch(&cwd)?;

    Ok(GitOverview {
        cwd: cwd.clone(),
        branch,
        upstream,
        ahead,
        behind,
        files: changed_files(&cwd)?,
        branches: branches(&cwd)?,
    })
}

#[tauri::command]
pub fn git_diff_file(cwd: String, path: String, group: String) -> Result<GitDiffFile, String> {
    run_git(&cwd, &["rev-parse", "--is-inside-work-tree"])?;
    let status = run_git(&cwd, &["status", "--porcelain=v1", "--", &path])?;
    let deleted = status.chars().next() == Some('D') || status.chars().nth(1) == Some('D');
    let untracked = group == "untracked";

    let (original, modified) = if untracked {
        (String::new(), read_worktree_file(&cwd, &path))
    } else if deleted {
        (read_git_object(&cwd, &format!("HEAD:{path}")), String::new())
    } else if group == "staged" {
        (read_git_object(&cwd, &format!("HEAD:{path}")), read_git_object(&cwd, &format!(":{path}")))
    } else {
        (read_git_object(&cwd, &format!(":{path}")), read_worktree_file(&cwd, &path))
    };

    Ok(GitDiffFile {
        id: format!("{group}:{path}"),
        language: language_for(&path),
        binary: false,
        path,
        group,
        original,
        modified,
    })
}
```

- [ ] **Step 5: Register Rust commands**

In `tauri/src/commands/mod.rs`, add:

```rust
pub mod git;
```

In `tauri/src/lib.rs`, add inside `tauri::generate_handler![...]`:

```rust
commands::git::git_overview,
commands::git::git_diff_file,
```

- [ ] **Step 6: Verify Rust block**

Run:

```bash
cargo fmt --manifest-path tauri/Cargo.toml
cargo check --manifest-path tauri/Cargo.toml
cargo clippy --manifest-path tauri/Cargo.toml -- -D warnings
```

Expected: all pass.

---

## Task 3: Monaco Diff Viewer

**Files:**

- Create: `ui/components/MonacoDiffViewer.svelte`
- Modify: `package.json`
- Modify: `package-lock.json`

- [ ] **Step 1: Install Monaco if missing**

Check `package.json`. If `monaco-editor` is absent, run:

```bash
npm install monaco-editor
```

Expected: `package.json` and `package-lock.json` include `monaco-editor`.

- [ ] **Step 2: Create Monaco diff wrapper**

Create `ui/components/MonacoDiffViewer.svelte`:

```svelte
<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  export let original: string;
  export let modified: string;
  export let language = 'plaintext';

  let host: HTMLDivElement;
  let editor: import('monaco-editor').editor.IStandaloneDiffEditor | null = null;
  let monaco: typeof import('monaco-editor') | null = null;

  function setModel() {
    if (!editor || !monaco) return;
    const current = editor.getModel();
    current?.original.dispose();
    current?.modified.dispose();
    editor.setModel({
      original: monaco.editor.createModel(original, language),
      modified: monaco.editor.createModel(modified, language),
    });
  }

  onMount(async () => {
    monaco = await import('monaco-editor');
    editor = monaco.editor.createDiffEditor(host, {
      automaticLayout: true,
      readOnly: true,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
    });
    setModel();
  });

  $: if (editor && monaco) setModel();

  onDestroy(() => {
    const current = editor?.getModel();
    current?.original.dispose();
    current?.modified.dispose();
    editor?.dispose();
  });
</script>

<div class="monaco-diff-host" bind:this={host}></div>

<style>
  .monaco-diff-host {
    width: 100%;
    height: 100%;
    min-height: 0;
  }
</style>
```

- [ ] **Step 3: Verify Monaco wrapper**

Run:

```bash
npm run check
```

Expected: no Svelte/TypeScript errors. Build may later warn about large Monaco chunks; that is acceptable for this feature.

---

## Task 4: Git Panel Component

**Files:**

- Create: `ui/components/GitPanel.svelte`

- [ ] **Step 1: Create component script**

Create `ui/components/GitPanel.svelte` with the script:

```svelte
<script lang="ts">
  import {
    CheckSquare,
    ChevronDown,
    ChevronRight,
    FileDiff,
    FileText,
    Folder,
    FolderOpen,
    GitBranch,
    RefreshCw,
    Square,
    Tag,
    X,
  } from 'lucide-svelte';
  import MonacoDiffViewer from './MonacoDiffViewer.svelte';
  import { gitDiffFile, gitOverview, type GitDiffFile, type GitFileChange, type GitOverview } from '../lib/tauri/git';
  import { buildGitTree, filterGitFiles, type GitTreeGroup, type GitTreeNode } from '../lib/git-tree';
  import { applyTagToFiles, FIXED_GIT_TAGS, loadGitTags, saveGitTags, tagKey, tagsByFileId, type FixedGitTag } from '../lib/git-tags';

  export let cwd: string;
  export let onClose: (() => void) | null = null;

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

  refresh();
</script>
```

- [ ] **Step 2: Add component markup**

Append markup:

```svelte
<section class="git-panel">
  <header class="git-header">
    <div class="git-header-left">
      <GitBranch size={13} />
      <span class="title">Git Overview</span>
      {#if overview?.branch}<span class="branch">{overview.branch}</span>{/if}
    </div>
    <div class="git-header-right">
      <span class="pill">{files.length} changes</span>
      <span class="pill staged">{stagedCount} staged</span>
      <button type="button" class="icon-button" aria-label="Refresh Git status" on:click={refresh}><RefreshCw size={13} /></button>
      {#if onClose}<button type="button" class="icon-button" aria-label="Close Git panel" on:click={onClose}><X size={13} /></button>{/if}
    </div>
  </header>

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

        <div class="tree" role="tree" aria-label="Changed files">
          {#each tree as group (group.group)}
            <div class="group-row" role="treeitem" tabindex="0" on:click={() => toggleExpanded(group.group)}>
              {#if expanded.has(group.group)}<ChevronDown size={13} />{:else}<ChevronRight size={13} />{/if}
              <span>{group.label}</span>
              <span class="count">{group.count}</span>
            </div>
            {#if expanded.has(group.group)}
              {#each group.children as node (node.id)}
                <svelte:self.TreeNode
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
          {#if selectedFile}<span class="pill">{selectedFile.group}</span>{/if}
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
```

If Svelte does not support the inline recursive component pattern shown as `svelte:self.TreeNode`, split tree rendering into `ui/components/GitTreeNode.svelte` during implementation. Use the same props and callbacks from this step.

- [ ] **Step 3: Add component styles**

Append styles:

```svelte
<style>
  .git-panel {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    background: #050706;
    color: var(--t0);
  }

  .git-header,
  .diff-header,
  .selection-bar {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    border-bottom: 1px solid rgba(255, 255, 255, 0.045);
    background: #070908;
  }

  .git-header {
    height: 36px;
    padding: 0 var(--sp-5);
    flex-shrink: 0;
  }

  .git-header-left,
  .git-header-right,
  .tag-actions,
  .group-row {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }

  .git-header-left {
    min-width: 0;
    flex: 1;
  }

  .title {
    color: #d9f7e8;
    font-size: var(--sm);
    font-weight: 600;
  }

  .branch,
  .count,
  .diff-path {
    overflow: hidden;
    color: #6b7f75;
    font-size: var(--xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pill,
  .tag-pill {
    border: 1px solid rgba(0, 212, 126, 0.18);
    border-radius: 999px;
    padding: 1px var(--sp-3);
    color: var(--ac);
    font-size: var(--xs);
    background: rgba(0, 212, 126, 0.04);
  }

  .pill.staged {
    border-color: rgba(232, 160, 48, 0.22);
    color: var(--s-input);
    background: rgba(232, 160, 48, 0.055);
  }

  .icon-button,
  .tag-actions button,
  .selection-bar button {
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    background: transparent;
    color: #6b7f75;
    font-size: var(--xs);
  }

  .icon-button {
    display: inline-flex;
    width: 22px;
    height: 22px;
    align-items: center;
    justify-content: center;
  }

  .git-body {
    display: grid;
    grid-template-columns: 310px minmax(0, 1fr);
    flex: 1;
    min-height: 0;
  }

  .tree-pane {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    border-right: 1px solid rgba(255, 255, 255, 0.045);
    background: #060806;
  }

  .tree-tools {
    padding: var(--sp-5);
    border-bottom: 1px solid rgba(255, 255, 255, 0.035);
  }

  .tree-tools input {
    width: 100%;
    height: 26px;
    border: 1px solid rgba(255, 255, 255, 0.055);
    border-radius: 6px;
    background: transparent;
    color: var(--t0);
    padding: 0 var(--sp-4);
    font-size: var(--xs);
  }

  .tag-actions {
    flex-wrap: wrap;
    margin-top: var(--sp-4);
  }

  .selection-bar {
    height: 33px;
    padding: 0 var(--sp-5);
    color: #6b7f75;
    font-size: var(--xs);
  }

  .selection-bar button {
    margin-left: auto;
    padding: var(--sp-1) var(--sp-3);
  }

  .tree {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: var(--sp-4);
    color: #6b7f75;
    font-size: var(--xs);
  }

  .group-row {
    height: 24px;
    color: #d9f7e8;
    cursor: pointer;
  }

  .count {
    margin-left: auto;
  }

  .diff-pane {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
  }

  .diff-header {
    height: 36px;
    padding: 0 var(--sp-5);
    flex-shrink: 0;
  }

  .diff-path {
    flex: 1;
    color: #d9f7e8;
    font-weight: 600;
  }

  .diff-body {
    flex: 1;
    min-height: 0;
  }

  .state {
    display: grid;
    height: 100%;
    place-items: center;
    color: #6b7f75;
    font-size: var(--sm);
  }

  .state.error {
    color: var(--s-error);
  }
</style>
```

- [ ] **Step 4: Extract tree node component if needed**

If `GitPanel.svelte` cannot compile with inline recursion, create `ui/components/GitTreeNode.svelte` with this API:

```svelte
<script lang="ts">
  import { CheckSquare, ChevronDown, ChevronRight, FileText, Folder, FolderOpen, Square } from 'lucide-svelte';
  import type { GitFileChange } from '../lib/tauri/git';
  import type { GitTreeNode } from '../lib/git-tree';

  export let node: GitTreeNode;
  export let depth: number;
  export let expanded: Set<string>;
  export let selectedIds: Set<string>;
  export let selectedFile: GitFileChange | null;
  export let fileTags: Record<string, string[]>;
  export let onToggleExpanded: (id: string) => void;
  export let onToggleSelected: (file: GitFileChange) => void;
  export let onSelectFile: (file: GitFileChange) => void;
</script>
```

Render folder rows with `Folder`/`FolderOpen`, file rows with `FileText`, `Square`/`CheckSquare`, and recursively render child nodes when the folder is expanded.

- [ ] **Step 5: Verify component block**

Run:

```bash
npm run check
```

Expected: no Svelte/TypeScript errors. Fix recursive rendering by extracting `GitTreeNode.svelte` if needed.

---

## Task 5: Mock Data And Workspace Wiring

**Files:**

- Modify: `ui/lib/mock/tauri-mock.ts`
- Modify: `ui/components/workspace/PaneContainer.svelte`
- Modify: `ui/components/workspace/TabAddMenu.svelte`
- Modify: `ui/lib/stores/workspace.ts` if Git tab target does not exist yet.

- [ ] **Step 1: Add mock Git overview data**

Add near existing mock constants in `ui/lib/mock/tauri-mock.ts`:

```ts
const MOCK_GIT_OVERVIEW = {
  cwd: 'C:\\Users\\dev\\dashboard',
  branch: 'feat/workspace-tabs',
  upstream: 'origin/feat/workspace-tabs',
  ahead: 3,
  behind: 1,
  branches: [
    { name: 'main', fullName: 'refs/heads/main', kind: 'local', current: false, upstream: 'origin/main', ahead: 0, behind: 2 },
    { name: 'feat/workspace-tabs', fullName: 'refs/heads/feat/workspace-tabs', kind: 'local', current: true, upstream: 'origin/feat/workspace-tabs', ahead: 3, behind: 1 },
  ],
  files: [
    { id: 'staged:ui/components/workspace/PanelHeader.svelte', path: 'ui/components/workspace/PanelHeader.svelte', fileName: 'PanelHeader.svelte', group: 'staged', status: 'modified', staged: true, untracked: false, oldPath: null, additions: 84, deletions: 0 },
    { id: 'staged:ui/components/workspace/TabItem.svelte', path: 'ui/components/workspace/TabItem.svelte', fileName: 'TabItem.svelte', group: 'staged', status: 'modified', staged: true, untracked: false, oldPath: null, additions: 22, deletions: 9 },
    { id: 'unstaged:tauri/src/commands/git.rs', path: 'tauri/src/commands/git.rs', fileName: 'git.rs', group: 'unstaged', status: 'modified', staged: false, untracked: false, oldPath: null, additions: 136, deletions: 12 },
    { id: 'unstaged:ui/lib/tauri/git.ts', path: 'ui/lib/tauri/git.ts', fileName: 'git.ts', group: 'unstaged', status: 'modified', staged: false, untracked: false, oldPath: null, additions: 74, deletions: 0 },
    { id: 'untracked:docs/superpowers/specs/git-overview-design.md', path: 'docs/superpowers/specs/git-overview-design.md', fileName: 'git-overview-design.md', group: 'untracked', status: 'untracked', staged: false, untracked: true, oldPath: null, additions: 41, deletions: 0 },
  ],
};

const MOCK_GIT_DIFF = {
  id: 'staged:ui/components/workspace/PanelHeader.svelte',
  path: 'ui/components/workspace/PanelHeader.svelte',
  group: 'staged',
  language: 'svelte',
  binary: false,
  original: '<div class="header">\n  <span>marlin</span>\n</div>\n',
  modified: '<PanelHeader title="marlin" status="running" />\n',
};
```

- [ ] **Step 2: Add mock invoke cases**

In the `mockInvoke` switch, add:

```ts
case 'git_overview':
  return { ...MOCK_GIT_OVERVIEW, cwd: String(args?.cwd ?? MOCK_GIT_OVERVIEW.cwd) };
case 'git_diff_file':
  return {
    ...MOCK_GIT_DIFF,
    id: `${String(args?.group ?? 'staged')}:${String(args?.path ?? MOCK_GIT_DIFF.path)}`,
    path: String(args?.path ?? MOCK_GIT_DIFF.path),
    group: String(args?.group ?? MOCK_GIT_DIFF.group),
  };
```

- [ ] **Step 3: Add Git tab target if missing**

If `workspace.ts` has tab targets, add Git:

```ts
| { kind: 'git'; cwd: string }
```

If the current checkout is still session-only, do not migrate the entire workspace in this Git panel task. Instead, make `GitPanel` usable from the existing route/pane after the tab migration plan is applied. Keep this task focused on Git.

- [ ] **Step 4: Wire `GitPanel` in pane rendering**

When the workspace has tab targets, update `PaneContainer.svelte`:

```svelte
{:else if activeTab?.target.kind === 'git'}
  <GitPanel cwd={activeTab.target.cwd} onClose={() => closeTab(paneId, activeTab.id)} />
```

If the workspace is still session-only, add this line as a note in the final implementation report: Git panel UI is implemented and mockable, but activation from workspace tabs depends on the tab migration block.

- [ ] **Step 5: Add menu action if missing**

In `TabAddMenu.svelte`, ensure the dispatcher supports:

```ts
select: { action: 'terminal' | 'session' | 'open' | 'git' };
```

Add menu item with Lucide `GitBranch` in implementation code, not text glyphs.

- [ ] **Step 6: Verify mock block**

Run:

```bash
npm run check
npm run dev:mock
```

Expected:

- Typecheck passes.
- Mock app can render the Git panel once wired into a tab/pane.
- Tree shows staged, unstaged, and untracked groups.
- Selecting a file updates the inline diff.
- Tag actions add tags to selected file rows.

---

## Task 6: Changelog And Final Verification

**Files:**

- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add changelog entry**

Add under `## May 2026` in `CHANGELOG.md`:

```markdown
### 05/04 · New — Git review panel
The Git overview now presents changed files as a status-grouped tree with an inline diff viewer, making it easier to review staged, unstaged, and untracked work without leaving the workspace.
```

- [ ] **Step 2: Run frontend checks**

Run:

```bash
npm run check
npm run build
npm run lint:ui
npm run format:check:ui
```

Expected:

- `npm run check` passes.
- `npm run build` passes. Monaco chunk-size warnings are acceptable.
- `npm run lint:ui` passes.
- `npm run format:check:ui` passes.

- [ ] **Step 3: Run Rust checks**

Run:

```bash
cargo fmt --manifest-path tauri/Cargo.toml --check
cargo check --manifest-path tauri/Cargo.toml
cargo clippy --manifest-path tauri/Cargo.toml -- -D warnings
```

Expected: all pass.

- [ ] **Step 4: Run whitespace check**

Run:

```bash
git diff --check
```

Expected: no output.

- [ ] **Step 5: Manual verification**

Run mock mode:

```bash
npm run dev:mock
```

Expected:

- Git panel uses compact header styling and Lucide icons.
- Tree groups files by `Staged`, `Unstaged`, and `Untracked`.
- Folder rows expand and collapse.
- File rows can be selected.
- Tags can be applied to selected files.
- Search filters by path and tag.
- Clicking a file loads the inline diff on the right.
- There is no `Open modal` action.

Run Tauri mode:

```bash
npm run tauri:dev
```

Expected:

- Real Git repositories load changed files.
- Staged diffs compare `HEAD:path` to index.
- Unstaged diffs compare index to working tree.
- Untracked diffs compare empty to working tree.
- Deleted diffs compare base content to empty.

If Tauri mode cannot be run in the current environment, document that blocker and do not claim real Git verification.

---

## Self-Review Notes

- Spec coverage: Tasks 1-2 cover typed Git data, backend commands, diff pair semantics, and flat file lists. Tasks 3-5 cover inline diff, status-grouped tree, tags, filtering, mock mode, and workspace panel wiring. Task 6 covers changelog and verification.
- Placeholder scan: no placeholder implementation steps remain. Conditional workspace wiring is explicit because this checkout currently appears session-only while the worklog references a tab-aware workspace branch.
- Type consistency: `GitChangeGroup`, `GitFileChange`, `GitOverview`, `GitDiffFile`, tag keys, and command names are consistent across frontend wrappers, helpers, mock data, and Rust commands.
