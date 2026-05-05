import type { GitFileChange } from './tauri/git';

export const FIXED_GIT_TAGS = [
  'ready',
  'needs review',
  'docs',
  'risky',
  'generated',
  'wip',
  'bug',
  'refactor',
] as const;
export type FixedGitTag = (typeof FIXED_GIT_TAGS)[number];

/** Distinct colors for each tag, readable against dark backgrounds. */
export const TAG_COLORS: Record<string, string> = {
  ready: '#00d47e',
  'needs review': '#e8a030',
  docs: '#4888e0',
  risky: '#e04848',
  generated: '#888',
  wip: '#a855f7',
  bug: '#f97316',
  refactor: '#06b6d4',
};

const STORAGE_PREFIX = 'orbit:git-file-tags:';

function storageKey(repoPath: string): string {
  return `${STORAGE_PREFIX}${repoPath}`;
}

export function tagKey(file: { path: string; group: string }): string {
  return `${file.group}:${file.path}`;
}

export function loadGitTags(
  repoPath: string,
  files: GitFileChange[],
): Record<string, string[]> {
  const raw = localStorage.getItem(storageKey(repoPath));
  if (!raw) return {};

  let parsed: Record<string, string[]>;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return {};
  }

  if (typeof parsed !== 'object' || parsed === null) return {};

  // Build set of valid keys from current files
  const validKeys = new Set<string>();
  for (const file of files) {
    validKeys.add(tagKey(file));
  }

  // Filter out stale entries and invalid tag values
  const validTagValues = new Set<string>(FIXED_GIT_TAGS);
  const cleaned: Record<string, string[]> = {};
  let changed = false;

  for (const [key, tags] of Object.entries(parsed)) {
    if (!validKeys.has(key)) {
      changed = true;
      continue;
    }
    if (!Array.isArray(tags)) {
      changed = true;
      continue;
    }
    const filtered = tags.filter((t): t is string => typeof t === 'string' && validTagValues.has(t));
    if (filtered.length === 0) {
      changed = true;
      continue;
    }
    if (filtered.length !== tags.length) changed = true;
    cleaned[key] = filtered;
  }

  // Re-save cleaned data if anything was removed
  if (changed) {
    saveGitTags(repoPath, cleaned);
  }

  return cleaned;
}

export function saveGitTags(repoPath: string, tags: Record<string, string[]>): void {
  localStorage.setItem(storageKey(repoPath), JSON.stringify(tags));
}

export function applyTagToFiles(
  tags: Record<string, string[]>,
  files: GitFileChange[],
  tag: string,
): Record<string, string[]> {
  const next: Record<string, string[]> = { ...tags };

  for (const file of files) {
    const key = tagKey(file);
    const current = next[key];
    if (current) {
      if (!current.includes(tag)) {
        next[key] = [...current, tag];
      }
    } else {
      next[key] = [tag];
    }
  }

  return next;
}

/** Remove a tag from a set of files (toggle-off). */
export function removeTagFromFiles(
  tags: Record<string, string[]>,
  files: GitFileChange[],
  tag: string,
): Record<string, string[]> {
  const next: Record<string, string[]> = { ...tags };

  for (const file of files) {
    const key = tagKey(file);
    const current = next[key];
    if (!current) continue;
    const filtered = current.filter((t) => t !== tag);
    if (filtered.length === 0) {
      delete next[key];
    } else {
      next[key] = filtered;
    }
  }

  return next;
}

export function tagsByFileId(
  files: GitFileChange[],
  tags: Record<string, string[]>,
): Record<string, string[]> {
  const result: Record<string, string[]> = {};

  for (const file of files) {
    const key = tagKey(file);
    if (tags[key] && tags[key].length > 0) {
      result[file.id] = tags[key];
    }
  }

  return result;
}
