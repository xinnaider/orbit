/**
 * Daemon → Feed bridge.
 *
 * Consumes the daemon SDK event stream (DaemonEvent, delivered over SSE),
 * runs each event through the homologation mapper (daemon-feed.ts), accumulates
 * streaming text deltas into a single chat entry, and pushes the result into the
 * journal store the Feed renders.
 *
 * This is the integration counterpart to daemon-feed.ts: the mapper decides
 * *what* an event becomes; this bridge decides *where* it lands and how
 * incremental deltas collapse into one bubble.
 */

import { journal } from './stores/journal';
import {
  mapDaemonEvent,
  classifyDaemonEvent,
  EVENT_CLASSIFICATION,
  type DaemonEvent,
} from './daemon-feed';
import type { JournalEntry } from './types';

/** Resolve the daemon's string runId to Orbit's numeric session id. */
export type SessionResolver = (event: DaemonEvent) => number | null;

/**
 * Key under which streaming events for the same logical message/reasoning block
 * collapse into one entry. Returns null when the event should append standalone
 * (tool calls, permissions, errors keep their own line — the Feed groups them).
 */
function streamKey(event: DaemonEvent): string | null {
  const data = event.data ?? {};
  const id = (data.messageId ?? data.id) as string | undefined;
  if (!id) return null;
  if (event.type.startsWith('message.') || event.type.startsWith('reasoning.')) {
    return `${event.runId}:msg:${id}`;
  }
  if (event.type === 'tool.delta' || event.type === 'tool.update') {
    return `${event.runId}:tooldelta:${id}`;
  }
  return null;
}

function isDelta(type: string): boolean {
  return type.endsWith('.delta');
}

function mergeStreamed(target: JournalEntry, incoming: JournalEntry, delta: boolean): void {
  const field = target.entryType === 'thinking' ? 'thinking' : 'text';
  const addition = incoming[field] ?? '';
  if (delta) {
    target[field] = (target[field] ?? '') + addition;
  } else if (addition) {
    // started/updated/completed carry the canonical text — replace the accumulation.
    target[field] = addition;
  }
  if (incoming.output) target.output = (delta ? (target.output ?? '') : '') + incoming.output;
  if (incoming.exitCode != null) target.exitCode = incoming.exitCode;
  target.timestamp = incoming.timestamp;
}

/**
 * Stateful bridge instance. One per app; holds the stream-accumulation index so
 * deltas for the same message land in the same entry across calls.
 */
export function createDaemonFeedBridge(resolveSessionId: SessionResolver) {
  let seq = 0;
  // `${sessionId}:${streamKey}` → index into that session's journal array.
  const streamIndex = new Map<string, number>();

  function ingest(event: DaemonEvent): void {
    if (classifyDaemonEvent(event.type) !== 'render') return;

    const sessionId = resolveSessionId(event);
    if (sessionId == null) return;

    const entry = mapDaemonEvent(event, { sessionId, seq: seq++ });
    if (!entry) return;

    const key = streamKey(event);
    const indexKey = key ? `${sessionId}:${key}` : null;

    journal.update((map) => {
      const existing = map.get(sessionId) ?? [];
      const next = new Map(map);

      if (indexKey && streamIndex.has(indexKey)) {
        const i = streamIndex.get(indexKey)!;
        const arr = [...existing];
        const merged = { ...arr[i] };
        mergeStreamed(merged, entry, isDelta(event.type));
        arr[i] = merged;
        next.set(sessionId, arr);
        return next;
      }

      const arr = [...existing, entry];
      if (indexKey) streamIndex.set(indexKey, arr.length - 1);
      next.set(sessionId, arr);
      return next;
    });
  }

  return { ingest };
}

/**
 * Open an SSE connection to the daemon's `/events` stream and pipe every event
 * through the bridge. Returns a disposer. Opt-in: only called when a daemon URL
 * is configured (VITE_DAEMON_URL).
 */
export function connectDaemonSse(baseUrl: string, resolveSessionId: SessionResolver): () => void {
  const bridge = createDaemonFeedBridge(resolveSessionId);
  const url = `${baseUrl.replace(/\/$/, '')}/events`;
  const es = new EventSource(url);

  const onEvent = (e: Event) => {
    try {
      bridge.ingest(JSON.parse((e as MessageEvent).data) as DaemonEvent);
    } catch {
      // ignore malformed frames
    }
  };

  // The daemon names each frame with its event type, so a generic onmessage
  // (which only fires for unnamed events) would miss them. Subscribe to every
  // type in the catalog explicitly.
  for (const type of Object.keys(EVENT_CLASSIFICATION)) {
    es.addEventListener(type, onEvent);
  }

  return () => es.close();
}
