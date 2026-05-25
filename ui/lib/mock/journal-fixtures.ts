import type { JournalEntry } from '../types';

/** Fixed values so feed/diff snapshots stay deterministic in mock + E2E. */
const FIXTURE_TIMESTAMP = '2026-05-24T12:00:00.000Z';
const FIXTURE_EPOCH = 'fixture-epoch-1';

let fixtureSeq = 10_000;

const FIXTURE_TRIGGER_RE = /\[fixture:(\w+)\]/gi;

export type FixtureKind = 'edit' | 'write' | 'bash' | 'read';

export function parseFixtureTriggers(message: string): FixtureKind[] | null {
  const matches = [...message.matchAll(FIXTURE_TRIGGER_RE)];
  if (matches.length === 0) return null;

  const kinds: FixtureKind[] = [];
  for (const match of matches) {
    const kind = match[1].toLowerCase() as FixtureKind;
    if (kind === 'edit' || kind === 'write' || kind === 'bash' || kind === 'read') {
      kinds.push(kind);
    }
  }
  return kinds.length > 0 ? kinds : null;
}

function nextSeq(): number {
  return fixtureSeq++;
}

function baseEntry(
  sessionId: number,
  entryType: JournalEntry['entryType'],
  overrides: Partial<JournalEntry>
): JournalEntry {
  return {
    sessionId: String(sessionId),
    timestamp: FIXTURE_TIMESTAMP,
    entryType,
    text: null,
    thinking: null,
    thinkingDuration: null,
    tool: null,
    toolInput: null,
    output: null,
    exitCode: null,
    linesChanged: null,
    seq: nextSeq(),
    epoch: FIXTURE_EPOCH,
    ...overrides,
  };
}

export function buildFixtureUserEntry(sessionId: number, userMsg: string): JournalEntry {
  return baseEntry(sessionId, 'user', { text: userMsg });
}

export function buildFixtureToolPair(
  sessionId: number,
  kind: FixtureKind
): { call: JournalEntry; result: JournalEntry } {
  switch (kind) {
    case 'edit':
      return {
        call: baseEntry(sessionId, 'toolCall', {
          tool: 'Edit',
          toolInput: {
            file_path: 'src/example.ts',
            old_string: 'const value = 1;\n',
            new_string: 'const value = 2;\n',
          },
          linesChanged: { added: 1, removed: 1 },
        }),
        result: baseEntry(sessionId, 'toolResult', {
          output: 'Updated src/example.ts',
          exitCode: 0,
        }),
      };
    case 'write':
      return {
        call: baseEntry(sessionId, 'toolCall', {
          tool: 'Write',
          toolInput: {
            file_path: 'src/new-file.ts',
            content: 'export const created = true;\n',
          },
          linesChanged: { added: 1, removed: 0 },
        }),
        result: baseEntry(sessionId, 'toolResult', {
          output: 'Wrote src/new-file.ts',
          exitCode: 0,
        }),
      };
    case 'bash':
      return {
        call: baseEntry(sessionId, 'toolCall', {
          tool: 'Bash',
          toolInput: { command: 'echo "fixture-bash-output"' },
        }),
        result: baseEntry(sessionId, 'toolResult', {
          output: 'fixture-bash-output\n',
          exitCode: 0,
        }),
      };
    case 'read':
      return {
        call: baseEntry(sessionId, 'toolCall', {
          tool: 'Read',
          toolInput: { file_path: 'README.md' },
        }),
        result: baseEntry(sessionId, 'toolResult', {
          output: '     1→# Orbit\n     2→E2E fixture read output\n',
          exitCode: 0,
        }),
      };
  }
}

export function buildFixtureAssistantEntry(sessionId: number, kinds: FixtureKind[]): JournalEntry {
  return baseEntry(sessionId, 'assistant', {
    text: `Fixture response (${kinds.join(', ')}) for browser testing.`,
  });
}
