import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, cleanup } from '@testing-library/svelte';

// --- Mock helpers and vi.mock calls ---

const {
  mockSessionsStore,
  mockWorkspaceStore,
  mockMutedSessionsObj,
  mockSessionEffortObj,
  mockPinnedSessionsObj,
  mockBackendsStore,
  mockProviderCapsStore,
} = vi.hoisted(() => {
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
    mockSessionsStore: createStore<any[]>([]),
    mockWorkspaceStore: createStore<any>({
      root: { type: 'leaf', paneId: 'p1' },
      panes: { p1: { tabs: [], activeTabId: null } },
      focusedPaneId: 'p1',
    }),
    mockMutedSessionsObj: {
      subscribe: createStore(new Set<string>()).subscribe,
      isMuted: vi.fn((set: Set<string>, id: string) => set.has(id)),
      toggle: vi.fn(),
    },
    mockSessionEffortObj: {
      subscribe: createStore({} as Record<string, string>).subscribe,
      get: vi.fn(() => 'high'),
      set: vi.fn(),
    },
    mockPinnedSessionsObj: {
      subscribe: createStore(new Set<string>()).subscribe,
      toggle: vi.fn(),
      isPinned: vi.fn((set: Set<string>, id: string) => set.has(id)),
    },
  };
});

vi.mock('$lib/tauri/invoke', () => ({
  HAS_TAURI: false,
}));

vi.mock('$lib/tauri', () => ({
  deleteSession: vi.fn(),
  stopSession: vi.fn(),
  getAppVersion: vi.fn().mockResolvedValue('1.0.0'),
  getProviders: vi.fn().mockResolvedValue([]),
  diagnoseProvider: vi.fn(),
}));

vi.mock('$lib/tauri/attention', () => ({
  clearAttention: vi.fn(),
}));

vi.mock('$lib/stores/sessions', () => ({
  sessions: mockSessionsStore,
  updateSessionState: vi.fn((list, id, patch) =>
    list.map((s: any) => (s.id === id ? { ...s, ...patch } : s))
  ),
}));

vi.mock('$lib/stores/workspace', () => ({
  workspace: mockWorkspaceStore,
  assignSession: vi.fn(),
  splitPane: vi.fn(),
}));

vi.mock('$lib/stores/session-actions', () => ({
  upsertAndOpenSession: vi.fn(),
}));

vi.mock('$lib/stores/ui', () => ({
  mutedSessions: mockMutedSessionsObj,
  sessionEffort: mockSessionEffortObj,
  pinnedSessions: mockPinnedSessionsObj,
}));

vi.mock('$lib/stores/preferences', () => {
  function createPrefStore<T>(initial: T) {
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
    };
  }
  return {
    sidebarVisible: createPrefStore(true),
  };
});

vi.mock('$lib/status', () => ({
  statusColor: vi.fn((_status: string) => 'var(--s-working)'),
  statusLabel: vi.fn((status: string) => status),
  isPulsing: vi.fn(() => false),
  modelShortName: vi.fn((model: string | null) => model ?? '—'),
}));

vi.mock('$lib/cost', () => ({
  formatTokens: vi.fn((n: number) => {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${Math.round(n / 1_000)}K`;
    return String(n);
  }),
}));

vi.mock('$lib/tauri/providers', () => ({
  saveProviderKey: vi.fn(() => Promise.resolve()),
  loadProviderKey: vi.fn(() => Promise.resolve(null)),
}));

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

vi.mock('$lib/assets/orbit.svg?raw', () => ({
  default: '',
}));

// --- Svelte component import ---

import Sidebar from './Sidebar.svelte';

// --- Test data ---

const baseSession = {
  id: 1,
  status: 'initializing' as const,
  name: null,
  projectId: null,
  permissionMode: 'ignore',
  model: null,
  provider: 'claude-code',
  pid: null,
  cwd: null,
  projectName: null,
  gitBranch: null,
  worktreePath: null,
  branchName: null,
  tokens: null,
  contextPercent: null,
  pendingApproval: null,
  miniLog: null,
  sshHost: null,
  sshUser: null,
  createdAt: '2025-01-01T00:00:00Z',
  updatedAt: '2025-01-01T00:00:00Z',
};

function makeSession(overrides: Partial<typeof baseSession> = {}) {
  return { ...baseSession, ...overrides };
}

// --- Tests ---

describe('Sidebar', () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
    mockSessionsStore.set([]);
    mockBackendsStore.set([]);
    mockProviderCapsStore.set(new Map());
    mockWorkspaceStore.set({
      root: { type: 'leaf', paneId: 'p1' },
      panes: { p1: { tabs: [], activeTabId: null } },
      focusedPaneId: 'p1',
    });
  });

  // ── Empty state ──

  it('renders empty state', () => {
    const { getByText, getByTestId } = render(Sidebar);

    expect(getByText('No sessions yet')).toBeTruthy();
    expect(getByTestId('new-session-button')).toBeTruthy();
    expect(getByText(/drag sessions into panes/i)).toBeTruthy();
  });

  // ── Session list ──

  it('renders session list', () => {
    const session = makeSession({ id: 1, name: 'My Session', projectName: 'My Project' });
    mockSessionsStore.set([session]);

    const { getByText } = render(Sidebar);

    expect(getByText('My Session')).toBeTruthy();
  });

  // ── New session button opens modal ──

  it('new session button opens modal', async () => {
    const { getByTestId, queryByTestId } = render(Sidebar);

    // Modal should not be visible initially
    expect(queryByTestId('new-session-path')).toBeNull();

    // Click new session button
    await fireEvent.click(getByTestId('new-session-button'));

    // Modal should now be visible
    expect(getByTestId('new-session-path')).toBeTruthy();
  });

  // ── Quiet Console sidebar landmarks ──

  it('renders Quiet Console sidebar landmarks', () => {
    mockSessionsStore.set([
      makeSession({
        id: 1,
        name: 'Refactor billing flow',
        model: 'claude-sonnet-4-6',
        gitBranch: 'feature/auth',
        status: 'running',
        contextPercent: 41,
      }),
    ]);

    const { getByText, getByTestId, queryByText } = render(Sidebar);

    expect(getByTestId('orbit-brand-icon')).toBeTruthy();
    expect(getByText('orbit')).toBeTruthy();
    expect(getByText('Recent sessions')).toBeTruthy();
    expect(getByText('Refactor billing flow')).toBeTruthy();
    expect(getByText('feature/auth')).toBeTruthy();
    expect(getByText('41% ctx')).toBeTruthy();
    expect(queryByText('tokens')).toBeNull();
  });

  // ── Footer quiet workspace hints ──

  it('footer shows quiet workspace hints instead of only session count', () => {
    mockSessionsStore.set([makeSession({ id: 1, name: 'One' })]);
    const { getByText } = render(Sidebar);
    expect(getByText(/drag sessions into panes/i)).toBeTruthy();
    expect(getByText(/⌘I inspect/i)).toBeTruthy();
  });

  // ── Session search ──

  it('session search input accepts text and filters the list', async () => {
    mockSessionsStore.set([
      makeSession({ id: 1, name: 'Billing refactor' }),
      makeSession({ id: 2, name: 'Auth migration' }),
    ]);

    const { getByTestId, getByText, queryByText } = render(Sidebar);
    const search = getByTestId('session-search-input') as HTMLInputElement;

    expect(getByText('Billing refactor')).toBeTruthy();
    expect(getByText('Auth migration')).toBeTruthy();

    await fireEvent.input(search, { target: { value: 'billing' } });

    expect(getByText('Billing refactor')).toBeTruthy();
    expect(queryByText('Auth migration')).toBeNull();
  });

  it('session search shows empty state when nothing matches', async () => {
    mockSessionsStore.set([makeSession({ id: 1, name: 'Only session' })]);

    const { getByTestId, getByText, queryByText } = render(Sidebar);
    const search = getByTestId('session-search-input') as HTMLInputElement;

    await fireEvent.input(search, { target: { value: 'zzznomatch' } });

    expect(queryByText('Only session')).toBeNull();
    expect(getByText('No matching sessions')).toBeTruthy();
  });

  it('session search by branch label filters sessions', async () => {
    mockSessionsStore.set([
      makeSession({ id: 1, name: 'Alpha', gitBranch: 'feature/payments' }),
      makeSession({ id: 2, name: 'Beta', gitBranch: 'main' }),
    ]);

    const { getByTestId, getByText, queryByText } = render(Sidebar);
    const search = getByTestId('session-search-input') as HTMLInputElement;

    await fireEvent.input(search, { target: { value: 'payments' } });

    expect(getByText('Alpha')).toBeTruthy();
    expect(queryByText('Beta')).toBeNull();
  });
});
