import { describe, it, expect } from 'vitest';

// These tests verify that the mock wrapper functions (used when VITE_MOCK=true)
// return expected data shapes and handle edge cases without throwing.
// Run with: npx vitest run --config vitest.config.js ui/lib/tauri/tauri.test.ts

import { HAS_TAURI, IS_MOCK } from './invoke';
import { getAppVersion } from './system';
import { getProviders } from './providers';
import {
  createSession,
  stopSession,
  deleteSession,
  sendSessionMessage,
  listSessions,
  getSessionJournal,
} from './sessions';
import {
  getSlashCommands,
  listProjectFiles,
  getSubagents,
  createProject,
  listProjects,
} from './projects';
import { readFileContent } from './files';

// --- Basic mock mode detection ---

describe('tauri mock wrapper', () => {
  describe('HAS_TAURI is false in mock mode', () => {
    it('should be false when no Tauri runtime', () => {
      expect(HAS_TAURI).toBe(false);
    });

    it('should detect mock mode via IS_MOCK', () => {
      // VITE_MOCK is set in beforeEach, so IS_MOCK should be true
      expect(IS_MOCK).toBe(true);
    });
  });

  // --- system.ts ---

  describe('getAppVersion', () => {
    it('should return a version string', async () => {
      const version = await getAppVersion();
      expect(typeof version).toBe('string');
      expect(version.length).toBeGreaterThan(0);
    });

    it('should return "0.0.0" in mock mode', async () => {
      const version = await getAppVersion();
      expect(version).toBe('0.0.0');
    });
  });

  // --- providers.ts ---

  describe('getProviders', () => {
    it('should return an array', async () => {
      const providers = await getProviders();
      expect(Array.isArray(providers)).toBe(true);
    });

    it('should return providers with expected shape', async () => {
      const providers = await getProviders();
      expect(providers.length).toBeGreaterThan(0);

      const cc = providers.find((p) => p.id === 'claude-code');
      expect(cc).toBeDefined();
      expect(cc!.name).toBe('Claude Code');
      expect(cc!.cliAvailable).toBe(true);
      expect(cc!.supportsEffort).toBe(true);
      expect(cc!.supportsSsh).toBe(true);
      expect(cc!.supportsSubagents).toBe(true);
      expect(cc!.supportsTasks).toBe(true);
      expect(Array.isArray(cc!.models)).toBe(true);
      expect(cc!.models.length).toBeGreaterThan(0);
    });

    it('should include codex and opencode providers', async () => {
      const providers = await getProviders();
      const ids = providers.map((p) => p.id);
      expect(ids).toContain('codex');
      expect(ids).toContain('opencode');
    });

    it('each provider should have id, name, and capabilities', async () => {
      const providers = await getProviders();
      for (const p of providers) {
        expect(typeof p.id).toBe('string');
        expect(p.id.length).toBeGreaterThan(0);
        expect(typeof p.name).toBe('string');
        expect(typeof p.cliAvailable).toBe('boolean');
        expect(typeof p.supportsEffort).toBe('boolean');
        expect(typeof p.supportsSsh).toBe('boolean');
      }
    });
  });

  // --- sessions.ts ---

  describe('createSession', () => {
    it('should return a session object with expected shape', async () => {
      const session = await createSession({
        projectPath: '/test/project',
        prompt: 'test prompt',
      });

      expect(session).toBeDefined();
      expect(typeof session.id).toBe('number');
      expect(session.id).toBeGreaterThan(0);
      expect(session.status).toBe('initializing');
      expect(session.cwd).toBe('/test/project');
      expect(session.projectName).toBe('project');
      expect(session.provider).toBe('claude-code');
    });

    it('should accept optional sessionName and model', async () => {
      const session = await createSession({
        projectPath: '/another/path',
        prompt: 'hello',
        sessionName: 'my-session',
        model: 'claude-sonnet-4-6',
        provider: 'codex',
      });

      expect(session.name).toBe('my-session');
      expect(session.model).toBe('claude-sonnet-4-6');
      expect(session.provider).toBe('codex');
    });

    it('should have null tokens initially', async () => {
      const session = await createSession({
        projectPath: '/test',
        prompt: 'test',
      });
      expect(session.tokens).toBeNull();
      expect(session.contextPercent).toBeNull();
      expect(session.miniLog).toBeNull();
      expect(session.subagents).toEqual([]);
    });
  });

  describe('stopSession', () => {
    it('should return void (undefined/ null)', async () => {
      const result = await stopSession(1);
      expect(result).toBeUndefined();
    });

    it('should not throw for valid session id', async () => {
      await expect(stopSession(1)).resolves.toBeUndefined();
    });
  });

  describe('deleteSession', () => {
    it('should return void', async () => {
      const result = await deleteSession(1);
      expect(result).toBeUndefined();
    });

    it('should not throw for any session id', async () => {
      await expect(deleteSession(999)).resolves.toBeUndefined();
    });
  });

  describe('sendSessionMessage', () => {
    it('should return void', async () => {
      const result = await sendSessionMessage(1, 'hello');
      expect(result).toBeUndefined();
    });

    it('should handle empty message', async () => {
      await expect(sendSessionMessage(1, '')).resolves.toBeUndefined();
    });
  });

  describe('listSessions', () => {
    it('should return an array of sessions', async () => {
      const result = await listSessions();
      expect(Array.isArray(result)).toBe(true);
    });

    it('each session should have required fields', async () => {
      const result = await listSessions();
      for (const s of result) {
        expect(typeof s.id).toBe('number');
        expect(typeof s.status).toBe('string');
        expect(typeof s.provider).toBe('string');
        expect(s.createdAt).toBeDefined();
        expect(s.updatedAt).toBeDefined();
      }
    });
  });

  describe('getSessionJournal', () => {
    it('should return an array of journal entries', async () => {
      const entries = await getSessionJournal(1);
      expect(Array.isArray(entries)).toBe(true);
    });

    it('each entry should have required fields', async () => {
      const entries = await getSessionJournal(1);
      for (const entry of entries) {
        expect(typeof entry.sessionId).toBe('string');
        expect(typeof entry.entryType).toBe('string');
        expect(typeof entry.timestamp).toBe('string');
      }
    });

    it('should return empty array for non-existent session', async () => {
      const entries = await getSessionJournal(9999);
      expect(entries).toEqual([]);
    });
  });

  // --- projects.ts (shared functions) ---

  describe('getSlashCommands', () => {
    it('should return an array of slash commands', async () => {
      const cmds = await getSlashCommands();
      expect(Array.isArray(cmds)).toBe(true);
      expect(cmds.length).toBeGreaterThan(0);
    });

    it('each command should have cmd, desc, category', async () => {
      const cmds = await getSlashCommands();
      for (const c of cmds) {
        expect(typeof c.cmd).toBe('string');
        expect(c.cmd.startsWith('/')).toBe(true);
        expect(typeof c.desc).toBe('string');
        expect(typeof c.category).toBe('string');
      }
    });

    it('should include /help command', async () => {
      const cmds = await getSlashCommands();
      const cmdsList = cmds.map((c) => c.cmd);
      expect(cmdsList).toContain('/help');
    });

    it('should return claude-code commands by default', async () => {
      const cmds = await getSlashCommands();
      const cmdsList = cmds.map((c) => c.cmd);
      expect(cmdsList).toContain('/compact');
      expect(cmdsList).toContain('/model');
    });

    it('should return codex-specific commands', async () => {
      const cmds = await getSlashCommands('codex');
      const cmdsList = cmds.map((c) => c.cmd);
      expect(cmdsList).toContain('/help');
      expect(cmdsList).toContain('/model');
      expect(cmdsList).toContain('/fast');
    });
  });

  describe('listProjectFiles', () => {
    it('should return an array of file paths', async () => {
      const files = await listProjectFiles('/some/path');
      expect(Array.isArray(files)).toBe(true);
      expect(files.length).toBeGreaterThan(0);
    });

    it('each file should be a string path', async () => {
      const files = await listProjectFiles('/some/path');
      for (const f of files) {
        expect(typeof f).toBe('string');
        expect(f.length).toBeGreaterThan(0);
      }
    });

    it('should include source files', async () => {
      const files = await listProjectFiles('/some/path');
      expect(files).toContain('src/index.ts');
      expect(files).toContain('package.json');
    });
  });

  describe('getSubagents', () => {
    it('should return an array', async () => {
      const agents = await getSubagents(1);
      expect(Array.isArray(agents)).toBe(true);
    });

    it('each subagent should have id, agentType, description, status', async () => {
      const agents = await getSubagents(1);
      for (const a of agents) {
        expect(typeof a.id).toBe('string');
        expect(typeof a.agentType).toBe('string');
        expect(typeof a.description).toBe('string');
        expect(typeof a.status).toBe('string');
      }
    });

    it('should return empty array for session with no subagents', async () => {
      const agents = await getSubagents(2);
      expect(agents).toEqual([]);
    });
  });

  // --- files.ts ---

  describe('readFileContent', () => {
    it('should return a string', async () => {
      const content = await readFileContent('test.ts');
      expect(typeof content).toBe('string');
    });

    it('should include the file path in mock content', async () => {
      const content = await readFileContent('src/hello.ts');
      expect(content).toContain('src/hello.ts');
    });
  });

  // --- projects.ts (project CRUD) ---

  describe('createProject', () => {
    it('should return a project object', async () => {
      const project = (await createProject('my-project', '/path/to/project')) as {
        id: number;
        name: string;
        path: string;
      };
      expect(project).toBeDefined();
      expect(typeof project.id).toBe('number');
      expect(project.name).toBe('my-project');
      expect(project.path).toBe('/path/to/project');
    });
  });

  describe('listProjects', () => {
    it('should return an array of projects', async () => {
      const projects = await listProjects();
      expect(Array.isArray(projects)).toBe(true);
    });

    it('each project should have id, name, path', async () => {
      const projects = (await listProjects()) as Array<{
        id: number;
        name: string;
        path: string;
      }>;
      for (const p of projects) {
        expect(typeof p.id).toBe('number');
        expect(typeof p.name).toBe('string');
        expect(typeof p.path).toBe('string');
      }
    });
  });
});
