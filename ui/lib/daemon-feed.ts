/**
 * Homologation layer: maps daemon SDK events (DaemonEvent) into the Feed's
 * JournalEntry model, provider-agnostically.
 *
 * The daemon (agent-daemon) normalizes every provider's raw output into a small,
 * shared `DaemonEvent` vocabulary. Orbit's chat only understands `JournalEntry`.
 * This module is the single bridge between the two — and the single place that
 * decides, for EVERY event type, whether it becomes a visible chat entry
 * (`render`) or is intentionally not shown (`ignore`, e.g. token/usage/control
 * plane events handled by other panels).
 *
 * Invariant: no SDK event is ever silently dropped. Every type in the SDK
 * catalog must appear in {@link EVENT_CLASSIFICATION}. The homologation test
 * fails if a new event type ships without a deliberate render/ignore decision.
 */

import type { JournalEntry, JournalEntryType } from './types';

/** Provider ids the daemon normalizes to. Mirror of daemon `ProviderId`. */
export type ProviderId = 'opencode' | 'codex' | 'claude';

/** Minimal mirror of the daemon `DaemonEvent` contract (domain/events.ts). */
export interface DaemonEvent {
  id: string;
  runId: string;
  provider: ProviderId;
  type: string;
  createdAt: string;
  sequence: number;
  data: Record<string, unknown>;
}

/** Whether an event becomes a chat entry or is intentionally invisible. */
export type FeedClassification = 'render' | 'ignore';

/**
 * Every SDK event type, classified. `render` → produces a JournalEntry;
 * `ignore` → deliberately not a chat line (consumed by tokens/git/tasks panels
 * or pure control-plane lifecycle). Keep alphabetised within each group.
 */
export const EVENT_CLASSIFICATION: Record<string, FeedClassification> = {
  // ── Run lifecycle — drives the "working" indicator, not chat lines ────────
  'run.create': 'ignore',
  'run.created': 'ignore',
  'run.start': 'ignore',
  'run.started': 'ignore',
  'run.completed': 'ignore',
  'run.failed': 'render', // surfaced as a system error line

  // ── Assistant message ─────────────────────────────────────────────────────
  'message.created': 'render',
  'message.started': 'render',
  'message.delta': 'render',
  'message.updated': 'render',
  'message.completed': 'render',
  'message.part.delta': 'render',
  'message.part.updated': 'render',

  // ── Reasoning / thinking ──────────────────────────────────────────────────
  'reasoning.started': 'render',
  'reasoning.delta': 'render',
  'reasoning.completed': 'render',

  // ── Tool calls ────────────────────────────────────────────────────────────
  'tool.create': 'render',
  'tool.start': 'render',
  'tool.started': 'render',
  'tool.delta': 'render', // streaming output → progress entry
  'tool.update': 'render',
  'tool.finish': 'render',
  'tool.completed': 'render',
  'tool.failed': 'render',

  // ── Permission prompts ────────────────────────────────────────────────────
  'permission.request': 'render',
  'permission.requested': 'render',
  'permission.asked': 'render',
  'permission.reply': 'ignore', // resolves a pending prompt, no new line
  'permission.resolved': 'ignore',

  // ── Errors ────────────────────────────────────────────────────────────────
  error: 'render',
  unknown: 'ignore',

  // ── Tokens / files / todos — owned by other panels, not the chat ──────────
  'usage.updated': 'ignore',
  'file.read': 'ignore',
  'file.status': 'ignore',
  'file.changed': 'ignore',
  'todo.updated': 'ignore',

  // ── Session control plane — lifecycle / RPC, never chat lines ─────────────
  'session.abort': 'ignore',
  'session.children': 'ignore',
  'session.command': 'ignore',
  'session.create': 'ignore',
  'session.created': 'ignore',
  'session.delete': 'ignore',
  'session.deleted': 'ignore',
  'session.diff': 'ignore',
  'session.discovered': 'ignore',
  'session.get': 'ignore',
  'session.idle': 'ignore',
  'session.init': 'ignore',
  'session.list': 'ignore',
  'session.message': 'ignore',
  'session.messages': 'ignore',
  'session.next.agent.switched': 'ignore',
  'session.next.model.switched': 'ignore',
  'session.permission.reply': 'ignore',
  'session.prompt': 'ignore',
  'session.revert': 'ignore',
  'session.share': 'ignore',
  'session.shell': 'ignore',
  'session.status': 'ignore',
  'session.summarize': 'ignore',
  'session.unrevert': 'ignore',
  'session.unshare': 'ignore',
  'session.update': 'ignore',
  'session.updated': 'ignore',
  'sessions.info': 'ignore',
  'sessions.list': 'ignore',
  'sessions.messages': 'ignore',
  'sessions.rename': 'ignore',
  'sessions.tag': 'ignore',
};

/**
 * Classify an event type. Returns the render/ignore decision, or `undefined`
 * for an unrecognised type — which the homologation test treats as a failure.
 */
export function classifyDaemonEvent(type: string): FeedClassification | undefined {
  return EVENT_CLASSIFICATION[type];
}

/** Map an SDK event type to the Feed entry type, when it renders. */
function entryTypeFor(type: string): JournalEntryType | null {
  if (type.startsWith('reasoning.')) return 'thinking';
  if (type.startsWith('message.')) return 'assistant';
  if (type === 'tool.delta' || type === 'tool.update') return 'progress';
  if (type === 'tool.finish' || type === 'tool.completed' || type === 'tool.failed') {
    return 'toolResult';
  }
  if (type.startsWith('tool.')) return 'toolCall';
  if (type.startsWith('permission.')) return 'system';
  if (type === 'run.failed' || type === 'error') return 'system';
  return null;
}

function str(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null;
}

function num(value: unknown): number | null {
  return typeof value === 'number' ? value : null;
}

export interface MapContext {
  /** Numeric session id this run belongs to. */
  sessionId: number;
  /** Monotonic sequence for ordering within the feed. */
  seq: number;
}

/**
 * Map a single DaemonEvent to a JournalEntry, or `null` when the event is
 * classified `ignore` (or — defensively — unrecognised).
 */
export function mapDaemonEvent(event: DaemonEvent, ctx: MapContext): JournalEntry | null {
  if (classifyDaemonEvent(event.type) !== 'render') return null;

  const entryType = entryTypeFor(event.type);
  if (!entryType) return null;

  const data = event.data ?? {};
  const isError = event.type === 'run.failed' || event.type === 'error' || event.type === 'tool.failed';

  const text =
    str(data.text) ??
    (isError ? (str(data.message) ?? str(data.error)) : null);

  return {
    sessionId: String(ctx.sessionId),
    timestamp: event.createdAt ?? new Date().toISOString(),
    entryType,
    text,
    thinking: entryType === 'thinking' ? (str(data.text) ?? str(data.thinking)) : null,
    thinkingDuration: entryType === 'thinking' ? num(data.duration) : null,
    tool: str(data.tool) ?? str(data.name),
    toolInput: (data.input as Record<string, unknown> | undefined) ?? null,
    output: str(data.output) ?? str(data.result),
    exitCode: num(data.exitCode),
    linesChanged: null,
    seq: ctx.seq,
    epoch: event.runId ?? '',
    feedError: isError,
  };
}
