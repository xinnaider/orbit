import { describe, expect, it } from 'vitest';
import { mockInvoke } from '../mock/tauri-mock';

const GIT_COMMANDS = [
  'git_stage_all',
  'git_reset_staged',
  'git_commit',
  'git_quick_commit',
  'git_reset_working_tree',
  'git_diff_formatted',
  'git_validate_config',
  'git_stage_file',
  'git_unstage_file',
  'git_overview',
  'git_diff_file',
  'get_diff',
  'get_file_versions',
  'search_project_files',
] as const;

describe('invoke contract (mock ↔ Rust)', () => {
  it.each(GIT_COMMANDS)('mock handles %s', async (cmd) => {
    const result = await mockInvoke(cmd, {
      cwd: '/tmp/orbit-mock',
      filePath: 'ui/lib/tauri/git.ts',
      path: 'ui/lib/tauri/git.ts',
      group: 'unstaged',
      query: 'git',
      sessionId: '1',
      fileHash: 'abc',
      fromVersion: 0,
      toVersion: 1,
      message: 'test',
    });
    expect(result !== undefined || result === null).toBe(true);
  });
});
