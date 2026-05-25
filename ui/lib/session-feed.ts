import { get } from 'svelte/store';
import { journal } from './stores/journal';
import { workspace } from './stores/workspace';
import type { JournalEntry } from './types';

let feedSeq = 0;

/** Whether this session is open in any workspace tab. */
export function isSessionOpenInWorkspace(sessionId: number): boolean {
  const ws = get(workspace);
  for (const pane of Object.values(ws.panes)) {
    for (const tab of pane.tabs) {
      if (tab.target.kind === 'agent' && tab.target.sessionId === sessionId) {
        return true;
      }
    }
  }
  return false;
}

/** Append a system line to the session feed (chat timeline). */
export function appendSessionFeedMessage(
  sessionId: number,
  text: string,
  opts?: { error?: boolean }
): JournalEntry {
  const entry: JournalEntry = {
    sessionId: String(sessionId),
    timestamp: new Date().toISOString(),
    entryType: 'system',
    text,
    thinking: null,
    thinkingDuration: null,
    tool: null,
    toolInput: null,
    output: null,
    exitCode: null,
    linesChanged: null,
    seq: feedSeq++,
    epoch: '',
    feedError: opts?.error ?? false,
  };

  journal.update((map) => {
    const next = new Map(map);
    next.set(sessionId, [...(next.get(sessionId) ?? []), entry]);
    return next;
  });

  return entry;
}
