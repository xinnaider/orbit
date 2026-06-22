import type { JournalEntry } from './types';

/** Merge DB history with live feed entries; live wins on seq conflicts. */
export function mergeJournalBySeq(live: JournalEntry[], db: JournalEntry[]): JournalEntry[] {
  if (live.length === 0) return db;
  if (db.length === 0) return live;

  const bySeq = new Map<number, JournalEntry>();
  for (const entry of db) bySeq.set(entry.seq, entry);
  for (const entry of live) bySeq.set(entry.seq, entry);
  return [...bySeq.values()].sort((a, b) => a.seq - b.seq);
}
