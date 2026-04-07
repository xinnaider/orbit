import { writable } from 'svelte/store';
import type { DetailLevel, RightPanelTab } from '../types';

export type Theme = 'dark' | 'light';

function createThemeStore() {
  const stored =
    typeof localStorage !== 'undefined' ? (localStorage.getItem('theme') as Theme) : null;
  const initial: Theme = stored === 'light' ? 'light' : 'dark';
  const { subscribe, set, update } = writable<Theme>(initial);

  // Apply on init
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', initial);
  }

  return {
    subscribe,
    set(value: Theme) {
      set(value);
      if (typeof document !== 'undefined') {
        document.documentElement.setAttribute('data-theme', value);
        localStorage.setItem('theme', value);
      }
    },
    toggle() {
      update((current) => {
        const next: Theme = current === 'dark' ? 'light' : 'dark';
        if (typeof document !== 'undefined') {
          document.documentElement.setAttribute('data-theme', next);
          localStorage.setItem('theme', next);
        }
        return next;
      });
    },
  };
}

export const theme = createThemeStore();
export const detailLevel = writable<DetailLevel>('full');
export const rightPanelTab = writable<RightPanelTab>('agents');
