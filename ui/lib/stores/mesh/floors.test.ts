import { describe, expect, it, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';

vi.mock('../../tauri/mesh', () => ({
  meshListFloors: vi.fn(),
  meshCreateFloor: vi.fn(),
  meshRenameFloor: vi.fn(),
  meshDeleteFloor: vi.fn(),
}));

import * as tauriMod from '../../tauri/mesh';
import {
  floorsStore,
  activeFloorId,
  loadFloors,
  addFloor,
  renameFloor,
  removeFloor,
} from './floors';

describe('floors store', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    floorsStore.set([]);
    activeFloorId.set(null);
  });

  it('loads floors from backend', async () => {
    (tauriMod.meshListFloors as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce([
      { id: 1, name: 'A', position: 0, createdAt: '2026-04-24' },
    ]);
    await loadFloors();
    const floors = get(floorsStore);
    expect(floors.length).toBe(1);
    expect(floors[0].name).toBe('A');
  });

  it('adds a floor optimistically on create', async () => {
    (tauriMod.meshCreateFloor as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
      id: 42,
      name: 'New',
      position: 0,
      createdAt: '2026-04-24',
    });
    const f = await addFloor('New');
    expect(f.id).toBe(42);
    const floors = get(floorsStore);
    expect(floors).toContainEqual(expect.objectContaining({ id: 42 }));
  });

  it('renames a floor and updates store in-place', async () => {
    floorsStore.set([{ id: 1, name: 'Old', position: 0, createdAt: '2026-04-24' }]);
    (tauriMod.meshRenameFloor as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
      undefined
    );
    await renameFloor(1, 'New');
    expect(get(floorsStore)[0].name).toBe('New');
  });

  it('removes a floor from store after delete', async () => {
    floorsStore.set([
      { id: 1, name: 'A', position: 0, createdAt: '2026-04-24' },
      { id: 2, name: 'B', position: 0, createdAt: '2026-04-24' },
    ]);
    (tauriMod.meshDeleteFloor as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce(
      undefined
    );
    await removeFloor(1);
    const rest = get(floorsStore);
    expect(rest.length).toBe(1);
    expect(rest[0].id).toBe(2);
  });
});
