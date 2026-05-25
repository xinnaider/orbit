/** Shorten a filesystem path for compact header display (e.g. …/NEPEN/nms). */
export function shortenPath(cwd: string, maxLen = 36): string {
  const normalized = cwd.replace(/\\/g, '/');
  if (normalized.length <= maxLen) return normalized;
  const parts = normalized.split('/').filter(Boolean);
  if (parts.length >= 2) {
    const tail = parts.slice(-2).join('/');
    const candidate = `…/${tail}`;
    if (candidate.length <= maxLen) return candidate;
  }
  return `…${normalized.slice(-(maxLen - 1))}`;
}
