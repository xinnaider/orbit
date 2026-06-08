import { describe, it, expect } from 'vitest';
import {
  EVENT_CLASSIFICATION,
  classifyDaemonEvent,
  mapDaemonEvent,
  type DaemonEvent,
  type ProviderId,
} from './daemon-feed';

/**
 * The full SDK event catalog — the homologation contract.
 *
 * Source of truth: every event type the daemon normalizers can emit
 * (orbit/daemon/src/adapters/providers/**). Regenerate with:
 *   grep -rohE '"(run|message|reasoning|tool|permission|session|sessions|usage|file|todo|error|unknown)[.a-z]*"' \
 *     daemon/src/adapters/providers daemon/src/application --include="*.ts" | tr -d '"' | sort -u
 *
 * If the daemon adds a new event type, add it here AND classify it in
 * EVENT_CLASSIFICATION — otherwise the "no event is unclassified" test fails.
 */
const SDK_EVENT_CATALOG = [
  'run.create', 'run.created', 'run.start', 'run.started', 'run.completed', 'run.failed',
  'message.created', 'message.started', 'message.delta', 'message.updated', 'message.completed',
  'message.part.delta', 'message.part.updated',
  'reasoning.started', 'reasoning.delta', 'reasoning.completed',
  'tool.create', 'tool.start', 'tool.started', 'tool.delta', 'tool.update',
  'tool.finish', 'tool.completed', 'tool.failed',
  'permission.request', 'permission.requested', 'permission.asked',
  'permission.reply', 'permission.resolved',
  'error', 'unknown',
  'usage.updated', 'file.read', 'file.status', 'file.changed', 'todo.updated',
  'session.abort', 'session.children', 'session.command', 'session.create', 'session.created',
  'session.delete', 'session.deleted', 'session.diff', 'session.discovered', 'session.get',
  'session.idle', 'session.init', 'session.list', 'session.message', 'session.messages',
  'session.next.agent.switched', 'session.next.model.switched', 'session.permission.reply',
  'session.prompt', 'session.revert', 'session.share', 'session.shell', 'session.status',
  'session.summarize', 'session.unrevert', 'session.unshare', 'session.update', 'session.updated',
  'sessions.info', 'sessions.list', 'sessions.messages', 'sessions.rename', 'sessions.tag',
] as const;

const PROVIDERS: ProviderId[] = ['claude', 'codex', 'opencode'];

function makeEvent(type: string, provider: ProviderId, data: Record<string, unknown> = {}): DaemonEvent {
  return {
    id: `evt_${type}`,
    runId: 'run_1',
    provider,
    type,
    createdAt: '2026-06-08T12:00:00.000Z',
    sequence: 1,
    data,
  };
}

describe('daemon-feed homologation', () => {
  it('classifies EVERY SDK event type — none unclassified', () => {
    const unclassified = SDK_EVENT_CATALOG.filter((t) => classifyDaemonEvent(t) === undefined);
    expect(unclassified).toEqual([]);
  });

  it('has no classification entries for non-existent (stale) event types', () => {
    const catalog = new Set<string>(SDK_EVENT_CATALOG);
    const stale = Object.keys(EVENT_CLASSIFICATION).filter((t) => !catalog.has(t));
    expect(stale).toEqual([]);
  });

  it('classification is provider-agnostic', () => {
    for (const type of SDK_EVENT_CATALOG) {
      const decisions = PROVIDERS.map((p) => {
        const entry = mapDaemonEvent(makeEvent(type, p), { sessionId: 1, seq: 0 });
        return entry === null ? 'ignore' : 'render';
      });
      expect(new Set(decisions).size, `type ${type} differs across providers`).toBe(1);
    }
  });

  it('render events always produce a JournalEntry; ignore events produce null', () => {
    for (const type of SDK_EVENT_CATALOG) {
      const entry = mapDaemonEvent(makeEvent(type, 'claude'), { sessionId: 7, seq: 3 });
      if (classifyDaemonEvent(type) === 'render') {
        expect(entry, `render type ${type} must map to an entry`).not.toBeNull();
        expect(entry!.sessionId).toBe('7');
        expect(entry!.seq).toBe(3);
      } else {
        expect(entry, `ignore type ${type} must map to null`).toBeNull();
      }
    }
  });
});

describe('daemon-feed entry mapping', () => {
  it.each(PROVIDERS)('maps message.delta → assistant with text [%s]', (provider) => {
    const entry = mapDaemonEvent(
      makeEvent('message.delta', provider, { role: 'assistant', text: 'hello world' }),
      { sessionId: 1, seq: 0 }
    );
    expect(entry?.entryType).toBe('assistant');
    expect(entry?.text).toBe('hello world');
  });

  it.each(PROVIDERS)('maps reasoning.delta → thinking [%s]', (provider) => {
    const entry = mapDaemonEvent(
      makeEvent('reasoning.delta', provider, { text: 'pondering...' }),
      { sessionId: 1, seq: 0 }
    );
    expect(entry?.entryType).toBe('thinking');
    expect(entry?.thinking).toBe('pondering...');
  });

  it.each(PROVIDERS)('maps tool.started → toolCall with tool + input [%s]', (provider) => {
    const entry = mapDaemonEvent(
      makeEvent('tool.started', provider, { tool: 'bash', input: { cmd: 'ls' } }),
      { sessionId: 1, seq: 0 }
    );
    expect(entry?.entryType).toBe('toolCall');
    expect(entry?.tool).toBe('bash');
    expect(entry?.toolInput).toEqual({ cmd: 'ls' });
  });

  it.each(PROVIDERS)('maps tool.completed → toolResult with output [%s]', (provider) => {
    const entry = mapDaemonEvent(
      makeEvent('tool.completed', provider, { output: 'file1\nfile2', exitCode: 0 }),
      { sessionId: 1, seq: 0 }
    );
    expect(entry?.entryType).toBe('toolResult');
    expect(entry?.output).toBe('file1\nfile2');
    expect(entry?.exitCode).toBe(0);
    expect(entry?.feedError).toBe(false);
  });

  it.each(PROVIDERS)('maps tool.failed → toolResult flagged as error [%s]', (provider) => {
    const entry = mapDaemonEvent(
      makeEvent('tool.failed', provider, { output: 'boom' }),
      { sessionId: 1, seq: 0 }
    );
    expect(entry?.entryType).toBe('toolResult');
    expect(entry?.feedError).toBe(true);
  });

  it.each(PROVIDERS)('maps tool.delta → progress (streaming) [%s]', (provider) => {
    const entry = mapDaemonEvent(
      makeEvent('tool.delta', provider, { text: 'partial output' }),
      { sessionId: 1, seq: 0 }
    );
    expect(entry?.entryType).toBe('progress');
  });

  it.each(PROVIDERS)('maps permission.requested → system line [%s]', (provider) => {
    const entry = mapDaemonEvent(
      makeEvent('permission.requested', provider, { text: 'allow edit?' }),
      { sessionId: 1, seq: 0 }
    );
    expect(entry?.entryType).toBe('system');
  });

  it.each(PROVIDERS)('maps run.failed → system error line [%s]', (provider) => {
    const entry = mapDaemonEvent(
      makeEvent('run.failed', provider, { message: 'connection lost' }),
      { sessionId: 1, seq: 0 }
    );
    expect(entry?.entryType).toBe('system');
    expect(entry?.feedError).toBe(true);
    expect(entry?.text).toBe('connection lost');
  });

  it('ignores token/usage events (owned by meta panel)', () => {
    const entry = mapDaemonEvent(
      makeEvent('usage.updated', 'codex', { usage: { input: 10, output: 5 } }),
      { sessionId: 1, seq: 0 }
    );
    expect(entry).toBeNull();
  });
});
