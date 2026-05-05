import type { Session, TokenUsage } from '../stores/sessions';
import type { JournalEntry, SlashCommand, SubagentInfo, TaskItem } from '../types';

const nowIso = () => new Date().toISOString();

const MOCK_SUBAGENTS: Record<number, SubagentInfo[]> = {
  1: [
    {
      id: 'task-1',
      agentType: 'Task',
      description: 'Write auth regression tests',
      status: 'running',
    },
    {
      id: 'task-2',
      agentType: 'Task',
      description: 'Verify token refresh behavior',
      status: 'done',
    },
  ],
  2: [],
  3: [],
};

const MOCK_TASKS: Record<string, TaskItem[]> = {
  '1': [
    {
      id: 'task-1',
      subject: 'Write tests',
      description: 'Cover refresh token edge cases',
      activeForm: 'Writing tests',
      status: 'in_progress',
      blocks: [],
      blockedBy: [],
    },
    {
      id: 'task-2',
      subject: 'Verify fix',
      description: 'Confirm expiry is back to one hour',
      activeForm: null,
      status: 'pending',
      blocks: [],
      blockedBy: ['task-1'],
    },
  ],
};

const MOCK_SUBAGENT_JOURNAL: Record<string, JournalEntry[]> = {
  '1:task-1': [
    {
      sessionId: 'task-1',
      timestamp: new Date(Date.now() - 45000).toISOString(),
      entryType: 'assistant',
      text: 'I am adding regression coverage for token refresh.',
      thinking: null,
      thinkingDuration: null,
      tool: null,
      toolInput: null,
      output: null,
      exitCode: null,
      linesChanged: null,
      seq: 0,
      epoch: '',
    },
    {
      sessionId: 'task-1',
      timestamp: new Date(Date.now() - 42000).toISOString(),
      entryType: 'toolCall',
      text: null,
      thinking: null,
      thinkingDuration: null,
      tool: 'Read',
      toolInput: { file_path: 'src/auth/auth.test.ts' },
      output: null,
      exitCode: null,
      linesChanged: null,
      seq: 0,
      epoch: '',
    },
  ],
  '1:task-2': [
    {
      sessionId: 'task-2',
      timestamp: new Date(Date.now() - 70000).toISOString(),
      entryType: 'assistant',
      text: 'Verified the fix manually and through the existing tests.',
      thinking: null,
      thinkingDuration: null,
      tool: null,
      toolInput: null,
      output: null,
      exitCode: null,
      linesChanged: null,
      seq: 0,
      epoch: '',
    },
  ],
};

let mockProjects = [
  { id: 1, name: 'api-server', path: 'C:\\Users\\dev\\api-server' },
  { id: 2, name: 'dashboard', path: 'C:\\Users\\dev\\dashboard' },
];

let mockHttpSettings = {
  enabled: true,
  host: '0.0.0.0',
  port: 9999,
};

let mockApiKeys = [
  {
    id: 'key-1',
    label: 'phone-1',
    key: 'orbit_mock_phone_token',
    createdAt: nowIso(),
  },
];

const savedProviderKeys = new Map<string, { envVar: string; apiKey: string }>();

const MOCK_SESSIONS: Session[] = [
  {
    id: 1,
    projectId: 1,
    name: 'marlin',
    status: 'running' as const,
    permissionMode: 'ignore',
    model: 'claude-sonnet-4-6',
    provider: 'claude-code',
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
    subagents: MOCK_SUBAGENTS[1],
    sshHost: null,
    sshUser: null,

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
    provider: 'claude-code',
    pid: 23456,
    cwd: 'C:\\Users\\dev\\dashboard',
    projectName: 'dashboard',
    gitBranch: 'feat/redesign',
    tokens: { input: 89200, output: 12300, cacheRead: 45000, cacheWrite: 3200 },
    contextPercent: 51.3,
    pendingApproval: null,
    miniLog: null,
    subagents: MOCK_SUBAGENTS[2],
    sshHost: null,
    sshUser: null,

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
    provider: 'claude-code',
    pid: null,
    cwd: 'C:\\Users\\dev\\utils',
    projectName: 'utils-lib',
    gitBranch: null,
    tokens: { input: 5200, output: 980, cacheRead: 2100, cacheWrite: 120 },
    contextPercent: 3.1,
    pendingApproval: null,
    miniLog: null,
    subagents: MOCK_SUBAGENTS[3],
    sshHost: null,
    sshUser: null,

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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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

    case 'get_session_journal_page': {
      const id = Number(args?.sessionId ?? 0);
      const offset = Number(args?.offset ?? 0);
      const limit = Number(args?.limit ?? 200);
      return (journals[id] ?? []).slice(offset, offset + limit);
    }

    case 'create_session': {
      const id = nextId++;
      const newSession: Session = {
        id,
        projectId: id,
        name: (args?.sessionName as string) ?? null,
        status: 'initializing',
        permissionMode: (args?.permissionMode as string) ?? 'ignore',
        model: (args?.model as string) ?? null,
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
        subagents: [],
        sshHost: null,
        sshUser: null,
        provider: (args?.provider as string) ?? 'claude-code',

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
      const provider = (args?.provider as string | null) ?? 'claude-code';
      const cmds: SlashCommand[] = [{ cmd: '/help', desc: 'Show help', category: 'built-in' }];
      if (provider === 'claude-code') {
        cmds.push(
          { cmd: '/compact', desc: 'Compact context', category: 'built-in' },
          { cmd: '/clear', desc: 'Clear conversation', category: 'built-in' },
          { cmd: '/cost', desc: 'Show token cost', category: 'built-in' },
          { cmd: '/model', desc: 'Switch model', category: 'built-in' },
          { cmd: '/fast', desc: 'Toggle fast output mode', category: 'built-in' }
        );
      } else if (provider === 'codex') {
        cmds.push(
          { cmd: '/model', desc: 'Choose model and reasoning effort', category: 'built-in' },
          { cmd: '/fast', desc: 'Toggle Fast mode for faster inference', category: 'built-in' }
        );
      } else {
        cmds.push(
          { cmd: '/compact', desc: 'Compact context', category: 'built-in' },
          { cmd: '/clear', desc: 'Clear conversation', category: 'built-in' },
          { cmd: '/cost', desc: 'Show token cost', category: 'built-in' },
          { cmd: '/model', desc: 'Switch model', category: 'built-in' }
        );
      }
      return cmds;
    }

    case 'create_project': {
      const project = {
        id: mockProjects.length + 1,
        name: String(args?.name ?? `project-${mockProjects.length + 1}`),
        path: String(args?.path ?? ''),
      };
      mockProjects = [...mockProjects, project];
      return project;
    }

    case 'list_projects':
      return mockProjects;

    case 'list_project_files':
      return ['src/index.ts', 'src/auth/auth.ts', 'src/api/routes.ts', 'package.json', 'README.md'];

    case 'read_file_content': {
      const filePath = args?.path as string;
      return `// Mock file content for: ${filePath}`;
    }

    case 'update_session_model': {
      const id = args?.sessionId as number;
      const model = args?.model as string;
      sessions = sessions.map((s) => (s.id === id ? { ...s, model } : s));
      return null;
    }

    case 'update_session_effort':
      return null;

    case 'set_session_api_key':
      return null;

    case 'save_provider_key':
      if (args?.providerId && args?.envVar && args?.apiKey) {
        savedProviderKeys.set(String(args.providerId), {
          envVar: String(args.envVar),
          apiKey: String(args.apiKey),
        });
      }
      return null;

    case 'load_provider_key':
      return args?.providerId ? (savedProviderKeys.get(String(args.providerId)) ?? null) : null;

    case 'delete_provider_key':
      if (args?.providerId) {
        savedProviderKeys.delete(String(args.providerId));
      }
      return null;

    case 'get_tasks':
      return MOCK_TASKS[String(args?.sessionId ?? '')] ?? [];

    case 'get_subagents':
      return Promise.resolve(MOCK_SUBAGENTS[Number(args?.sessionId ?? 0)] ?? []);

    case 'get_subagent_journal': {
      const sessionId = String(args?.sessionId ?? '');
      const subagentId = String(args?.subagentId ?? '');
      return MOCK_SUBAGENT_JOURNAL[`${sessionId}:${subagentId}`] ?? [];
    }

    case 'git_overview': {
      const cwd = (args?.cwd as string) ?? '';
      return {
        cwd,
        branch: 'feat/improve-feed-ui',
        upstream: 'origin/feat/improve-feed-ui',
        ahead: 3,
        behind: 0,
        files: [
          {
            id: 'staged:src/components/GitPanel.svelte',
            path: 'src/components/GitPanel.svelte',
            fileName: 'GitPanel.svelte',
            group: 'staged',
            status: 'modified',
            staged: true,
            untracked: false,
            oldPath: null,
            additions: 200,
            deletions: 0,
          },
          {
            id: 'unstaged:src/components/GitPanel.svelte',
            path: 'src/components/GitPanel.svelte',
            fileName: 'GitPanel.svelte',
            group: 'unstaged',
            status: 'modified',
            staged: false,
            untracked: false,
            oldPath: null,
            additions: 15,
            deletions: 3,
          },
          {
            id: 'unstaged:tauri/src/commands/git.rs',
            path: 'tauri/src/commands/git.rs',
            fileName: 'git.rs',
            group: 'unstaged',
            status: 'added',
            staged: false,
            untracked: false,
            oldPath: null,
            additions: 280,
            deletions: 0,
          },
          {
            id: 'untracked:ui/lib/git-tree.ts',
            path: 'ui/lib/git-tree.ts',
            fileName: 'git-tree.ts',
            group: 'untracked',
            status: 'untracked',
            staged: false,
            untracked: true,
            oldPath: null,
            additions: null,
            deletions: null,
          },
          {
            id: 'untracked:ui/lib/git-tags.ts',
            path: 'ui/lib/git-tags.ts',
            fileName: 'git-tags.ts',
            group: 'untracked',
            status: 'untracked',
            staged: false,
            untracked: true,
            oldPath: null,
            additions: null,
            deletions: null,
          },
          {
            id: 'staged:package.json',
            path: 'package.json',
            fileName: 'package.json',
            group: 'staged',
            status: 'modified',
            staged: true,
            untracked: false,
            oldPath: null,
            additions: 3,
            deletions: 0,
          },
        ],
        branches: [
          { name: 'feat/improve-feed-ui', fullName: 'refs/heads/feat/improve-feed-ui', kind: 'local', current: true, upstream: 'origin/feat/improve-feed-ui', ahead: 3, behind: 0 },
          { name: 'master', fullName: 'refs/heads/master', kind: 'local', current: false, upstream: 'origin/master', ahead: 0, behind: 1 },
          { name: 'origin/feat/improve-feed-ui', fullName: 'refs/remotes/origin/feat/improve-feed-ui', kind: 'remote', current: false, upstream: null, ahead: 0, behind: 0 },
          { name: 'origin/master', fullName: 'refs/remotes/origin/master', kind: 'remote', current: false, upstream: null, ahead: 0, behind: 0 },
        ],
      };
    }

    case 'git_diff_file': {
      const path = (args?.path as string) ?? '';
      const group = (args?.group as string) ?? 'unstaged';
      let original = '';
      let modified = '';

      if (path.includes('GitPanel.svelte') || path.includes('git-tree.ts')) {
        original = `<script>\n  import { onMount } from 'svelte';\n  export let cwd: string;\n</script>\n\n<div class="placeholder">\n  Git panel placeholder\n</div>\n\n<style>\n  .placeholder {\n    color: #6b7f75;\n  }\n</style>\n`;
        modified = `<script lang="ts">\n  import { onMount } from 'svelte';\n  import { GitBranch, RefreshCw, X } from 'lucide-svelte';\n  import { gitOverview } from '../lib/tauri/git';\n  import { buildGitTree } from '../lib/git-tree';\n\n  export let cwd: string;\n  export let onClose: (() => void) | null = null;\n\n  let overview: GitOverview | null = null;\n  let loading = false;\n\n  onMount(async () => {\n    loading = true;\n    overview = await gitOverview(cwd);\n    loading = false;\n  });\n</script>\n\n<section class="git-panel">\n  <header class="git-header">\n    <GitBranch size={13} />\n    <span>Git Overview</span>\n    <button type="button" on:click={onClose}><X size={13} /></button>\n  </header>\n  ...\n</section>\n`;
      } else if (path.includes('git-tags.ts')) {
        original = '';
        modified = `import type { GitFileChange } from './tauri/git';\n\nexport const FIXED_GIT_TAGS = ['ready', 'needs review', 'docs', 'risky', 'generated'] as const;\nexport type FixedGitTag = (typeof FIXED_GIT_TAGS)[number];\n\nexport function tagKey(file: { path: string; group: string }): string {\n  return \`\${file.group}:\${file.path}\`;\n}\n\nexport function loadGitTags(repoPath: string): Record<string, string[]> {\n  try {\n    const raw = localStorage.getItem(\`orbit:git-file-tags:\${repoPath}\`);\n    return raw ? JSON.parse(raw) : {};\n  } catch { return {}; }\n}\n`;
      } else {
        original = '{\n  "name": "orbit",\n  "version": "1.0.0",\n  "private": true\n}\n';
        modified = '{\n  "name": "orbit",\n  "version": "1.0.0",\n  "private": true,\n  "dependencies": {\n    "monaco-editor": "^0.55.0"\n  }\n}\n';
      }

      const langMap: Record<string, string> = {
        svelte: 'html',
        ts: 'typescript',
        rs: 'rust',
        json: 'json',
      };
      const ext = path.split('.').pop() ?? '';
      const language = langMap[ext] ?? 'plaintext';

      return { id: `${group}:${path}`, path, group, language, binary: false, original, modified };
    }

    case 'get_changelog':
      return '# Changelog\n\n## April 2026\n\n### 04/07 · New — In-app changelog\nYou can now view the history of Orbit updates directly inside the app.';

    case 'get_providers':
      return [
        {
          id: 'claude-code',
          name: 'Claude Code',
          cliAvailable: true,
          supportsEffort: true,
          supportsSsh: true,
          supportsSubagents: true,
          supportsTasks: true,
          hasSubProviders: false,
          models: [
            { id: 'auto', name: 'auto', context: null, output: null },
            { id: 'claude-opus-4-7', name: 'opus-4.7', context: 1000000, output: 128000 },
            { id: 'claude-sonnet-4-6', name: 'sonnet-4.6', context: 1000000, output: 64000 },
            { id: 'claude-opus-4-6', name: 'opus-4.6', context: 1000000, output: 128000 },
          ],
          subProviders: [],
          effortLevels: {
            'claude-opus-4-7': ['low', 'medium', 'high', 'xhigh', 'max', 'auto'],
            auto: ['low', 'medium', 'high', 'max'],
            'claude-opus-4-6': ['low', 'medium', 'high', 'max'],
            'claude-sonnet-4-6': ['low', 'medium', 'high', 'max'],
          },
          taskToolNames: ['TodoWrite'],
          taskFormat: 'claude_tool_use',
        },
        {
          id: 'codex',
          name: 'Codex',
          cliAvailable: true,
          supportsEffort: true,
          supportsSsh: true,
          supportsSubagents: true,
          supportsTasks: true,
          hasSubProviders: false,
          models: [
            { id: 'gpt-5.5', name: 'gpt-5.5', context: 1000000, output: 128000 },
            { id: 'gpt-5.4', name: 'gpt-5.4', context: null, output: null },
            { id: 'gpt-5.4-mini', name: 'gpt-5.4-mini', context: null, output: null },
            { id: 'gpt-5.3-codex', name: 'gpt-5.3-codex', context: null, output: null },
            { id: 'gpt-5.2', name: 'gpt-5.2', context: null, output: null },
          ],
          subProviders: [],
          effortLevels: {
            'gpt-5.5': ['none', 'minimal', 'low', 'medium', 'high', 'xhigh'],
            'gpt-5.4': ['none', 'minimal', 'low', 'medium', 'high', 'xhigh'],
            'gpt-5.4-mini': ['none', 'minimal', 'low', 'medium', 'high', 'xhigh'],
            'gpt-5.3-codex': ['none', 'minimal', 'low', 'medium', 'high', 'xhigh'],
            'gpt-5.2': ['none', 'minimal', 'low', 'medium', 'high', 'xhigh'],
          },
          taskToolNames: ['todo_list'],
          taskFormat: 'codex_item_list',
        },
        {
          id: 'opencode',
          name: 'OpenCode',
          cliAvailable: true,
          supportsEffort: false,
          supportsSsh: false,
          supportsSubagents: true,
          supportsTasks: true,
          hasSubProviders: true,
          models: [],
          subProviders: [
            {
              id: 'openrouter',
              name: 'OpenRouter',
              env: ['OPENROUTER_API_KEY'],
              configured: false,
              models: [
                {
                  id: 'anthropic/claude-sonnet-4',
                  name: 'Claude Sonnet 4',
                  context: 200000,
                  output: 64000,
                },
              ],
            },
            {
              id: 'anthropic',
              name: 'Anthropic',
              env: ['ANTHROPIC_API_KEY'],
              configured: true,
              models: [
                {
                  id: 'claude-sonnet-4-6',
                  name: 'Claude Sonnet 4.6',
                  context: 1000000,
                  output: 64000,
                },
              ],
            },
          ],
          effortLevels: {},
          taskToolNames: ['todowrite'],
          taskFormat: 'opencode_tool_use',
        },
      ];

    case 'check_env_var':
      return false;

    case 'generate_api_key': {
      const created = {
        id: `key-${mockApiKeys.length + 1}`,
        label: String(args?.label ?? `key-${mockApiKeys.length + 1}`),
        key: `orbit_mock_${Math.random().toString(36).slice(2, 14)}`,
        createdAt: nowIso(),
      };
      mockApiKeys = [...mockApiKeys, created];
      return { id: created.id, label: created.label, key: created.key };
    }

    case 'list_api_keys':
      return mockApiKeys.map(({ id, label, createdAt }) => ({ id, label, createdAt }));

    case 'revoke_api_key': {
      const before = mockApiKeys.length;
      mockApiKeys = mockApiKeys.filter((key) => key.id !== String(args?.id ?? ''));
      return mockApiKeys.length !== before;
    }

    case 'get_http_settings':
      return mockHttpSettings;

    case 'set_http_settings':
      mockHttpSettings = {
        enabled: Boolean(args?.enabled),
        host: String(args?.host ?? '127.0.0.1'),
        port: Number(args?.port ?? 9999),
      };
      return null;

    case 'get_lan_ip':
      return '192.168.0.42';

    case 'setup_orchestration':
      return `${String(args?.projectPath ?? 'C:\\Users\\dev\\project')}\\.mcp.json`;

    case 'check_orchestration':
      return { available: true, path: 'C:\\Users\\dev\\project\\.mcp.json' };

    case 'diagnose_provider': {
      const backend = (args?.backend as string) ?? 'claude-code';
      const sshHost = args?.sshHost as string | null;
      const projectPath = args?.projectPath as string | null;
      return {
        backend,
        cliName: backend === 'claude-code' ? 'claude' : backend === 'codex' ? 'codex' : 'opencode',
        found: true,
        path: sshHost ? `/home/ubuntu/.local/bin/${backend}` : '/mock/path/' + backend,
        version: '1.0.0-mock',
        installHint: 'npm install -g mock',
        ssh: sshHost ? { ok: true, latencyMs: 42, error: '' } : null,
        projectDirOk: projectPath ? true : null,
      };
    }

    case 'test_ssh':
      return { ok: true, latencyMs: 42, error: '' };

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
    seq: 0,
    epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
      seq: 0,
      epoch: '',
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
    gitBranch: null,
    subagents: session?.subagents ?? [],
    model: session?.model ?? null,
    contextWindow: 200_000,
    attention: null,
    rateLimit: [],
    costUsd: null,
  };
}

function delay(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}
