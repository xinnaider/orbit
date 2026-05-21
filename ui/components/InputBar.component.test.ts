import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, waitFor, cleanup } from '@testing-library/svelte';

// --- Mock helpers and vi.mock calls ---

const {
  mockSessionsStore,
  mockJournalStore,
  mockBackendsStore,
  mockProviderCapsStore,
  mockSessionEffortObj,
  mockMessageHistory,
  mockPendingMessages,
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
    mockSessionsStore: createStore<any[]>([]),
    mockJournalStore: createStore<Map<number, any[]>>(new Map()),
    mockBackendsStore: createStore<any[]>([]),
    mockProviderCapsStore: createStore<Map<string, any>>(new Map()),
    mockSessionEffortObj: {
      subscribe: createStore({} as Record<string, string>).subscribe,
      get: vi.fn(() => 'high'),
      set: vi.fn(),
    },
    mockMessageHistory: {
      push: vi.fn(),
      up: vi.fn(() => null),
      down: vi.fn(() => null),
      resetCursor: vi.fn(),
    },
    mockPendingMessages: {
      subscribe: vi.fn(() => {
        // Return a no-op unsubscribe function
        const fn = () => {};
        return fn;
      }),
      add: vi.fn(),
      clear: vi.fn(),
      remove: vi.fn(),
    },
  };
});

vi.mock('$lib/tauri', () => ({
  sendSessionMessage: vi.fn().mockResolvedValue(undefined),
  getSlashCommands: vi.fn().mockResolvedValue([]),
  listProjectFiles: vi.fn().mockResolvedValue([]),
  updateSessionModel: vi.fn().mockResolvedValue(undefined),
  updateSessionEffort: vi.fn().mockResolvedValue(undefined),
  stopSession: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('$lib/stores/history', () => ({
  messageHistory: mockMessageHistory,
}));

vi.mock('$lib/stores/sessions', () => ({
  sessions: mockSessionsStore,
  updateSessionState: vi.fn((list, id, patch) =>
    list.map((s: any) => (s.id === id ? { ...s, ...patch } : s))
  ),
}));

vi.mock('$lib/stores/journal', () => ({
  journal: mockJournalStore,
  pendingMessages: mockPendingMessages,
}));

vi.mock('$lib/stores/ui', () => ({
  sessionEffort: mockSessionEffortObj,
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

// Note: SlashCommandPicker.svelte is not mocked — it renders fine in tests
// and is needed for bind:this={picker} to work with Svelte 5's component API.

// --- Svelte component import ---

import InputBar from './InputBar.svelte';

// --- Tests ---

describe('InputBar', () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
    mockSessionsStore.set([]);
    mockJournalStore.set(new Map());
    mockBackendsStore.set([]);
    mockProviderCapsStore.set(new Map());
    mockSessionEffortObj.get.mockReturnValue('high');
    mockMessageHistory.push.mockClear();
    mockMessageHistory.up.mockReturnValue(null);
    mockMessageHistory.down.mockReturnValue(null);
    mockMessageHistory.resetCursor.mockClear();
  });

  // ═══════════════════════════════════════
  //  Rendering basics
  // ═══════════════════════════════════════

  it('renders message input and send button', () => {
    const { getByTestId } = render(InputBar, {
      props: {
        sessionId: 1,
        cwd: '/test',
        sessionStatus: 'running',
        provider: 'claude-code',
        providerModels: [],
      },
    });

    expect(getByTestId('message-input')).toBeTruthy();
    expect(getByTestId('send-message-button')).toBeTruthy();
  });

  it('send button disabled when text empty', () => {
    const { getByTestId } = render(InputBar, {
      props: {
        sessionId: 1,
        cwd: '/test',
        sessionStatus: 'running',
        provider: 'claude-code',
        providerModels: [],
      },
    });

    const btn = getByTestId('send-message-button') as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  it('typing in textarea updates value', async () => {
    const { getByTestId } = render(InputBar, {
      props: {
        sessionId: 1,
        cwd: '/test',
        sessionStatus: 'running',
        provider: 'claude-code',
        providerModels: [],
      },
    });

    const textarea = getByTestId('message-input') as HTMLTextAreaElement;
    await fireEvent.input(textarea, { target: { value: 'hello world' } });

    expect(textarea.value).toBe('hello world');
  });

  it('send button enabled when text present', async () => {
    const { getByTestId } = render(InputBar, {
      props: {
        sessionId: 1,
        cwd: '/test',
        sessionStatus: 'running',
        provider: 'claude-code',
        providerModels: [],
      },
    });

    const textarea = getByTestId('message-input') as HTMLTextAreaElement;
    await fireEvent.input(textarea, { target: { value: 'hello' } });

    const btn = getByTestId('send-message-button') as HTMLButtonElement;
    expect(btn.disabled).toBe(false);
  });

  // ═══════════════════════════════════════
  //  Send message
  // ═══════════════════════════════════════

  it('Enter key sends message', async () => {
    const { sendSessionMessage } = await import('$lib/tauri');
    const { getByTestId } = render(InputBar, {
      props: {
        sessionId: 42,
        cwd: '/test',
        sessionStatus: 'running',
        provider: 'claude-code',
        providerModels: [],
      },
    });

    const textarea = getByTestId('message-input') as HTMLTextAreaElement;
    await fireEvent.input(textarea, { target: { value: 'test message' } });
    await fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });

    await waitFor(() => {
      expect(sendSessionMessage).toHaveBeenCalledWith(42, 'test message');
    });
  });

  // ═══════════════════════════════════════
  //  Placeholder behavior
  // ═══════════════════════════════════════

  it('placeholder shows "waiting..." when initializing', () => {
    const { getByTestId } = render(InputBar, {
      props: {
        sessionId: 1,
        cwd: '/test',
        sessionStatus: 'initializing',
        provider: 'claude-code',
        providerModels: [],
      },
    });

    const textarea = getByTestId('message-input') as HTMLTextAreaElement;
    expect(textarea.placeholder.toLowerCase()).toContain('waiting');
  });

  it('placeholder shows "stopped" hint when stopped', () => {
    const { getByTestId } = render(InputBar, {
      props: {
        sessionId: 1,
        cwd: '/test',
        sessionStatus: 'stopped',
        provider: 'claude-code',
        providerModels: [],
      },
    });

    const textarea = getByTestId('message-input') as HTMLTextAreaElement;
    expect(textarea.placeholder.toLowerCase()).toContain('stopped');
  });

  it('send-message-button disabled when initializing', () => {
    const { getByTestId } = render(InputBar, {
      props: {
        sessionId: 1,
        cwd: '/test',
        sessionStatus: 'initializing',
        provider: 'claude-code',
        providerModels: [],
      },
    });

    const btn = getByTestId('send-message-button') as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  // ═══════════════════════════════════════
  //  Quiet Composer
  // ═══════════════════════════════════════

  it('renders Quiet Journal composer controls', () => {
    const { container, getByText } = render(InputBar, {
      props: {
        sessionId: 1,
        cwd: 'C:/orbit',
        sessionStatus: 'running',
        provider: 'claude-code',
        providerModels: [],
      },
    });

    expect(container.querySelector('.quiet-composer')).toBeTruthy();
    expect(getByText('@ file')).toBeTruthy();
    expect(getByText('/ command')).toBeTruthy();
  });

  it('renders compact composer variant', () => {
    const { container } = render(InputBar, {
      props: {
        sessionId: 1,
        cwd: 'C:/orbit',
        sessionStatus: 'running',
        provider: 'claude-code',
        providerModels: [],
        compact: true,
      },
    });

    expect(container.querySelector('.quiet-composer.compact')).toBeTruthy();
  });
});
