import type { McpStatus } from '../mcp';
import { invoke } from './invoke';

export async function setupOrchestration(projectPath: string): Promise<string> {
  return invoke<string>('setup_orchestration', { projectPath });
}

export async function checkOrchestration(): Promise<McpStatus> {
  return invoke<McpStatus>('check_orchestration');
}

export async function getMcpStatus(): Promise<McpStatus> {
  return invoke<McpStatus>('get_mcp_status');
}
