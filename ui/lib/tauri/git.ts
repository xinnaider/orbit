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
