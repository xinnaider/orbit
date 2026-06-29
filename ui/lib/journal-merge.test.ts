import { describe, expect, it } from 'vitest';
import { mergeJournalBySeq } from './journal-merge';
import type { JournalEntry } from './types';

function entry(seq: number, text: string): JournalEntry {
  return {
    sessionId: '1',
    timestamp: '2026-01-01T00:00:00Z',
    entryType: 'assistant',
    text,
    thinking: null,
    thinkingDuration: null,
    tool: null,
    toolInput: null,
    output: null,
    exitCode: null,
    linesChanged: null,
    seq,
    epoch: '',
  };
}

describe('mergeJournalBySeq', () => {
  it('returns db entries when live feed is empty', () => {
    const db = [entry(1, 'from db')];
    expect(mergeJournalBySeq([], db)).toEqual(db);
  });

  it('keeps live entries when db is empty', () => {
    const live = [entry(1, 'live')];
    expect(mergeJournalBySeq(live, [])).toEqual(live);
  });

  it('prefers live over db for the same seq', () => {
    const live = [entry(1, 'live'), entry(2, 'only live')];
    const db = [entry(1, 'stale db')];
    const merged = mergeJournalBySeq(live, db);
    expect(merged).toHaveLength(2);
    expect(merged[0].text).toBe('live');
    expect(merged[1].text).toBe('only live');
  });

  it('includes higher-seq live entries missing from db snapshot', () => {
    const live = [entry(1, 'a'), entry(2, 'b'), entry(3, 'c')];
    const db = [entry(1, 'a')];
    expect(mergeJournalBySeq(live, db)).toHaveLength(3);
  });
});
