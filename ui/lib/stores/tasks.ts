import { writable } from 'svelte/store';

/// Emits a sessionId whenever a providers detects a task update in the output stream.
/// TasksList components can subscribe and reload when their sessionId matches.
export const taskUpdateTrigger = writable<number | null>(null);
