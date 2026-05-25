# Glass chrome (ORB-12)

## Objective

Port the legacy **Glass Tier** look from `orbit-tabs-v3.html` into the current Quiet Journal UI without forcing it on every theme.

## Behavior

- **Frosted chrome** toggle (palette menu): when enabled, sets `data-glass-chrome="true"` on `<html>` and applies translucent surfaces + `backdrop-filter: blur(12px)` to workspace chrome only (tab bar, panel headers, git tree/diff header).
- **Glass theme**: color preset (dark-tuned) that also enables frosted chrome when selected.
- **Quiet Journal** areas (feed, sidebar, composer) are unchanged in both modes.
- Per-theme `--glass-*` CSS variables supply accent-aware frosted colors for every palette.

## Edge cases

- Toggling frosted chrome off while on Glass theme keeps Glass colors but solid chrome.
- Preference persists in `localStorage` (`glassChrome`).
- Invalid stored theme falls back to `dark`; invalid `glassChrome` defaults to off unless theme is `glass`.

## Acceptance

- Default install: no frosted chrome, existing Quiet chrome unchanged.
- Frosted chrome on + any theme: glass styling on tab/header/git shell only.
- Glass theme selects dark glass palette and enables frosted chrome.
