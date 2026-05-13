import { get } from 'svelte/store';
import { assignSession, workspace } from './workspace';
import { selectedSessionId, sessions, upsertSession, type Session } from './sessions';

export function upsertAndOpenSession(session: Session): void {
  sessions.update((list) => upsertSession(list, session));
  const ws = get(workspace);
  if (ws.focusedPaneId) assignSession(ws.focusedPaneId, session.id);
}

export function upsertSessionFromEvent(session: Session): void {
  sessions.update((list) => upsertSession(list, session));
  if (get(selectedSessionId)) return;

  const ws = get(workspace);
  if (ws.focusedPaneId) assignSession(ws.focusedPaneId, session.id);
}
