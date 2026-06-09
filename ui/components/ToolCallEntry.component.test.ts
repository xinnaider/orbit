import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import type { JournalEntry } from '../lib/types';

// --- Mock stores and helpers ---

const { mockBackendsStore, mockProviderCapsStore } = vi.hoisted(() => {
  function createStore<T>(initial: T) {
    let val = initial;
    const subs = new Set<(v: T) => void>();
    return {
      subscribe: (fn: (v: T) => void) => {
        subs.add(fn);
        fn(val);
        return () => subs.delete(fn);
      },
      set: (v: T) => {
        val = v;
        subs.forEach((fn) => fn(val));
      },
      update: (updater: (v: T) => T) => {
        val = updater(val);
        subs.forEach((fn) => fn(val));
      },
    };
  }
  return {
    mockBackendsStore: createStore<any[]>([]),
    mockProviderCapsStore: createStore<Map<string, any>>(new Map()),
  };
});

vi.mock('$lib/stores/providers', () => ({
  backends: mockBackendsStore,
  providerCaps: mockProviderCapsStore,
  getCaps: vi.fn(() => ({
    supportsEffort: false,
    supportsSsh: false,
    supportsSubagents: false,
    supportsTasks: false,
    hasSubProviders: false,
    effortLevels: {},
  })),
}));

vi.mock('$lib/tauri/files', () => ({
  readFileContent: vi.fn().mockRejectedValue(new Error('no file')),
}));

vi.mock('lucide-svelte', () => {
  const MockIcon = () => ({ $$anchor: null, children: [] });
  return {
    FileText: MockIcon,
    FilePen: MockIcon,
    FilePlus: MockIcon,
    Terminal: MockIcon,
    Search: MockIcon,
    Folder: MockIcon,
    Bot: MockIcon,
    Wrench: MockIcon,
    Settings: MockIcon,
    Maximize2: MockIcon,
    Copy: MockIcon,
  };
});

// --- Component import ---

import ToolCallEntry from './ToolCallEntry.svelte';

// --- Helpers ---

function makeEntry(overrides: Partial<JournalEntry> & { entryType: string }): JournalEntry {
  return {
    sessionId: '1',
    timestamp: new Date().toISOString(),
    text: '',
    thinking: null,
    thinkingDuration: null,
    tool: null,
    toolInput: null,
    output: null,
    exitCode: null,
    linesChanged: null,
    seq: 0,
    epoch: '',
    ...overrides,
  };
}

// --- Tests ---

describe('ToolCallEntry', () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
    mockBackendsStore.set([{ id: 'claude-code', name: 'Claude Code', subProviders: [] }]);
    mockProviderCapsStore.set(new Map());
  });

  it('renders bash tool name and command', () => {
    const entry = makeEntry({
      entryType: 'toolCall',
      tool: 'bash',
      toolInput: { command: 'ls -la' },
    });
    const { container, getByText } = render(ToolCallEntry, {
      props: { entry, resultEntry: null, streamingEntries: [], cwd: null },
    });

    expect(getByText('bash')).toBeTruthy();
    expect(container.textContent).toContain('ls -la');
  });

  it('renders write tool with content', () => {
    const entry = makeEntry({
      entryType: 'toolCall',
      tool: 'write',
      toolInput: {
        file_path: '/tmp/test.txt',
        content: 'hello world\nline two',
      },
    });
    const { container } = render(ToolCallEntry, {
      props: { entry, resultEntry: null, streamingEntries: [], cwd: null },
    });

    expect(container.textContent).toContain('write');
    expect(container.textContent).toContain('hello world');
    expect(container.textContent).toContain('line two');
  });

  it('renders edit tool with old/new string', () => {
    const entry = makeEntry({
      entryType: 'toolCall',
      tool: 'edit',
      toolInput: {
        file_path: '/tmp/test.ts',
        old_string: 'old code',
        new_string: 'new code',
      },
    });
    const { container } = render(ToolCallEntry, {
      props: { entry, resultEntry: null, streamingEntries: [], cwd: null },
    });

    expect(container.textContent).toContain('edit');
    expect(container.textContent).toContain('old code');
    expect(container.textContent).toContain('new code');
  });

  it('renders read tool entry', () => {
    const entry = makeEntry({
      entryType: 'toolCall',
      tool: 'read',
      toolInput: {
        file_path: '/tmp/readme.md',
      },
    });
    const { getByText } = render(ToolCallEntry, {
      props: { entry, resultEntry: null, streamingEntries: [], cwd: null },
    });

    expect(getByText('read')).toBeTruthy();
  });

  it('renders quiet tool card with state label', () => {
    const entry = makeEntry({
      entryType: 'toolCall',
      tool: 'bash',
      toolInput: { command: 'npm test' },
    });
    const result = makeEntry({
      entryType: 'toolResult',
      output: '20 passed',
      exitCode: 0,
    });
    const { container, getByText } = render(ToolCallEntry, {
      props: { entry, resultEntry: result, streamingEntries: [], cwd: null },
    });

    expect(container.querySelector('.quiet-tool-card')).toBeTruthy();
    expect(getByText('bash')).toBeTruthy();
    // State is conveyed by the exit code + card styling, not a text/dot label.
    expect(container.textContent).toContain('exit 0');
    expect(container.textContent).toContain('npm test');
    expect(container.textContent).toContain('20 passed');
  });

  it('shows streaming progress before tool result', () => {
    const entry = makeEntry({
      entryType: 'toolCall',
      tool: 'bash',
      toolInput: { command: 'npm test' },
    });
    const streaming = [
      makeEntry({
        entryType: 'progress',
        text: 'running tests...',
      }),
    ];
    const { container } = render(ToolCallEntry, {
      props: { entry, resultEntry: null, streamingEntries: streaming, cwd: null },
    });

    expect(container.textContent).toContain('running tests');
    expect(container.querySelector('.streaming-output')).toBeTruthy();
  });

  it('shows streaming edit diff preview before tool result', () => {
    const entry = makeEntry({
      entryType: 'toolCall',
      tool: 'edit',
      toolInput: { file_path: '/tmp/a.ts' },
    });
    const streaming = [
      makeEntry({
        entryType: 'progress',
        tool: 'edit',
        toolInput: {
          file_path: '/tmp/a.ts',
          old_string: 'before',
          new_string: 'after',
        },
      }),
    ];
    const { container } = render(ToolCallEntry, {
      props: { entry, resultEntry: null, streamingEntries: streaming, cwd: null },
    });

    expect(container.textContent).toContain('before');
    expect(container.textContent).toContain('after');
    expect(container.querySelector('.stream-diff-preview')).toBeTruthy();
  });

  it('renders compact quiet tool card', () => {
    const entry = makeEntry({
      entryType: 'toolCall',
      tool: 'read',
      toolInput: { file_path: '/tmp/readme.md' },
    });
    const { container } = render(ToolCallEntry, {
      props: { entry, resultEntry: null, streamingEntries: [], cwd: null, compact: true },
    });
    expect(container.querySelector('.quiet-tool-card.compact')).toBeTruthy();
  });

  it('clamps long tool output and shows an expand affordance', () => {
    const output = Array.from({ length: 30 }, (_, i) => `line ${i + 1}`).join('\n');
    const entry = makeEntry({
      entryType: 'toolCall',
      tool: 'bash',
      toolInput: { command: 'seq 30' },
    });
    const result = makeEntry({ entryType: 'toolResult', tool: 'bash', output, exitCode: 0 });
    const { container } = render(ToolCallEntry, {
      props: { entry, resultEntry: result, streamingEntries: [], cwd: null },
    });

    const view = container.querySelector('.term-view.clamped');
    expect(view).toBeTruthy();
    expect(view!.querySelectorAll('.term-text').length).toBe(14);
    expect(container.querySelector('.expand-pill')?.textContent).toContain('Show all 30 lines');
  });

  it('compact mode clamps output tighter (8 lines)', () => {
    const output = Array.from({ length: 30 }, (_, i) => `line ${i + 1}`).join('\n');
    const entry = makeEntry({ entryType: 'toolCall', tool: 'bash', toolInput: { command: 'x' } });
    const result = makeEntry({ entryType: 'toolResult', tool: 'bash', output, exitCode: 0 });
    const { container } = render(ToolCallEntry, {
      props: { entry, resultEntry: result, streamingEntries: [], cwd: null, compact: true },
    });
    expect(
      container.querySelector('.term-view.clamped')!.querySelectorAll('.term-text').length
    ).toBe(8);
  });
});
