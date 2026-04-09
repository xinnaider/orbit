import { writable } from 'svelte/store';

export type ToastType = 'error' | 'warning' | 'info' | 'success' | 'update';

export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface Toast {
  id: string;
  type: ToastType;
  message: string;
  autoDismiss: boolean;
  action?: ToastAction;
}

export const toasts = writable<Toast[]>([]);

export function addToast(toast: Omit<Toast, 'id'>): string {
  const id = Math.random().toString(36).slice(2);
  toasts.update((list) => [...list, { ...toast, id }]);

  if (toast.autoDismiss) {
    setTimeout(() => removeToast(id), 5000);
  }

  return id;
}

export function removeToast(id: string): void {
  toasts.update((list) => list.filter((t) => t.id !== id));
}
