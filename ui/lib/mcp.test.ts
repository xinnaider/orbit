import { describe, expect, it } from 'vitest';
import { isMcpSubagent, mcpChildSessionId, mcpStatusLabel, mcpStatusLevel } from './mcp';
import type { SubagentInfo } from './types';

describe('mcp helpers', () => {
  const mcpAgent: SubagentInfo = {
    id: '42',
    agentType: 'mcp:claude-code',
    description: 'review PR',
    status: 'running',
  };

  const nativeAgent: SubagentInfo = {
    id: 'abc-meta',
    agentType: 'Explore',
    description: 'search codebase',
    status: 'done',
  };

  it('detects MCP subagents by agentType prefix', () => {
    expect(isMcpSubagent(mcpAgent)).toBe(true);
    expect(isMcpSubagent(nativeAgent)).toBe(false);
  });

  it('parses MCP child session id from numeric id', () => {
    expect(mcpChildSessionId(mcpAgent)).toBe(42);
    expect(mcpChildSessionId(nativeAgent)).toBeNull();
  });

  it('labels MCP status by readiness', () => {
    const ready = {
      binaryAvailable: true,
      binaryPath: '/orbit',
      ipcListening: true,
      orchestrationReady: true,
      unifiedBinary: true,
      stdioArg: '--mcp-stdio',
    };
    expect(mcpStatusLevel(ready)).toBe('ready');
    expect(mcpStatusLabel(ready)).toBe('MCP ready');
  });
});
