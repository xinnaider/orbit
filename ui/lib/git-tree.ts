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

  for (const file of files) {
    const parts = file.path.split('/');
    let current = root;

    for (const part of parts.slice(0, -1)) {
      const nextPath = current.path ? `${current.path}/${part}` : part;
      let folder = current.children.find(
        (child): child is GitTreeFolderNode =>
          child.kind === 'folder' && child.path === nextPath,
      );
      if (!folder) {
        folder = {
          kind: 'folder',
          id: `${group}:${nextPath}`,
          name: part,
          path: nextPath,
          children: [],
        };
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

export function filterGitFiles(
  files: GitFileChange[],
  query: string,
  tagsByFile: Record<string, string[]>,
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
