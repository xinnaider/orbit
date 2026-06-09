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

vi.mock('lucide-svelte', () => {
  const MockIcon = () => ({ $$anchor: null, children: [] });
  return {
    Bot: MockIcon,
    MessageSquare: MockIcon,
    Sparkles: MockIcon,
    FileText: MockIcon,
    FilePen: MockIcon,
    FilePlus: MockIcon,
    Terminal: MockIcon,
    Search: MockIcon,
    Folder: MockIcon,
    Wrench: MockIcon,
    Settings: MockIcon,
    Maximize2: MockIcon,
    Copy: MockIcon,
  };
});

// --- Component import ---

import Feed from './Feed.svelte';

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

describe('Feed', () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
    mockBackendsStore.set([{ id: 'claude-code', name: 'Claude Code', subProviders: [] }]);
    mockProviderCapsStore.set(new Map());
  });

  // ═══════════════════════════════════════
  //  Rendering basics
  // ═══════════════════════════════════════

  it('renders empty when no entries', () => {
    const { container } = render(Feed, {
      props: { entries: [], status: '', provider: 'claude-code', cwd: null },
    });

    const events = container.querySelectorAll('.timeline-event');
    expect(events.length).toBe(0);
  });

  it('renders user entry', () => {
    const entries = [makeEntry({ entryType: 'user', text: 'hello from user' })];
    const { container } = render(Feed, {
      props: { entries, status: '', provider: 'claude-code', cwd: null },
    });

    expect(container.textContent).toContain('hello from user');
  });

  it('renders assistant entry', () => {
    const entries = [makeEntry({ entryType: 'assistant', text: 'hello from assistant' })];
    const { container } = render(Feed, {
      props: { entries, status: '', provider: 'claude-code', cwd: null },
    });

    expect(container.textContent).toContain('hello from assistant');
  });

  it('renders system entry', () => {
    const entries = [makeEntry({ entryType: 'system', text: 'system message' })];
    const { getByText } = render(Feed, {
      props: { entries, status: '', provider: 'claude-code', cwd: null },
    });

    expect(getByText('system message')).toBeTruthy();
  });

  it('shows typing indicator when working', () => {
    const { getByText, container } = render(Feed, {
      props: { entries: [], status: 'working', provider: 'claude-code', cwd: null },
    });

    expect(getByText('working')).toBeTruthy();
    expect(container.querySelector('.typing-dots')).toBeTruthy();
  });

  it('hides typing indicator when status is running or waiting', () => {
    for (const status of ['running', 'waiting'] as const) {
      const { container } = render(Feed, {
        props: { entries: [], status, provider: 'codex', cwd: null },
      });
      expect(container.querySelector('.timeline-event.working')).toBeNull();
      cleanup();
    }
  });

  it('renders toolCall entry', () => {
    const entries = [
      makeEntry({
        entryType: 'toolCall',
        tool: 'bash',
        toolInput: { command: 'echo hi' },
        text: '',
      }),
    ];
    const { container } = render(Feed, {
      props: { entries, status: '', provider: 'claude-code', cwd: null },
    });

    const toolCard = container.querySelector('.tc-card');
    expect(toolCard).toBeTruthy();
    expect(toolCard!.textContent).toContain('bash');
    expect(toolCard!.textContent).toContain('echo hi');
  });

  it('collapses 4+ consecutive same-tool calls into one expandable group', () => {
    const entries = Array.from({ length: 5 }, (_, i) =>
      makeEntry({
        entryType: 'toolCall',
        tool: 'read',
        toolInput: { file_path: `/tmp/file-${i}.ts` },
        seq: i,
        text: '',
      })
    );
    const { container } = render(Feed, {
      props: { entries, status: '', provider: 'claude-code', cwd: null },
    });

    const toggle = container.querySelector('.tool-group-toggle');
    expect(toggle).toBeTruthy();
    expect(toggle!.textContent).toContain('5 read steps');
    // Collapsed by default: tool cards hidden until expanded.
    expect(container.querySelector('.tc-card')).toBeNull();
  });

  it('does not group short runs of the same tool', () => {
    const entries = Array.from({ length: 2 }, (_, i) =>
      makeEntry({ entryType: 'toolCall', tool: 'read', toolInput: { file_path: `/a-${i}` }, seq: i })
    );
    const { container } = render(Feed, {
      props: { entries, status: '', provider: 'claude-code', cwd: null },
    });
    expect(container.querySelector('.tool-group-toggle')).toBeNull();
    expect(container.querySelectorAll('.tc-card').length).toBe(2);
  });

  // ═══════════════════════════════════════
  //  Timeline & Compact
  // ═══════════════════════════════════════

  it('renders entries as Quiet Journal timeline events', () => {
    const entries = [
      makeEntry({ entryType: 'user', text: 'user prompt' }),
      makeEntry({ entryType: 'assistant', text: 'assistant response' }),
      makeEntry({ entryType: 'system', text: 'rate limit warning' }),
    ];
    const { container } = render(Feed, {
      props: { entries, status: '', provider: 'claude-code', cwd: null },
    });

    expect(container.querySelector('.timeline')).toBeTruthy();
    expect(container.querySelectorAll('.timeline-event').length).toBe(3);
    expect(container.querySelector('.timeline-event.user')).toBeTruthy();
    expect(container.querySelector('.timeline-event.assistant')).toBeTruthy();
    expect(container.querySelector('.timeline-event.system')).toBeTruthy();
  });

  it('supports compact timeline density', () => {
    const entries = [makeEntry({ entryType: 'user', text: 'compact prompt' })];
    const { container } = render(Feed, {
      props: { entries, status: '', provider: 'claude-code', cwd: null, compact: true },
    });
    expect(container.querySelector('.feed-scroller.compact')).toBeTruthy();
  });

  it('uses compact class only when compact prop is true', () => {
    const entries = [makeEntry({ entryType: 'assistant', text: 'normal' })];
    const normal = render(Feed, {
      props: { entries, status: '', provider: 'claude-code', cwd: null },
    });
    expect(normal.container.querySelector('.feed-scroller.compact')).toBeNull();
    cleanup();
    const compact = render(Feed, {
      props: { entries, status: '', provider: 'claude-code', cwd: null, compact: true },
    });
    expect(compact.container.querySelector('.feed-scroller.compact')).toBeTruthy();
  });

  it('keeps working indicator inside timeline', () => {
    const { container, getByText } = render(Feed, {
      props: { entries: [], status: 'working', provider: 'claude-code', cwd: null },
    });
    expect(container.querySelector('.timeline-event.working')).toBeTruthy();
    expect(getByText('working')).toBeTruthy();
  });
});
