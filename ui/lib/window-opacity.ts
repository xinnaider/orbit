import { HAS_TAURI } from './tauri/invoke';
import { setWindowOpacity } from './tauri/window';

export const WINDOW_OPACITY_STORAGE_KEY = 'windowOpacity';
export const DEFAULT_WINDOW_OPACITY = 100;

export function parseStoredWindowOpacity(raw: string | null): number {
  if (raw === null) return DEFAULT_WINDOW_OPACITY;
  const n = Number.parseInt(raw, 10);
  if (!Number.isFinite(n) || n < 0 || n > 100) return DEFAULT_WINDOW_OPACITY;
  return n;
}

/** Apply opacity to the document (readability) and native window when running in Tauri. */
export async function applyWindowOpacity(percent: number): Promise<void> {
  const value = Math.min(100, Math.max(0, Math.round(percent)));

  if (typeof document !== 'undefined') {
    const root = document.documentElement;
    if (value >= 100) {
      root.removeAttribute('data-window-opacity');
      root.style.removeProperty('--window-opacity');
    } else {
      root.setAttribute('data-window-opacity', String(value));
      root.style.setProperty('--window-opacity', String(value / 100));
    }
  }

  if (HAS_TAURI) {
    try {
      await setWindowOpacity(value);
    } catch (err) {
      console.warn('[orbit] set_window_opacity failed:', err);
    }
  }
}
