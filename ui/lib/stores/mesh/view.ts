import { writable } from 'svelte/store';

export type AppView = 'sessions' | 'mesh';

export const activeView = writable<AppView>('sessions');
