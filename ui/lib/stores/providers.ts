import { writable, derived } from 'svelte/store';
import type { CliBackend } from '../tauri';

export const backends = writable<CliBackend[]>([]);

export interface ProviderCaps {
  supportsEffort: boolean;
  supportsSsh: boolean;
  supportsSubagents: boolean;
  hasSubProviders: boolean;
  effortLevels: Record<string, string[]>;
}

const DEFAULT_CAPS: ProviderCaps = {
  supportsEffort: false,
  supportsSsh: false,
  supportsSubagents: false,
  hasSubProviders: false,
  effortLevels: {},
};

/** Map of provider ID → capabilities, derived from the backends list. */
export const providerCaps = derived(backends, ($backends) => {
  const map = new Map<string, ProviderCaps>();
  for (const b of $backends) {
    map.set(b.id, {
      supportsEffort: b.supportsEffort,
      supportsSsh: b.supportsSsh,
      supportsSubagents: b.supportsSubagents,
      hasSubProviders: b.hasSubProviders,
      effortLevels: b.effortLevels,
    });
  }
  return map;
});

/** Get capabilities for a provider ID, with safe defaults. */
export function getCaps(map: Map<string, ProviderCaps>, providerId: string): ProviderCaps {
  return map.get(providerId) ?? DEFAULT_CAPS;
}
