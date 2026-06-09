export type ShortcutPlatform = 'mac' | 'windows' | 'linux' | 'other';

/** Detect OS for shortcut label formatting (WebView / browser). */
export function detectPlatform(): ShortcutPlatform {
  if (typeof navigator === 'undefined') return 'other';
  const platform = navigator.platform?.toLowerCase() ?? '';
  const ua = navigator.userAgent?.toLowerCase() ?? '';
  if (platform.includes('mac') || ua.includes('mac')) return 'mac';
  if (platform.startsWith('win') || ua.includes('windows')) return 'windows';
  if (platform.includes('linux') || ua.includes('linux')) return 'linux';
  return 'other';
}

export function isMacOS(): boolean {
  return detectPlatform() === 'mac';
}

/** Modifier label shown in UI hints: ⌘ on macOS, Ctrl elsewhere. */
export function modKeyLabel(): string {
  return isMacOS() ? '⌘' : 'Ctrl';
}

/** Chord hint such as mod+I → ⌘I (mac) or Ctrl+I (Windows/Linux). */
export function formatModChord(key: string, separator?: string): string {
  const sep = separator ?? (isMacOS() ? '' : '+');
  return `${modKeyLabel()}${sep}${key}`;
}

export function inspectorToggleHint(): string {
  return formatModChord('I');
}

export function sidebarToggleHint(): string {
  return formatModChord('B');
}

export function splitPaneHint(): string {
  return isMacOS() ? '⌘\\' : 'Ctrl+\\';
}

export function workspaceShortcutsFooter(): string {
  return `drag sessions into panes • ${splitPaneHint()} split • ${inspectorToggleHint()} inspect`;
}

/** Browse affordance on path/key fields (not a keyboard shortcut). */
export function browseButtonLabel(): string {
  return isMacOS() ? '⌘' : '…';
}
