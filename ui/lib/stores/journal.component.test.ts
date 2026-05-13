import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, waitFor, cleanup } from '@testing-library/svelte';
import { journal, pendingMessages } from './journal';
import JournalHarness from './JournalHarness.svelte';
import type { JournalEntry } from '../types';

function makeUserEntry(seq: number, text: string): JournalEntry {
  return {
    sessionId: '1',
    timestamp: new Date().toISOString(),
    entryType: 'user',
    text,
    thinking: null,
    thinkingDuration: null,
    tool: null,
    toolInput: null,
    output: null,
    exitCode: null,
    linesChanged: null,
    seq,
    epoch: '1',
  };
}

function simulateSessionOutput(sessionId: number, entry: JournalEntry) {
  journal.update((map) => {
    const existing = map.get(sessionId) ?? [];
    const dup = existing.some((e) => e.seq === entry.seq && e.entryType === entry.entryType);
    if (dup) return map;
    return new Map(map).set(sessionId, [...existing, entry]);
  });
}

describe('JournalHarness — component-level reactivity', () => {
  const SID = 1;

  beforeEach(() => {
    journal.set(new Map());
    pendingMessages.clear();
  });

  afterEach(() => {
    cleanup();
  });

  it('starts with feed-empty when no entries', () => {
    const { getByTestId } = render(JournalHarness, { session: { id: SID } });
    expect(getByTestId('feed-empty')).toBeTruthy();
    expect(getByTestId('counts').dataset.entries).toBe('0');
    expect(getByTestId('counts').dataset.pending).toBe('0');
  });

  it('renders entries after session:output arrives via store', async () => {
    const { getByTestId, queryByTestId } = render(JournalHarness, {
      session: { id: SID },
    });

    expect(getByTestId('feed-empty')).toBeTruthy();

    // Simulate session:output arriving AFTER mount
    simulateSessionOutput(SID, makeUserEntry(1, 'hello world'));

    await waitFor(() => {
      expect(queryByTestId('feed-empty')).toBeNull();
    });

    expect(getByTestId('feed')).toBeTruthy();
    expect(getByTestId('entry-1').textContent).toBe('hello world');
    expect(getByTestId('counts').dataset.entries).toBe('1');
    expect(getByTestId('counts').dataset.pending).toBe('0');
  });

  it('clears pendingMessages when entries arrive', async () => {
    const rendered = render(JournalHarness, { session: { id: SID } });
    const { getByTestId, queryByTestId } = rendered;

    // Step 1: User types message
    pendingMessages.add('hello world');
    await waitFor(() => {
      expect(getByTestId('pending')).toBeTruthy();
    });

    // Step 2: Backend responds with session:output
    simulateSessionOutput(SID, makeUserEntry(1, 'hello world'));

    // Step 3: Reactive blocks fire
    await waitFor(() => {
      expect(queryByTestId('feed-empty')).toBeNull();
    });

    expect(getByTestId('feed')).toBeTruthy();
    expect(getByTestId('entry-1').textContent).toBe('hello world');
    expect(getByTestId('counts').dataset.entries).toBe('1');
    expect(getByTestId('counts').dataset.pending).toBe('0');
    expect(queryByTestId('pending')).toBeNull();
  });

  it('survives loadHistory overwriting the journal store', async () => {
    const { getByTestId, queryByTestId } = render(JournalHarness, {
      session: { id: SID },
    });

    simulateSessionOutput(SID, makeUserEntry(1, 'hello'));
    await waitFor(() => {
      expect(getByTestId('feed')).toBeTruthy();
    });

    // loadHistory replaces journal store
    const dbEntries = [makeUserEntry(1, 'hello')];
    journal.update((m) => new Map(m).set(SID, dbEntries));

    await waitFor(() => {
      expect(queryByTestId('feed-empty')).toBeNull();
    });
    expect(getByTestId('entry-1').textContent).toBe('hello');
  });

  it('handles multiple entries arriving in sequence', async () => {
    const { getByTestId, queryByTestId } = render(JournalHarness, {
      session: { id: SID },
    });

    simulateSessionOutput(SID, makeUserEntry(1, 'first'));
    await waitFor(() => expect(queryByTestId('feed-empty')).toBeNull());
    expect(getByTestId('entry-1').textContent).toBe('first');

    simulateSessionOutput(SID, makeUserEntry(2, 'second'));
    await waitFor(() => {
      expect(getByTestId('entry-2')).toBeTruthy();
    });
    expect(getByTestId('counts').dataset.entries).toBe('2');
  });
});
