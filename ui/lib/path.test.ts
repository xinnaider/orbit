import { describe, expect, it } from 'vitest';
import { shortenPath } from './path';

describe('shortenPath', () => {
  it('returns short paths unchanged', () => {
    expect(shortenPath('C:/proj')).toBe('C:/proj');
  });

  it('keeps last two segments for long paths', () => {
    const p = 'C:\\Users\\fernandonepen\\Documents\\NEPEN\\nms';
    expect(shortenPath(p)).toBe('…/NEPEN/nms');
  });
});
