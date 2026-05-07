import { writable } from 'svelte/store';

export const rawJournal = writable<Map<number, string[]>>(new Map());
