import { writable } from 'svelte/store';
import {
  applyWindowOpacity,
  parseStoredWindowOpacity,
  WINDOW_OPACITY_STORAGE_KEY,
} from '../window-opacity';

export const THEME_OPTIONS = ['dark', 'light', 'nord', 'dracula', 'catppuccin', 'steel'] as const;
export type Theme = (typeof THEME_OPTIONS)[number];

export const THEME_LABELS: Record<Theme, string> = {
  dark: 'Dark',
  light: 'Light (you are a monster?)',
  nord: 'Nord',
  dracula: 'Dracula',
  catppuccin: 'Catppuccin',
  steel: 'Steel Workshop',
};

function applyTheme(value: Theme) {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', value);
    localStorage.setItem('theme', value);
  }
}

function isValidTheme(value: string | null): value is Theme {
  return value !== null && (THEME_OPTIONS as readonly string[]).includes(value);
}

function createThemeStore() {
  const stored = typeof localStorage !== 'undefined' ? localStorage.getItem('theme') : null;
  const initial: Theme = isValidTheme(stored) ? stored : 'dark';
  const { subscribe, set: _set, update } = writable<Theme>(initial);

  applyTheme(initial);

  return {
    subscribe,
    set(value: Theme) {
      _set(value);
      applyTheme(value);
    },
    cycle() {
      update((current) => {
        const idx = THEME_OPTIONS.indexOf(current);
        const next = THEME_OPTIONS[(idx + 1) % THEME_OPTIONS.length];
        applyTheme(next);
        return next;
      });
    },
  };
}

function createBooleanPreferenceStore(key: string, defaultValue: boolean) {
  const stored = typeof localStorage !== 'undefined' ? localStorage.getItem(key) : null;
  const initial = stored === null ? defaultValue : stored === 'true';
  const { subscribe, set } = writable<boolean>(initial);

  return {
    subscribe,
    set(value: boolean) {
      set(value);
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(key, String(value));
      }
    },
  };
}

function createWindowOpacityStore() {
  const stored =
    typeof localStorage !== 'undefined' ? localStorage.getItem(WINDOW_OPACITY_STORAGE_KEY) : null;
  const initial = parseStoredWindowOpacity(stored);
  const { subscribe, set: _set } = writable<number>(initial);

  if (typeof document !== 'undefined') {
    void applyWindowOpacity(initial);
  }

  return {
    subscribe,
    set(value: number) {
      const clamped = Math.min(100, Math.max(0, Math.round(value)));
      _set(clamped);
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(WINDOW_OPACITY_STORAGE_KEY, String(clamped));
      }
      void applyWindowOpacity(clamped);
    },
  };
}

export const theme = createThemeStore();
export const metaPanelVisible = createBooleanPreferenceStore('metaPanelVisible', false);
export const sidebarVisible = createBooleanPreferenceStore('sidebarVisible', true);
export const compactDensity = createBooleanPreferenceStore('compactDensity', false);
/** Git diff viewer: open in edit mode by default. The pencil toggle switches to
 * the read-only unified diff view. */
export const gitEditable = createBooleanPreferenceStore('gitEditable', true);
/** Git diff viewer: auto-save edits 1.5s after the last change (on by default). */
export const gitAutoSave = createBooleanPreferenceStore('gitAutoSave', true);
export const windowOpacity = createWindowOpacityStore();
/** Desktop notifications (session completed, permission, errors, tasks). */
export const notificationsEnabled = createBooleanPreferenceStore('notificationsEnabled', true);
