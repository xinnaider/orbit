import { writable } from 'svelte/store';

const STORAGE_KEY = 'mesh:node-sessions';

function loadInitial(): Record<string, number> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    if (typeof parsed === 'object' && parsed !== null) return parsed;
  } catch {
    /* corrupted entry — fall through to empty */
  }
  return {};
}

/** Map from graph node id (SvelteFlow string) → Orbit session id. */
export const meshNodeSessions = writable<Record<string, number>>(loadInitial());

meshNodeSessions.subscribe((val) => {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(val));
  } catch {
    /* ignore quota errors */
  }
});
