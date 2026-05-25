import { describe, expect, it } from 'vitest';
import { buildDiffPreview, resolveFileDiff } from './resolve-file-diff';

describe('resolveFileDiff', () => {
  it('prefers inline toolInput edit diff', async () => {
    const result = await resolveFileDiff({
      path: 'src/a.ts',
      toolInput: { old_string: 'a\n', new_string: 'b\n' },
    });
    expect(result?.source).toBe('inline');
    expect(result?.original).toBe('a\n');
    expect(result?.modified).toBe('b\n');
  });

  it('buildDiffPreview returns changed chunks', () => {
    const chunks = buildDiffPreview('line1\n', 'line2\n');
    expect(chunks.some((c) => c.added || c.removed)).toBe(true);
  });
});
