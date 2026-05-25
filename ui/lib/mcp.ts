import type { SubagentInfo } from './types';

/** MCP child sessions use numeric ids and agentType `mcp:<provider>`. */
export function isMcpSubagent(agent: SubagentInfo): boolean {
  return agent.agentType.startsWith('mcp:');
}

/** Orbit session id for an MCP-spawned child (SubagentInfo.id is the child session id). */
export function mcpChildSessionId(agent: SubagentInfo): number | null {
  const id = Number.parseInt(agent.id, 10);
  return Number.isFinite(id) && id > 0 ? id : null;
}

export interface McpStatus {
  binaryAvailable: boolean;
  binaryPath: string | null;
  ipcListening: boolean;
  orchestrationReady: boolean;
  unifiedBinary: boolean;
  stdioArg: string;
}

export function mcpStatusLabel(status: McpStatus): string {
  if (status.orchestrationReady) return 'MCP ready';
  if (status.binaryAvailable && !status.ipcListening) return 'MCP binary OK · start Orbit';
  if (!status.binaryAvailable) return 'MCP unavailable';
  return 'MCP partial';
}

export function mcpStatusLevel(status: McpStatus): 'ready' | 'warn' | 'error' {
  if (status.orchestrationReady) return 'ready';
  if (status.binaryAvailable) return 'warn';
  return 'error';
}
