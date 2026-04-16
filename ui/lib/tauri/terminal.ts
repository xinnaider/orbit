import { invoke, listen } from './invoke';

export interface PtyOutputPayload {
  sessionId: number;
  data: string;
  eof: boolean;
}

export async function ptyCreate(
  sessionId: number,
  command: string,
  args: string[],
  cwd: string,
  env: [string, string][],
  rows: number,
  cols: number
): Promise<number> {
  return invoke<number>('pty_create', {
    sessionId,
    command,
    args,
    cwd,
    env,
    rows,
    cols,
  });
}

export async function ptyWrite(sessionId: number, data: string): Promise<void> {
  return invoke('pty_write', { sessionId, data });
}

export async function ptyResize(sessionId: number, rows: number, cols: number): Promise<void> {
  return invoke('pty_resize', { sessionId, rows, cols });
}

export async function ptyKill(sessionId: number): Promise<void> {
  return invoke('pty_kill', { sessionId });
}

export function onPtyOutput(cb: (payload: PtyOutputPayload) => void): Promise<() => void> {
  return listen<PtyOutputPayload>('pty:output', (event) => cb(event.payload));
}
