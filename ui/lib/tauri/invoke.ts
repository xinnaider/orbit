import { invoke as _invoke } from '@tauri-apps/api/core';
import { listen as _listen } from '@tauri-apps/api/event';
import { mockInvoke, mockListen } from '../mock/tauri-mock';

const IS_MOCK =
  import.meta.env.VITE_MOCK === 'true' ||
  !(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (IS_MOCK) return mockInvoke(cmd, args) as Promise<T>;
  return _invoke<T>(cmd, args);
}

export function listen<T>(event: string, cb: (e: { payload: T }) => void): Promise<() => void> {
  if (IS_MOCK) {
    const unlisten = mockListen(event, (payload) => cb({ payload: payload as T }));
    return Promise.resolve(unlisten);
  }
  return _listen<T>(event, cb);
}

export { IS_MOCK };
