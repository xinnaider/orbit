# Git Panel Tree Diff Design

Date: 2026-05-04

## Goal

Design the Orbit Git overview as a normal workspace panel with a status-grouped file tree on the left and an inline diff viewer on the right.

## Current Context

Orbit is moving toward a tabbed workspace where each pane has tabs at the top and a standardized compact `PanelHeader` below the tabs. The Git overview should follow the same panel structure and the approved Compact Stack / Terminal Premium visual language.

The Git panel should not behave like a separate modal-first feature. It should be part of the workspace, easy to keep open beside agent and terminal tabs, and optimized for quickly moving through changed files.

## Selected Design

Use a two-column Git panel body:

- Left column: changed-file tree.
- Right column: inline diff for the currently selected file.

Clicking a file in the tree updates the inline diff on the right. There is no `Open modal` action in the approved design.

## Panel Chrome

The Git panel uses the same pane composition as the rest of the workspace:

1. `TabBar` at the top.
2. Shared compact `PanelHeader` below the tabs.
3. Git panel body below the header.

Header content:

- Leading Lucide icon: `GitBranch`.
- Title: `Git Overview` or `Git`.
- Repository state: current branch, such as `feat/workspace-tabs`.
- Summary pills: total changed files and staged count.
- Refresh action using `RefreshCw`.
- Close action using `X` when the pane/tab is closable.

No emojis or improvised glyphs should be used. All icons must come from `lucide-svelte`.

## Tree Layout

The tree is grouped by Git state first:

1. `Staged`
2. `Unstaged`
3. `Untracked`

Inside each state group, files are nested by folder path. Folder rows can expand and collapse.

File rows show:

- Selection checkbox using Lucide `Square` and `CheckSquare`.
- Git status indicator, such as modified, added, deleted, renamed, or untracked.
- File name.
- Optional local review tags.

Suggested Lucide icons:

- Expand/collapse: `ChevronRight`, `ChevronDown`.
- Folder: `Folder`, `FolderOpen`.
- File: `FileText`.
- Selected checkbox: `CheckSquare`.
- Unselected checkbox: `Square`.
- Git panel: `GitBranch`.
- Diff area: `FileDiff`.
- Refresh: `RefreshCw`.
- Close: `X`.
- Tag action: `Tag`.

## Tags

Users can add local review tags to files. Tags are for review organization only and do not change Git state.

Required tag behavior:

- Tags can be applied to one selected file.
- Tags can be applied to multiple selected files.
- Tags appear inline on file rows.
- Search/filter can match tags.
- Tags persist locally for the repository path and file path.
- Tags are cleared when a file no longer appears in Git status.

Initial suggested tags:

- `ready`
- `needs review`
- `docs`
- `risky`
- `generated`

Custom tags are out of scope for the first implementation unless they are already trivial to support. The first version can use a fixed tag list.

## Search And Filtering

The tree includes a compact search input above the groups.

Search should filter by:

- File name.
- Folder path.
- Git state group.
- Tag.

Filtering should preserve the status grouping. Empty groups can be hidden while a filter is active.

## Inline Diff

The right side shows the selected file diff inline.

Diff header content:

- Leading `FileDiff` icon.
- Full file path.
- Tags for the selected file.
- Git state, such as `staged`, `unstaged`, or `untracked`.

Diff content:

- Use the Monaco diff viewer already selected for VS Code-style diffs.
- It should fill the available right-side panel area.
- If no file is selected, show an empty state prompting the user to select a file.
- If loading fails, show an inline error state in the diff area.

There is no modal in the approved design. The inline diff is the primary and only diff surface for this feature.

## Diff Pair Semantics

The backend must return correct original/modified pairs:

- Staged file: `HEAD:path` vs index.
- Unstaged file: index vs working tree.
- Untracked file: empty vs working tree.
- Deleted file: base content vs empty.

For files with both staged and unstaged changes, the tree can show two entries for the same path, one under `Staged` and one under `Unstaged`, because they represent different diff pairs.

## Data Model

Frontend-facing Git data should support:

```ts
type GitChangeGroup = 'staged' | 'unstaged' | 'untracked';

interface GitFileChange {
  id: string;
  path: string;
  fileName: string;
  group: GitChangeGroup;
  status: 'modified' | 'added' | 'deleted' | 'renamed' | 'copied' | 'untracked';
  staged: boolean;
  untracked: boolean;
  oldPath: string | null;
  additions: number | null;
  deletions: number | null;
}

interface GitFileTagState {
  repoPath: string;
  filePath: string;
  group: GitChangeGroup;
  tags: string[];
}
```

The tree can be derived from flat `GitFileChange[]` in the UI. The backend does not need to return a nested tree.

## Persistence

Persist tags locally in the frontend. Use a key scoped by repository path, for example:

```text
orbit:git-file-tags:<repo-path>
```

Persisting tags in the database is out of scope for this first Git panel design.

## Error Handling

- Non-Git directories show a clear panel-level empty/error state.
- Git command failures show a compact error message and a refresh action.
- Diff loading failures are shown in the right diff area without clearing the tree.
- Missing working-tree files are handled as deleted files when Git status indicates deletion.
- Binary files should show a clear message that inline text diff is unavailable.

## Accessibility

- File rows are keyboard-focusable.
- Pressing `Enter` on a file row selects it and loads the diff.
- Checkboxes are real buttons or inputs with accessible labels.
- Icon-only actions have `aria-label` values.
- Tags have text labels, not color-only meaning.
- The selected file row has a visible selected state beyond color alone.

## Non-Goals

- No diff modal.
- No staging/unstaging actions in the first implementation.
- No commit creation flow.
- No custom tag creation unless fixed tags are already working and the cost is negligible.
- No backend/database persistence for tags.
- No replacement of Git CLI with a Git library.

## Acceptance Criteria

- Git opens as a normal workspace panel.
- The Git panel uses the shared compact panel header.
- Changed files are grouped by `Staged`, `Unstaged`, and `Untracked`.
- Files are nested by folder inside each group.
- Selecting a file loads its diff inline on the right.
- There is no `Open modal` action for diffs.
- Users can select one or more files and apply fixed local tags.
- Tags appear on file rows and are included in filtering.
- Git diff pairs are correct for staged, unstaged, untracked, and deleted files.
- `npm run check` passes after implementation.
- `cargo check --manifest-path tauri/Cargo.toml` passes after backend changes.

## Self-Review

- Placeholder scan: no placeholder sections or unfinished requirements remain.
- Internal consistency: the spec consistently uses inline diff and explicitly excludes a modal.
- Scope check: the first implementation is limited to tree browsing, inline diff, fixed local tags, and refresh/error states.
- Ambiguity check: grouping, diff pair semantics, persistence, and non-goals are explicit.
