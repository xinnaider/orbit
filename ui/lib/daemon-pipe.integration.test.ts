/**
 * Integration test for the full daemon → feed pipe:
 *   SSE frame → connectDaemonSse → bridge → mapper → run registry → journal.
 *
 * Uses a fake EventSource (jsdom has none) to drive real frames through the
 * actual wiring, instead of unit-testing each piece in isolation. This is the
 * test that proves an event a provider emits ends up in the right session's chat.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { connectDaemonSse } from './daemon-client';
import { registerDaemonRun, resetDaemonRuns, resolveSessionForRun } from './daemon-runs';
import { journal } from './stores/journal';
import type { DaemonEvent } from './daemon-feed';

class FakeEventSource {
  static instances: FakeEventSource[] = [];
  url: string;
  closed = false;
  private listeners = new Map<string, Set<(e: { data: string }) => void>>();

  constructor(url: string) {
    this.url = url;
    FakeEventSource.instances.push(this);
  }
  addEventListener(type: string, cb: (e: { data: string }) => void) {
    if (!this.listeners.has(type)) this.listeners.set(type, new Set());
    this.listeners.get(type)!.add(cb);
  }
  /** Simulate a named SSE frame arriving. */
  emit(event: DaemonEvent) {
    const frame = { data: JSON.stringify(event) };
    this.listeners.get(event.type)?.forEach((cb) => cb(frame));
  }
  emitRaw(type: string, data: string) {
    this.listeners.get(type)?.forEach((cb) => cb({ data }));
  }
  close() {
    this.closed = true;
  }
}

let evtId = 0;
function event(type: string, runId: string, data: Record<string, unknown>): DaemonEvent {
  return {
    id: `evt_${evtId++}`,
    runId,
    provider: 'claude',
    type,
    createdAt: '2026-06-08T12:00:00.000Z',
    sequence: evtId,
    data,
  };
}

function feed(sessionId: number) {
  return get(journal).get(sessionId) ?? [];
}

/** The resolver shape App.svelte uses: map an event's runId via the registry. */
const resolve = (e: DaemonEvent) => resolveSessionForRun(e.runId);

describe('daemon → feed pipe (integration)', () => {
  beforeEach(() => {
    journal.set(new Map());
    resetDaemonRuns();
    evtId = 0;
    FakeEventSource.instances = [];
    vi.stubGlobal('EventSource', FakeEventSource);
  });
  afterEach(() => vi.unstubAllGlobals());

  it('connects to the /events endpoint of the configured daemon', () => {
    connectDaemonSse('http://daemon:4000/', resolve);
    expect(FakeEventSource.instances[0].url).toBe('http://daemon:4000/events');
  });

  it('streams a full assistant turn into the mapped session feed', () => {
    registerDaemonRun('run_1', 77);
    connectDaemonSse('http://d', resolve);
    const es = FakeEventSource.instances[0];

    es.emit(event('message.started', 'run_1', { messageId: 'm1', text: '' }));
    es.emit(event('message.delta', 'run_1', { messageId: 'm1', text: 'Hel' }));
    es.emit(event('message.delta', 'run_1', { messageId: 'm1', text: 'lo' }));
    es.emit(event('message.completed', 'run_1', { messageId: 'm1', text: 'Hello!' }));

    const entries = feed(77);
    expect(entries).toHaveLength(1);
    expect(entries[0].entryType).toBe('assistant');
    expect(entries[0].text).toBe('Hello!');
  });

  it('drops events whose run is not registered to any session', () => {
    connectDaemonSse('http://d', resolve);
    FakeEventSource.instances[0].emit(event('message.completed', 'ghost', { messageId: 'x', text: 'hi' }));
    expect(get(journal).size).toBe(0);
  });

  it('ignores malformed frames without throwing', () => {
    registerDaemonRun('run_1', 1);
    connectDaemonSse('http://d', resolve);
    expect(() => FakeEventSource.instances[0].emitRaw('message.delta', '{not json')).not.toThrow();
    expect(get(journal).size).toBe(0);
  });

  it('routes interleaved runs to their own sessions', () => {
    registerDaemonRun('run_a', 1);
    registerDaemonRun('run_b', 2);
    connectDaemonSse('http://d', resolve);
    const es = FakeEventSource.instances[0];

    es.emit(event('message.completed', 'run_a', { messageId: 'a', text: 'AA' }));
    es.emit(event('message.completed', 'run_b', { messageId: 'b', text: 'BB' }));

    expect(feed(1).map((e) => e.text)).toEqual(['AA']);
    expect(feed(2).map((e) => e.text)).toEqual(['BB']);
  });

  it('disposer closes the connection', () => {
    const dispose = connectDaemonSse('http://d', resolve);
    dispose();
    expect(FakeEventSource.instances[0].closed).toBe(true);
  });
});
