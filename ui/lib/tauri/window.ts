import { invoke } from './invoke';

/** Set native window opacity (0–100). No-op in mock/web when not available. */
export async function setWindowOpacity(opacity: number): Promise<void> {
  const clamped = Math.min(100, Math.max(0, Math.round(opacity)));
  await invoke<void>('set_window_opacity', { opacity: clamped });
}
