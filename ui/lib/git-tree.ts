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

export const STATUS_SYMBOLS: Record<string, string> = {
  modified: 'M',
  added: 'A',
  deleted: 'D',
  renamed: 'R',
  copied: 'C',
  untracked: 'U',
};

export const STATUS_COLORS: Record<string, string> = {
  modified: '#e2b714',
  added: '#00d47e',
  deleted: '#f14c4c',
  renamed: '#a379e6',
  copied: '#4fc1ff',
  untracked: '#4ec9b0',
};

const GROUP_LABELS: Record<GitChangeGroup, string> = {
  staged: 'Staged',
  unstaged: 'Unstaged',
  untracked: 'Untracked',
};

function sortNodes(nodes: GitTreeNode[]): void {
  nodes.sort((a, b) => {
    if (a.kind !== b.kind) return a.kind === 'folder' ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
  for (const node of nodes) {
    if (node.kind === 'folder') sortNodes(node.children);
  }
}

function buildFolderTree(group: GitChangeGroup, files: GitFileChange[]): GitTreeNode[] {
  const root: GitTreeFolderNode = {
    kind: 'folder',
    id: `${group}:`,
    name: '',
    path: '',
    children: [],
  };

  // Use a Map for O(1) folder lookups during construction
  const folderMap = new Map<string, GitTreeFolderNode>();
  folderMap.set('', root);

  for (const file of files) {
    const parts = file.path.split('/');
    const folderPath = parts.slice(0, -1).join('/');

    // Ensure all parent folders exist using Map lookups
    let accumulated = '';
    for (const part of parts.slice(0, -1)) {
      const parentPath = accumulated;
      accumulated = accumulated ? `${accumulated}/${part}` : part;

      if (!folderMap.has(accumulated)) {
        const folder: GitTreeFolderNode = {
          kind: 'folder',
          id: `${group}:${accumulated}`,
          name: part,
          path: accumulated,
          children: [],
        };
        folderMap.set(accumulated, folder);

        // Add to parent (parent always exists in Map by construction)
        const parent = folderMap.get(parentPath)!;
        parent.children.push(folder);
      }
    }

    // Add file to its parent folder
    const parent = folderMap.get(folderPath)!;
    parent.children.push({
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

export function buildGitTree(files: GitFileChange[]): GitTreeGroup[] {
  const groups: GitChangeGroup[] = ['staged', 'unstaged', 'untracked'];
  const result: GitTreeGroup[] = [];

  for (const group of groups) {
    const groupFiles = files.filter((f) => f.group === group);
    if (groupFiles.length === 0) continue;

    result.push({
      group,
      label: GROUP_LABELS[group],
      children: buildFolderTree(group, groupFiles),
      count: groupFiles.length,
    });
  }

  return result;
}

/**
 * Build a flat tree from all files regardless of group.
 * No staged/unstaged/untracked separation — like VS Code's Source Control tree.
 */
export function buildFlatTree(files: GitFileChange[]): GitTreeNode[] {
  return buildFolderTree('all' as GitChangeGroup, files);
}

export function filterGitFiles(
  files: GitFileChange[],
  query: string,
  tagsByFile: Record<string, string[]>
): GitFileChange[] {
  if (!query.trim()) return files;

  const q = query.toLowerCase();

  return files.filter((file) => {
    // Search by path
    if (file.path.toLowerCase().includes(q)) return true;

    // Search by group
    if (file.group.toLowerCase().includes(q)) return true;

    // Search by status
    if (file.status.toLowerCase().includes(q)) return true;

    // Search by associated tags
    const tags = tagsByFile[file.id];
    if (tags && tags.some((tag) => tag.toLowerCase().includes(q))) return true;

    return false;
  });
}
