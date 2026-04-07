import { writable } from 'svelte/store';
import type { DetailLevel, RightPanelTab } from '../types';

export const THEME_OPTIONS = ['dark', 'light', 'nord', 'dracula', 'catppuccin'] as const;
export type Theme = (typeof THEME_OPTIONS)[number];

export const THEME_LABELS: Record<Theme, string> = {
  dark: 'Dark',
  light: 'Light (you are a monster?)',
  nord: 'Nord',
  dracula: 'Dracula',
  catppuccin: 'Catppuccin',
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

function createMetaPanelVisibleStore() {
  const stored =
    typeof localStorage !== 'undefined' ? localStorage.getItem('metaPanelVisible') : null;
  const initial = stored === null ? true : stored === 'true';
  const { subscribe, set } = writable<boolean>(initial);

  return {
    subscribe,
    set(value: boolean) {
      set(value);
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('metaPanelVisible', String(value));
      }
    },
  };
}

export const theme = createThemeStore();
export const detailLevel = writable<DetailLevel>('full');
export const rightPanelTab = writable<RightPanelTab>('agents');
export const metaPanelVisible = createMetaPanelVisibleStore();
