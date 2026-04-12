// Unified status helpers — Session.status can be DB values or runtime event values

export type SessionStatus =
  | 'initializing'
  | 'running'
  | 'waiting'
  | 'completed'
  | 'stopped'
  | 'error'
  | 'working'
  | 'input'
  | 'idle'
  | 'new';

export function statusColor(status: string): string {
  switch (status) {
    case 'working':
    case 'running':
      return 'var(--s-working)';
    case 'input':
    case 'waiting':
      return 'var(--s-input)';
    case 'initializing':
      return 'var(--s-init)';
    case 'error':
      return 'var(--s-error)';
    case 'completed':
    case 'stopped':
    case 'idle':
      return 'var(--s-done)';
    default:
      return 'var(--s-idle)';
  }
}

export function statusLabel(status: string): string {
  switch (status) {
    case 'working':
      return 'working';
    case 'running':
      return 'running';
    case 'input':
    case 'waiting':
      return 'waiting';
    case 'initializing':
      return 'init';
    case 'completed':
    case 'idle':
      return 'idle';
    case 'stopped':
      return 'stopped';
    case 'error':
      return 'error';
    default:
      return status;
  }
}

export function isActive(status: string): boolean {
  return ['working', 'running', 'input', 'waiting', 'initializing'].includes(status);
}

export function isPulsing(status: string): boolean {
  return ['working', 'running'].includes(status);
}

const MODEL_NAMES: Record<string, string> = {
  'claude-opus-4-6': 'Opus 4.6',
  'claude-sonnet-4-6': 'Sonnet 4.6',
  'claude-haiku-4-5-20251001': 'Haiku 4.5',
};

export function modelDisplayName(modelId: string | null): string {
  if (!modelId) return '—';
  return MODEL_NAMES[modelId] ?? modelId;
}
