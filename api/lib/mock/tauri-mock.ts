import type { Session, TokenUsage } from '../stores/sessions';
import type { JournalEntry, SlashCommand } from '../types';

const MOCK_SESSIONS: Session[] = [
  {
    id: 1,
    projectId: 1,
    name: 'fix auth bug',
    status: 'running' as const,
    permissionMode: 'ignore',
    model: 'claude-sonnet-4-6',
    pid: 12345,
    cwd: 'C:\\Users\\dev\\api-server',
    projectName: 'api-server',
    gitBranch: 'main',
    tokens: { input: 24800, output: 3400, cacheRead: 12000, cacheWrite: 800 },
    contextPercent: 14.2,
    pendingApproval: null,
    miniLog: [
      { tool: 'Read', target: 'auth.ts', result: null, success: true },
      { tool: 'Bash', target: 'git status', result: null, success: true },
    ],
    costUsd: null,
    worktreePath: null,
    branchName: null,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
  {
    id: 2,
    projectId: 2,
    name: null,
    status: 'waiting' as const,
    permissionMode: 'approve',
    model: 'claude-opus-4-6',
    pid: 23456,
    cwd: 'C:\\Users\\dev\\dashboard',
    projectName: 'dashboard',
    gitBranch: 'feat/redesign',
    tokens: { input: 89200, output: 12300, cacheRead: 45000, cacheWrite: 3200 },
    contextPercent: 51.3,
    pendingApproval: null,
    miniLog: null,
    costUsd: null,
    worktreePath: null,
    branchName: null,
    createdAt: new Date(Date.now() - 60000).toISOString(),
    updatedAt: new Date().toISOString(),
  },
  {
    id: 3,
    projectId: 3,
    name: 'add tests',
    status: 'completed',
    permissionMode: 'ignore',
    model: 'claude-haiku-4-5-20251001',
    pid: null,
    cwd: 'C:\\Users\\dev\\utils',
    projectName: 'utils-lib',
    gitBranch: null,
    tokens: { input: 5200, output: 980, cacheRead: 2100, cacheWrite: 120 },
    contextPercent: 3.1,
    pendingApproval: null,
    miniLog: null,
    costUsd: null,
    worktreePath: null,
    branchName: null,
    createdAt: new Date(Date.now() - 300000).toISOString(),
    updatedAt: new Date(Date.now() - 60000).toISOString(),
  },
];

const MOCK_JOURNAL: Record<number, JournalEntry[]> = {
  1: [
    {
      sessionId: '1',
      timestamp: new Date(Date.now() - 120000).toISOString(),
      entryType: 'user',
      text: 'Fix the JWT token refresh bug in auth.ts — tokens expire too early',
      thinking: null,
      thinkingDuration: null,
      tool: null,
      toolInput: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    },
    {
      sessionId: '1',
      timestamp: new Date(Date.now() - 115000).toISOString(),
      entryType: 'thinking',
      thinking:
        "Let me analyze the JWT token refresh logic. The issue is likely in the expiry calculation...\n\nI need to check:\n1. How the token expiry is set\n2. Where the refresh logic is\n3. Whether there's a race condition",
      thinkingDuration: 3.2,
      text: null,
      tool: null,
      toolInput: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    },
    {
      sessionId: '1',
      timestamp: new Date(Date.now() - 111000).toISOString(),
      entryType: 'assistant',
      text: "I'll look at the auth.ts file to understand the token refresh logic.",
      thinking: null,
      thinkingDuration: null,
      tool: null,
      toolInput: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    },
    {
      sessionId: '1',
      timestamp: new Date(Date.now() - 110000).toISOString(),
      entryType: 'toolCall',
      tool: 'Read',
      toolInput: { file_path: 'src/auth/auth.ts' },
      text: null,
      thinking: null,
      thinkingDuration: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    },
    {
      sessionId: '1',
      timestamp: new Date(Date.now() - 109000).toISOString(),
      entryType: 'toolResult',
      tool: null,
      toolInput: null,
      output: `export function refreshToken(token: string) {\n  const decoded = jwt.decode(token);\n  // BUG: expires in 1 second instead of 1 hour\n  const newToken = jwt.sign(decoded, SECRET, { expiresIn: '1s' });\n  return newToken;\n}`,
      text: null,
      thinking: null,
      thinkingDuration: null,
      exitCode: 0,
      linesChanged: null,
    },
    {
      sessionId: '1',
      timestamp: new Date(Date.now() - 105000).toISOString(),
      entryType: 'toolCall',
      tool: 'Edit',
      toolInput: {
        file_path: 'src/auth/auth.ts',
        old_string: "expiresIn: '1s'",
        new_string: "expiresIn: '1h'",
      },
      text: null,
      thinking: null,
      thinkingDuration: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    },
    {
      sessionId: '1',
      timestamp: new Date(Date.now() - 104000).toISOString(),
      entryType: 'toolResult',
      tool: null,
      toolInput: null,
      output: 'File updated successfully.',
      text: null,
      thinking: null,
      thinkingDuration: null,
      exitCode: 0,
      linesChanged: { added: 1, removed: 1 },
    },
    {
      sessionId: '1',
      timestamp: new Date(Date.now() - 100000).toISOString(),
      entryType: 'toolCall',
      tool: 'Bash',
      toolInput: { command: 'npm test -- --testPathPattern=auth' },
      text: null,
      thinking: null,
      thinkingDuration: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    },
    {
      sessionId: '1',
      timestamp: new Date(Date.now() - 95000).toISOString(),
      entryType: 'toolResult',
      tool: null,
      toolInput: null,
      output:
        'PASS src/auth/auth.test.ts\n  ✓ refreshes token with correct expiry (23ms)\n  ✓ rejects expired tokens (12ms)\n\nTest Suites: 1 passed, 1 total',
      text: null,
      thinking: null,
      thinkingDuration: null,
      exitCode: 0,
      linesChanged: null,
    },
    {
      sessionId: '1',
      timestamp: new Date(Date.now() - 90000).toISOString(),
      entryType: 'assistant',
      text: "Fixed. The bug was `expiresIn: '1s'` instead of `'1h'`. All tests pass now.",
      thinking: null,
      thinkingDuration: null,
      tool: null,
      toolInput: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    },
  ],
  2: [
    {
      sessionId: '2',
      timestamp: new Date(Date.now() - 30000).toISOString(),
      entryType: 'user',
      text: 'Rebuild the dashboard frontend from scratch using SvelteKit',
      thinking: null,
      thinkingDuration: null,
      tool: null,
      toolInput: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    },
    {
      sessionId: '2',
      timestamp: new Date(Date.now() - 28000).toISOString(),
      entryType: 'thinking',
      thinking:
        "I need to plan the dashboard rebuild. First I'll understand the current structure, then scaffold the new SvelteKit project.",
      thinkingDuration: 2.1,
      text: null,
      tool: null,
      toolInput: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    },
    {
      sessionId: '2',
      timestamp: new Date(Date.now() - 25000).toISOString(),
      entryType: 'toolCall',
      tool: 'Bash',
      toolInput: { command: 'find src -name "*.svelte" | head -20' },
      text: null,
      thinking: null,
      thinkingDuration: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    },
    {
      sessionId: '2',
      timestamp: new Date(Date.now() - 24000).toISOString(),
      entryType: 'toolResult',
      tool: null,
      toolInput: null,
      output:
        'src/routes/+page.svelte\nsrc/routes/+layout.svelte\nsrc/components/Header.svelte\nsrc/components/Sidebar.svelte\nsrc/components/Chart.svelte',
      text: null,
      thinking: null,
      thinkingDuration: null,
      exitCode: 0,
      linesChanged: null,
    },
  ],
  3: [],
};

type Listener = (payload: unknown) => void;
const listeners: Map<string, Listener[]> = new Map();

export function mockEmit(event: string, payload: unknown) {
  const cbs = listeners.get(event) ?? [];
  cbs.forEach((cb) => cb(payload));
}

export function mockListen(event: string, cb: Listener) {
  if (!listeners.has(event)) listeners.set(event, []);
  listeners.get(event)!.push(cb);
  return () => {
    const arr = listeners.get(event)!;
    const i = arr.indexOf(cb);
    if (i !== -1) arr.splice(i, 1);
  };
}

let nextId = 4;
let sessions = [...MOCK_SESSIONS];
const journals: Record<number, JournalEntry[]> = { ...MOCK_JOURNAL };

export async function mockInvoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  await delay(80); // realistic latency

  switch (cmd) {
    case 'list_sessions':
      return sessions;

    case 'get_session_journal': {
      const id = args?.sessionId as number;
      return journals[id] ?? [];
    }

    case 'create_session': {
      const id = nextId++;
      const newSession: Session = {
        id,
        projectId: id,
        name: (args?.sessionName as string) ?? null,
        status: 'initializing',
        permissionMode: (args?.permissionMode as string) ?? 'ignore',
        model: (args?.modelId as string) ?? null,
        pid: null,
        cwd: args?.projectPath as string,
        projectName:
          String(args?.projectPath ?? '')
            .split(/[\\/]/)
            .pop() ?? 'project',
        gitBranch: null,
        worktreePath: null,
        branchName: null,
        tokens: null,
        contextPercent: null,
        pendingApproval: null,
        miniLog: null,
        costUsd: null,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      sessions.push(newSession);
      journals[id] = [];

      setTimeout(() => {
        sessions = sessions.map((s) =>
          s.id === id ? { ...s, status: 'running', pid: 10000 + id } : s
        );
        mockEmit('session:running', { sessionId: id, pid: 10000 + id });
        simulateClaudeResponse(id, (args?.prompt as string) ?? 'hello');
      }, 600);

      return newSession;
    }

    case 'send_session_message': {
      const id = args?.sessionId as number;
      const msg = args?.message as string;
      setTimeout(() => simulateClaudeResponse(id, msg), 400);
      return null;
    }

    case 'stop_session': {
      const id = args?.sessionId as number;
      sessions = sessions.map((s) => (s.id === id ? { ...s, status: 'stopped', pid: null } : s));
      setTimeout(() => mockEmit('session:stopped', { sessionId: id }), 100);
      return null;
    }

    case 'rename_session': {
      const id = args?.sessionId as number;
      const name = args?.name as string;
      sessions = sessions.map((s) => (s.id === id ? { ...s, name } : s));
      return null;
    }

    case 'delete_session': {
      const id = args?.sessionId as number;
      sessions = sessions.filter((s) => s.id !== id);
      return null;
    }

    case 'check_claude':
      return { found: true, path: '/mock/claude', searchedPath: '' };

    case 'get_slash_commands': {
      const cmds: SlashCommand[] = [
        { cmd: '/help', desc: 'Show help', category: 'built-in' },
        { cmd: '/compact', desc: 'Compact context', category: 'built-in' },
        { cmd: '/clear', desc: 'Clear conversation', category: 'built-in' },
        { cmd: '/cost', desc: 'Show token cost', category: 'built-in' },
        { cmd: '/model', desc: 'Switch model', category: 'built-in' },
      ];
      return cmds;
    }

    case 'list_project_files':
      return ['src/index.ts', 'src/auth/auth.ts', 'src/api/routes.ts', 'package.json', 'README.md'];

    case 'get_tasks':
      return [];

    default:
      console.warn('[mock] Unhandled invoke:', cmd, args);
      return null;
  }
}

function simulateClaudeResponse(sessionId: number, userMsg: string) {
  if (!journals[sessionId]) journals[sessionId] = [];

  const userEntry: JournalEntry = {
    sessionId: String(sessionId),
    timestamp: new Date().toISOString(),
    entryType: 'user',
    text: userMsg,
    thinking: null,
    thinkingDuration: null,
    tool: null,
    toolInput: null,
    output: null,
    exitCode: null,
    linesChanged: null,
  };
  journals[sessionId].push(userEntry);
  mockEmit('session:output', { sessionId, entry: userEntry });
  mockEmit('session:state', makeStateEvent(sessionId, 'working'));

  // Thinking
  setTimeout(() => {
    const thinkEntry: JournalEntry = {
      sessionId: String(sessionId),
      timestamp: new Date().toISOString(),
      entryType: 'thinking',
      thinking: `Processing request: "${userMsg}"\n\nI should analyze this carefully and provide a helpful response.`,
      thinkingDuration: 1.8,
      text: null,
      tool: null,
      toolInput: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    };
    journals[sessionId].push(thinkEntry);
    mockEmit('session:output', { sessionId, entry: thinkEntry });
  }, 600);

  // Tool call
  setTimeout(() => {
    const toolEntry: JournalEntry = {
      sessionId: String(sessionId),
      timestamp: new Date().toISOString(),
      entryType: 'toolCall',
      tool: 'Bash',
      toolInput: { command: `echo "processing: ${userMsg.slice(0, 30)}"` },
      text: null,
      thinking: null,
      thinkingDuration: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    };
    journals[sessionId].push(toolEntry);
    mockEmit('session:output', { sessionId, entry: toolEntry });
  }, 1400);

  // Tool result
  setTimeout(() => {
    const resultEntry: JournalEntry = {
      sessionId: String(sessionId),
      timestamp: new Date().toISOString(),
      entryType: 'toolResult',
      tool: null,
      toolInput: null,
      output: `processing: ${userMsg.slice(0, 30)}`,
      text: null,
      thinking: null,
      thinkingDuration: null,
      exitCode: 0,
      linesChanged: null,
    };
    journals[sessionId].push(resultEntry);
    mockEmit('session:output', { sessionId, entry: resultEntry });
  }, 2000);

  // Assistant response
  setTimeout(() => {
    const aiEntry: JournalEntry = {
      sessionId: String(sessionId),
      timestamp: new Date().toISOString(),
      entryType: 'assistant',
      text: `I've processed your request: "${userMsg}"\n\nThis is a mock response for browser testing. In the real app this would be Claude's actual response.`,
      thinking: null,
      thinkingDuration: null,
      tool: null,
      toolInput: null,
      output: null,
      exitCode: null,
      linesChanged: null,
    };
    journals[sessionId].push(aiEntry);
    mockEmit('session:output', { sessionId, entry: aiEntry });
    mockEmit(
      'session:state',
      makeStateEvent(sessionId, 'idle', {
        input: 1200,
        output: 340,
        cacheRead: 0,
        cacheWrite: 0,
      })
    );
  }, 2800);
}

function makeStateEvent(sessionId: number, status: string, extraTokens?: Partial<TokenUsage>) {
  const session = sessions.find((s) => s.id === sessionId);
  const base = session?.tokens ?? { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 };
  const tokens: TokenUsage = {
    input: base.input + (extraTokens?.input ?? 0),
    output: base.output + (extraTokens?.output ?? 0),
    cacheRead: base.cacheRead + (extraTokens?.cacheRead ?? 0),
    cacheWrite: base.cacheWrite + (extraTokens?.cacheWrite ?? 0),
  };
  return {
    sessionId,
    status,
    tokens,
    contextPercent: (tokens.input + tokens.output) / 2000,
    pendingApproval: null,
    miniLog: [],
  };
}

function delay(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}
