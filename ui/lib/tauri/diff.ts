import { invoke } from './invoke';

export type DiffLineKind = 'context' | 'added' | 'removed';

export interface DiffLine {
  kind: DiffLineKind;
  content: string;
}

export interface DiffHunk {
  oldStart: number;
  newStart: number;
  lines: DiffLine[];
}

export interface DiffResult {
  filePath: string;
  fromVersion: number;
  toVersion: number;
  hunks: DiffHunk[];
  added: number;
  removed: number;
}

export interface FileVersionInfo {
  hashId: string;
  versions: [number, string][];
  maxVersion: number;
}

export async function getDiff(
  sessionId: string,
  fileHash: string,
  fromVersion: number,
  toVersion: number
): Promise<DiffResult> {
  return invoke<DiffResult>('get_diff', {
    sessionId,
    fileHash,
    fromVersion,
    toVersion,
  });
}

export async function getFileVersions(sessionId: string): Promise<FileVersionInfo[]> {
  return invoke<FileVersionInfo[]>('get_file_versions', { sessionId });
}
