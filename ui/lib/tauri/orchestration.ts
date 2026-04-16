import { invoke } from './invoke';

export async function setupOrchestration(projectPath: string): Promise<string> {
  return invoke<string>('setup_orchestration', { projectPath });
}

export async function checkOrchestration(): Promise<{ available: boolean; path: string | null }> {
  return invoke('check_orchestration');
}
