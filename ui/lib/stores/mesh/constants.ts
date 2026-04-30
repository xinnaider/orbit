/**
 * Prefix applied to the `session.name` of every session spawned by the Mesh
 * pipeline runner. Orbit's Sidebar filters these out so Mesh-owned sessions
 * don't clutter the Sessions list.
 */
export const MESH_SESSION_PREFIX = 'mesh::';

export function isMeshSessionName(name: string | null | undefined): boolean {
  return typeof name === 'string' && name.startsWith(MESH_SESSION_PREFIX);
}

/** Default provider id for Mesh-spawned agents when the template doesn't pin one. */
export const MESH_DEFAULT_PROVIDER = 'claude-code';

/** CLI provider ids Mesh accepts in v1. Others appear in pickers but are disabled. */
export const MESH_SUPPORTED_PROVIDERS: readonly string[] = ['claude-code'];

export function isMeshSupportedProvider(id: string): boolean {
  return MESH_SUPPORTED_PROVIDERS.includes(id);
}

/** Default permission mode for Mesh-spawned sessions. */
export const MESH_DEFAULT_PERMISSION_MODE = 'ignore';

export type MeshNodeKind = 'agent' | 'browser' | 'skill' | 'note';

/** Default render dimensions per node kind when no width/height is persisted. */
export const MESH_NODE_DEFAULT_SIZE: Record<MeshNodeKind, { width: number; height: number }> = {
  agent: { width: 360, height: 280 },
  browser: { width: 420, height: 320 },
  skill: { width: 300, height: 220 },
  note: { width: 320, height: 240 },
};

/** System template name + provider used to back note nodes — hidden from the tray. */
export const MESH_NOTE_TEMPLATE_NAME = '__note__';
export const MESH_NOTE_PROVIDER = 'note';

export function isNoteTemplate(t: { name: string; provider: string }): boolean {
  return t.provider === MESH_NOTE_PROVIDER && t.name === MESH_NOTE_TEMPLATE_NAME;
}

/** Maximize toggle dimensions for an agent node. */
export const MESH_NODE_MAX_SIZE = { width: 1200, height: 720 };
