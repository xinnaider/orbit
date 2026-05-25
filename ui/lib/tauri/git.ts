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

/** Stage all unstaged changes to git index */
export function gitStageAll(cwd: string): Promise<void> {
  return invoke<void>('git_stage_all', { cwd });
}

/** Reset all staged changes from git index */
export function gitResetStaged(cwd: string): Promise<void> {
  return invoke<void>('git_reset_staged', { cwd });
}

/** Commit staged changes with optional message */
export function gitCommit(cwd: string, message?: string): Promise<void> {
  return invoke<void>('git_commit', { cwd, message });
}

/** Get formatted diff output for a file */
export function gitDiffFormatted(cwd: string, filePath: string): Promise<string> {
  return invoke<string>('git_diff_formatted', { cwd, filePath });
}

/** Validate git configuration before operations */
export function validateGitConfig(cwd: string): Promise<boolean> {
  return invoke<boolean>('git_validate_config', { cwd });
}

/** Quick commit with auto-generated message from file changes */
export function gitQuickCommit(cwd: string): Promise<void> {
  return invoke<void>('git_quick_commit', { cwd });
}

/** Reset all working tree changes */
export function gitResetWorkingTree(cwd: string): Promise<void> {
  return invoke<void>('git_reset_working_tree', { cwd });
}

/** Stage a single file */
export function gitStageFile(cwd: string, filePath: string): Promise<void> {
  return invoke<void>('git_stage_file', { cwd, filePath });
}

/** Unstage a single file */
export function gitUnstageFile(cwd: string, filePath: string): Promise<void> {
  return invoke<void>('git_unstage_file', { cwd, filePath });
}

/** Chain git commands to handle PowerShell 5.1 limitations (convert && to ; if ($?) { }) */
export function chainGitCommands(commands: string[]): string[] {
  return commands.map(cmd => cmd.replace(/&&/g, '; if ($?) { }'));
}
