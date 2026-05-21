import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, cleanup } from '@testing-library/svelte';

// --- Mock helpers and vi.mock calls ---

const { mockProviderCapsStore, mockSessionsStore, mockMetaPanelVisible } = vi.hoisted(() => {
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
    mockProviderCapsStore: createStore<Map<string, any>>(new Map()),
    mockSessionsStore: createStore<any[]>([]),
    mockMetaPanelVisible: createStore<boolean>(true),
  };
});

vi.mock('$lib/stores/preferences', () => ({
  metaPanelVisible: mockMetaPanelVisible,
}));

vi.mock('$lib/stores/ui', () => ({
  sessionEffort: {
    subscribe: vi.fn(() => {
      const fn = () => {};
      return fn;
    }),
    get: vi.fn(() => 'medium'),
    set: vi.fn(),
  },
}));

vi.mock('$lib/stores/providers', () => ({
  providerCaps: mockProviderCapsStore,
  getCaps: vi.fn(() => ({
    supportsEffort: true,
    supportsTasks: true,
    supportsSubagents: true,
    supportsSsh: false,
    hasSubProviders: false,
    effortLevels: {},
  })),
}));

vi.mock('$lib/stores/sessions', () => ({
  sessions: mockSessionsStore,
  updateSessionState: vi.fn((list, id, patch) =>
    list.map((s: any) => (s.id === id ? { ...s, ...patch } : s))
  ),
}));

vi.mock('$lib/tauri', () => ({
  stopSession: vi.fn(),
  getSubagents: vi.fn().mockResolvedValue([]),
}));

vi.mock('$lib/cost', () => ({
  formatTokens: vi.fn((n: number) => {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${Math.round(n / 1_000)}K`;
    return String(n);
  }),
}));

vi.mock('$lib/status', () => ({
  isActive: vi.fn((status: string) =>
    ['working', 'running', 'input', 'waiting', 'initializing'].includes(status)
  ),
  modelShortName: vi.fn((model: string | null) => {
    if (!model) return '—';
    if (model === 'claude-sonnet-4-6') return 'Sonnet 4.6';
    return model;
  }),
}));

vi.mock('./TasksList.svelte', async () => {
  return await import('./__mocks__/TasksList.svelte');
});

vi.mock('./SubagentsPanel.svelte', async () => {
  return await import('./__mocks__/SubagentsPanel.svelte');
});

// --- Svelte component import ---

import MetaPanel from './MetaPanel.svelte';

// --- Test data ---

import type { Session } from '../lib/stores/sessions';

function makeSession(overrides: Partial<Session> = {}): Session {
  return {
    id: 1,
    status: 'running',
    model: 'claude-sonnet-4-6',
    provider: 'claude-code',
    pid: 12345,
    cwd: '/home/user/project',
    permissionMode: 'ignore',
    tokens: { input: 500, output: 200, cacheRead: 50, cacheWrite: 30 },
    contextPercent: 45,
    contextWindow: 200000,
    miniLog: [{ tool: 'bash', target: 'npm test', success: true, result: 'passed' }],
    subagents: [],
    name: 'test-session',
    projectName: 'my-project',
    projectId: null,
    gitBranch: null,
    worktreePath: null,
    branchName: null,
    pendingApproval: null,
    sshHost: null,
    sshUser: null,
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-01T00:00:00Z',
    ...overrides,
  };
}

// --- Tests ---

describe('MetaPanel', () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
    mockSessionsStore.set([]);
    mockProviderCapsStore.set(new Map());
  });

  // ═══════════════════════════════════════
  //  Stats tab (default)
  // ═══════════════════════════════════════

  it('renders stats tab by default', () => {
    const session = makeSession();
    const { getByText } = render(MetaPanel, { props: { session } });

    expect(getByText('tokens')).toBeTruthy();
    expect(getByText('context')).toBeTruthy();
    expect(getByText('Sonnet 4.6')).toBeTruthy();
  });

  it('shows input/output tokens', () => {
    const session = makeSession();
    const { getByText } = render(MetaPanel, { props: { session } });

    expect(getByText('500')).toBeTruthy(); // input
    expect(getByText('200')).toBeTruthy(); // output
  });

  it('shows context bar', () => {
    const session = makeSession();
    const { getByText } = render(MetaPanel, { props: { session } });

    expect(getByText('45%')).toBeTruthy();
  });

  it('shows recent tools', () => {
    const session = makeSession();
    const { getByText } = render(MetaPanel, { props: { session } });

    expect(getByText('bash')).toBeTruthy();
  });

  it('shows model name', () => {
    const session = makeSession();
    const { getByText } = render(MetaPanel, { props: { session } });

    expect(getByText('Sonnet 4.6')).toBeTruthy();
  });

  it('shows PID', () => {
    const session = makeSession();
    const { getByText } = render(MetaPanel, { props: { session } });

    expect(getByText('12345')).toBeTruthy();
  });

  // ═══════════════════════════════════════
  //  Tab switching
  // ═══════════════════════════════════════

  it('switches to tasks tab', async () => {
    const session = makeSession();
    const { getByText } = render(MetaPanel, { props: { session } });

    await fireEvent.click(getByText('tasks'));

    expect(getByText('TasksList')).toBeTruthy();
  });

  it('switches to agents tab', async () => {
    const session = makeSession();
    const { getByText } = render(MetaPanel, { props: { session } });

    await fireEvent.click(getByText('agents'));

    expect(getByText('SubagentsPanel')).toBeTruthy();
  });

  // ═══════════════════════════════════════
  //  Conditional rendering
  // ═══════════════════════════════════════

  // ═══════════════════════════════════════
  //  Inspector default (hidden)
  // ═══════════════════════════════════════

  it('renders as quiet inspector surface', () => {
    const session = makeSession({ id: 1, name: 'Inspector Session', status: 'running' });
    const { container, getByText } = render(MetaPanel, { props: { session } });
    expect(container.querySelector('.meta.inspector')).toBeTruthy();
    expect(getByText('stats')).toBeTruthy();
  });

  it('hides tasks tab when caps disables it', async () => {
    // Override getCaps for this test
    const { getCaps } = await import('$lib/stores/providers');
    vi.mocked(getCaps).mockReturnValue({
      supportsEffort: true,
      supportsTasks: false,
      supportsSubagents: true,
      supportsSsh: false,
      hasSubProviders: false,
      effortLevels: {},
    });

    const session = makeSession();
    const { queryByText } = render(MetaPanel, { props: { session } });

    expect(queryByText('tasks')).toBeNull();
  });
});
