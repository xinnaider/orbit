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
  statusOutput: string;
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

/** Fast branch detection — reads .git/HEAD without spawning git. */
export function gitBranch(cwd: string): Promise<string | null> {
  return invoke<string | null>('git_branch', { cwd });
}

export function gitOverview(cwd: string): Promise<GitOverview> {
  return invoke<GitOverview>('git_overview', { cwd });
}

export function gitDiffFile(
  cwd: string,
  file: GitFileChange,
  statusOutput?: string
): Promise<GitDiffFile> {
  return invoke<GitDiffFile>('git_diff_file', {
    cwd,
    path: file.path,
    group: file.group,
    statusOutput: statusOutput ?? null,
  });
}

export function writeFileContent(path: string, content: string): Promise<void> {
  return invoke<void>('write_file_content', { path, content });
}
