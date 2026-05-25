import { invoke } from './invoke';

export async function getDesktopNotificationsEnabled(): Promise<boolean> {
  return invoke<boolean>('get_desktop_notifications_enabled');
}

export async function setDesktopNotificationsEnabled(enabled: boolean): Promise<void> {
  return invoke('set_desktop_notifications_enabled', { enabled });
}
