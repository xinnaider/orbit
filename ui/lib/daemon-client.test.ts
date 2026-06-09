import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { createDaemonFeedBridge } from './daemon-client';
import { journal } from './stores/journal';
import type { DaemonEvent, ProviderId } from './daemon-feed';

let evtId = 0;
function ev(
  type: string,
  data: Record<string, unknown> = {},
  runId = 'run_1',
  provider: ProviderId = 'claude'
): DaemonEvent {
  return {
    id: `evt_${evtId++}`,
    runId,
    provider,
    type,
    createdAt: '2026-06-08T12:00:00.000Z',
    sequence: evtId,
    data,
  };
}

function feed(sessionId: number) {
  return get(journal).get(sessionId) ?? [];
}

describe('createDaemonFeedBridge', () => {
  beforeEach(() => {
    journal.set(new Map());
    evtId = 0;
  });

  it('appends a rendered event to the resolved session', () => {
    const bridge = createDaemonFeedBridge(() => 5);
    bridge.ingest(ev('message.completed', { messageId: 'm1', text: 'done' }));
    expect(feed(5)).toHaveLength(1);
    expect(feed(5)[0].entryType).toBe('assistant');
    expect(feed(5)[0].text).toBe('done');
  });

  it('drops events with no resolvable session', () => {
    const bridge = createDaemonFeedBridge(() => null);
    bridge.ingest(ev('message.completed', { messageId: 'm1', text: 'x' }));
    expect(get(journal).size).toBe(0);
  });

  it('ignores non-render events (usage/session)', () => {
    const bridge = createDaemonFeedBridge(() => 1);
    bridge.ingest(ev('usage.updated', { usage: {} }));
    bridge.ingest(ev('session.idle'));
    expect(get(journal).size).toBe(0);
  });

  it('accumulates message deltas with the same messageId into one entry', () => {
    const bridge = createDaemonFeedBridge(() => 1);
    bridge.ingest(ev('message.started', { messageId: 'm1', text: '' }));
    bridge.ingest(ev('message.delta', { messageId: 'm1', text: 'Hel' }));
    bridge.ingest(ev('message.delta', { messageId: 'm1', text: 'lo' }));
    bridge.ingest(ev('message.delta', { messageId: 'm1', text: ' world' }));

    const entries = feed(1);
    expect(entries).toHaveLength(1);
    expect(entries[0].text).toBe('Hello world');
  });

  it('completed replaces the accumulated delta text with the canonical text', () => {
    const bridge = createDaemonFeedBridge(() => 1);
    bridge.ingest(ev('message.delta', { messageId: 'm1', text: 'Hel' }));
    bridge.ingest(ev('message.delta', { messageId: 'm1', text: 'lo' }));
    bridge.ingest(ev('message.completed', { messageId: 'm1', text: 'Hello, world!' }));

    const entries = feed(1);
    expect(entries).toHaveLength(1);
    expect(entries[0].text).toBe('Hello, world!');
  });

  it('separate messageIds create separate entries', () => {
    const bridge = createDaemonFeedBridge(() => 1);
    bridge.ingest(ev('message.delta', { messageId: 'm1', text: 'first' }));
    bridge.ingest(ev('message.delta', { messageId: 'm2', text: 'second' }));
    expect(feed(1)).toHaveLength(2);
  });

  it('accumulates reasoning deltas into a thinking entry', () => {
    const bridge = createDaemonFeedBridge(() => 1);
    bridge.ingest(ev('reasoning.started', { messageId: 'r1', text: '' }));
    bridge.ingest(ev('reasoning.delta', { messageId: 'r1', text: 'think ' }));
    bridge.ingest(ev('reasoning.delta', { messageId: 'r1', text: 'more' }));

    const entries = feed(1);
    expect(entries).toHaveLength(1);
    expect(entries[0].entryType).toBe('thinking');
    expect(entries[0].thinking).toBe('think more');
  });

  it('tool start and result are kept as separate entries (Feed groups them)', () => {
    const bridge = createDaemonFeedBridge(() => 1);
    bridge.ingest(ev('tool.started', { id: 't1', tool: 'bash', input: { cmd: 'ls' } }));
    bridge.ingest(ev('tool.completed', { id: 't1', output: 'a\nb', exitCode: 0 }));

    const entries = feed(1);
    expect(entries).toHaveLength(2);
    expect(entries[0].entryType).toBe('toolCall');
    expect(entries[1].entryType).toBe('toolResult');
  });

  it('accumulates tool.delta chunks with the same id into one progress entry', () => {
    const bridge = createDaemonFeedBridge(() => 1);
    bridge.ingest(ev('tool.delta', { id: 't1', text: 'line 1\n' }));
    bridge.ingest(ev('tool.delta', { id: 't1', text: 'line 2\n' }));

    const entries = feed(1);
    expect(entries).toHaveLength(1);
    expect(entries[0].entryType).toBe('progress');
    expect(entries[0].text).toBe('line 1\nline 2\n');
  });

  it('routes events to the right session via the resolver', () => {
    const bridge = createDaemonFeedBridge((e) => (e.runId === 'run_a' ? 10 : 20));
    bridge.ingest(ev('message.completed', { messageId: 'm1', text: 'A' }, 'run_a'));
    bridge.ingest(ev('message.completed', { messageId: 'm2', text: 'B' }, 'run_b'));

    expect(feed(10).map((e) => e.text)).toEqual(['A']);
    expect(feed(20).map((e) => e.text)).toEqual(['B']);
  });

  it('is provider-agnostic — same accumulation for codex/opencode', () => {
    for (const provider of ['codex', 'opencode'] as ProviderId[]) {
      journal.set(new Map());
      const bridge = createDaemonFeedBridge(() => 1);
      bridge.ingest(ev('message.delta', { messageId: 'm1', text: 'x' }, 'run_1', provider));
      bridge.ingest(ev('message.delta', { messageId: 'm1', text: 'y' }, 'run_1', provider));
      expect(feed(1)).toHaveLength(1);
      expect(feed(1)[0].text).toBe('xy');
    }
  });
});
