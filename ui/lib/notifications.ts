import { get } from 'svelte/store';
import { notificationsEnabled } from './stores/preferences';
import { mutedSessions } from './stores/ui';
import { sessions } from './stores/sessions';
import { HAS_TAURI } from './tauri/invoke';

let permissionChecked = false;
let permissionGranted = false;

export async function initDesktopNotifications(): Promise<void> {
  if (!HAS_TAURI) return;
  try {
    const { isPermissionGranted, requestPermission } =
      await import('@tauri-apps/plugin-notification');
    permissionGranted = await isPermissionGranted();
    if (!permissionGranted) {
      const state = await requestPermission();
      permissionGranted = state === 'granted';
    }
    permissionChecked = true;
  } catch (e) {
    console.warn('Desktop notifications unavailable', e);
    permissionChecked = true;
    permissionGranted = false;
  }
}

export function syncNotificationsEnabled(enabled: boolean): void {
  notificationsEnabled.set(enabled);
}

export async function notifyDesktop(opts: {
  sessionId?: number;
  title: string;
  body: string;
}): Promise<void> {
  if (!HAS_TAURI || !get(notificationsEnabled)) return;
  if (opts.sessionId != null) {
    const muted = get(mutedSessions);
    if (mutedSessions.isMuted(muted, String(opts.sessionId))) return;
  }
  if (!permissionChecked) await initDesktopNotifications();
  if (!permissionGranted) return;

  try {
    const { sendNotification } = await import('@tauri-apps/plugin-notification');
    await sendNotification({ title: opts.title, body: opts.body });
  } catch (e) {
    console.warn('Failed to send desktop notification', e);
  }
}

export function sessionDisplayName(sessionId: number): string {
  const list = get(sessions);
  const s = list.find((x) => x.id === sessionId);
  if (!s) return `Session #${sessionId}`;
  return s.name ?? s.projectName ?? s.cwd?.split(/[/\\]/).pop() ?? `Session #${sessionId}`;
}

const notifiedKeys = new Set<string>();

/** Notify once per session+reason until cleared. */
export async function notifyAttentionOnce(
  sessionId: number,
  reason: string,
  title: string,
  body: string
): Promise<void> {
  const key = `${sessionId}:${reason}`;
  if (notifiedKeys.has(key)) return;
  notifiedKeys.add(key);
  await notifyDesktop({ sessionId, title, body });
}

export function clearAttentionNotifyKey(sessionId: number, reason?: string): void {
  if (reason) {
    notifiedKeys.delete(`${sessionId}:${reason}`);
    return;
  }
  for (const key of [...notifiedKeys]) {
    if (key.startsWith(`${sessionId}:`)) notifiedKeys.delete(key);
  }
}
