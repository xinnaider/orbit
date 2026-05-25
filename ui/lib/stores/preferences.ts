import { writable } from 'svelte/store';

export const THEME_OPTIONS = [
  'dark',
  'light',
  'nord',
  'dracula',
  'catppuccin',
  'steel',
  'glass',
] as const;
export type Theme = (typeof THEME_OPTIONS)[number];

export const THEME_LABELS: Record<Theme, string> = {
  dark: 'Dark',
  light: 'Light (you are a monster?)',
  nord: 'Nord',
  dracula: 'Dracula',
  catppuccin: 'Catppuccin',
  steel: 'Steel Workshop',
  glass: 'Glass',
};

const GLASS_CHROME_KEY = 'glassChrome';

function applyTheme(value: Theme) {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', value);
    localStorage.setItem('theme', value);
  }
}

function applyGlassChrome(enabled: boolean) {
  if (typeof document !== 'undefined') {
    if (enabled) {
      document.documentElement.setAttribute('data-glass-chrome', 'true');
    } else {
      document.documentElement.removeAttribute('data-glass-chrome');
    }
    localStorage.setItem(GLASS_CHROME_KEY, String(enabled));
  }
}

function isValidTheme(value: string | null): value is Theme {
  return value !== null && (THEME_OPTIONS as readonly string[]).includes(value);
}

function readGlassChromeInitial(): boolean {
  const stored =
    typeof localStorage !== 'undefined' ? localStorage.getItem(GLASS_CHROME_KEY) : null;
  if (stored !== null) return stored === 'true';
  const themeStored = typeof localStorage !== 'undefined' ? localStorage.getItem('theme') : null;
  return themeStored === 'glass';
}

function createGlassChromeStore() {
  const initial = readGlassChromeInitial();
  const { subscribe, set: _set, update } = writable<boolean>(initial);

  applyGlassChrome(initial);

  return {
    subscribe,
    set(value: boolean) {
      _set(value);
      applyGlassChrome(value);
    },
    toggle() {
      update((current) => {
        const next = !current;
        applyGlassChrome(next);
        return next;
      });
    },
  };
}

export const glassChrome = createGlassChromeStore();

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
      if (value === 'glass') {
        glassChrome.set(true);
      }
    },
    cycle() {
      update((current) => {
        const idx = THEME_OPTIONS.indexOf(current);
        const next = THEME_OPTIONS[(idx + 1) % THEME_OPTIONS.length];
        applyTheme(next);
        if (next === 'glass') {
          glassChrome.set(true);
        }
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

export const theme = createThemeStore();
export const metaPanelVisible = createBooleanPreferenceStore('metaPanelVisible', false);
export const sidebarVisible = createBooleanPreferenceStore('sidebarVisible', true);
export const compactDensity = createBooleanPreferenceStore('compactDensity', false);
