import { invoke } from './invoke';

export async function clearAttention(sessionId: number): Promise<void> {
  return invoke('clear_attention', { sessionId });
}

export async function respondPermission(sessionId: number, allow: boolean): Promise<void> {
  return invoke('respond_permission', { sessionId, allow });
}
