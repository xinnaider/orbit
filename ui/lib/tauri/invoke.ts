import { invoke as _invoke } from '@tauri-apps/api/core';
import { listen as _listen } from '@tauri-apps/api/event';
import { mockInvoke, mockListen } from '../mock/tauri-mock';
import { webInvoke, webListen } from './web-adapter';

const HAS_TAURI = !!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
const IS_MOCK = import.meta.env.VITE_MOCK === 'true';
const IS_WEB = !HAS_TAURI && !IS_MOCK;

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (HAS_TAURI) return _invoke<T>(cmd, args);
  if (IS_WEB) return webInvoke<T>(cmd, args);
  return mockInvoke(cmd, args) as Promise<T>;
}

export function listen<T>(event: string, cb: (e: { payload: T }) => void): Promise<() => void> {
  if (HAS_TAURI) return _listen<T>(event, cb);
  if (IS_WEB) return Promise.resolve(webListen(event, cb as (e: { payload: unknown }) => void));
  const unlisten = mockListen(event, (payload) => cb({ payload: payload as T }));
  return Promise.resolve(unlisten);
}

export { IS_MOCK, IS_WEB, HAS_TAURI };
