import { diffLines } from 'diff';
import { getDiff } from './tauri/diff';
import { gitDiffFile, type GitFileChange } from './tauri/git';

export interface ResolvedFileDiff {
  source: 'inline' | 'file-history' | 'git';
  original: string;
  modified: string;
  path: string;
}

function inlineFromToolInput(toolInput: Record<string, unknown> | null | undefined): {
  original: string;
  modified: string;
} | null {
  if (!toolInput) return null;
  const oldStr = toolInput.old_string;
  const newStr = toolInput.new_string;
  if (typeof oldStr === 'string' && typeof newStr === 'string') {
    return { original: oldStr, modified: newStr };
  }
  const content = toolInput.content;
  if (typeof content === 'string') {
    return { original: '', modified: content };
  }
  return null;
}

/** Resolve the best available diff for a file path from journal, Claude history, or git. */
export async function resolveFileDiff(options: {
  path: string;
  cwd?: string;
  toolInput?: Record<string, unknown> | null;
  sessionId?: string;
  fileHash?: string;
  fromVersion?: number;
  toVersion?: number;
  gitFile?: GitFileChange;
  statusOutput?: string;
}): Promise<ResolvedFileDiff | null> {
  const inline = inlineFromToolInput(options.toolInput);
  if (inline) {
    return { source: 'inline', path: options.path, ...inline };
  }

  if (
    options.sessionId &&
    options.fileHash != null &&
    options.fromVersion != null &&
    options.toVersion != null
  ) {
    try {
      const result = await getDiff(
        options.sessionId,
        options.fileHash,
        options.fromVersion,
        options.toVersion
      );
      const original = result.hunks
        .flatMap((h) => h.lines.filter((l) => l.kind !== 'added').map((l) => l.content))
        .join('\n');
      const modified = result.hunks
        .flatMap((h) => h.lines.filter((l) => l.kind !== 'removed').map((l) => l.content))
        .join('\n');
      if (original || modified) {
        return { source: 'file-history', path: options.path, original, modified };
      }
    } catch {
      /* fall through */
    }
  }

  if (options.cwd && options.gitFile) {
    try {
      const gitDiff = await gitDiffFile(options.cwd, options.gitFile, options.statusOutput);
      return {
        source: 'git',
        path: options.path,
        original: gitDiff.original,
        modified: gitDiff.modified,
      };
    } catch {
      return null;
    }
  }

  return null;
}

/** Build Myers diff lines for display (shared helper for tests and UI). */
export function buildDiffPreview(oldText: string, newText: string) {
  return diffLines(oldText, newText);
}
