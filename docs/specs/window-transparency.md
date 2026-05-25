# Window transparency (ORB-13)

## Objective

Let users adjust the **entire Orbit window** opacity (0–100%) so desktop content can show through, similar to terminal emulators. The setting is theme-independent, applies in real time, and is stored in user preferences.

## Behavior

- **Opacity slider** in the palette menu (sidebar header): 100% = fully opaque (default), lower values increase transparency.
- **Native window opacity** via a Tauri command (`set_window_opacity`) on Windows, macOS, and Linux.
- **Transparent window** enabled in `tauri.conf.json`; shadow disabled while transparency is active (Windows).
- **Optional acrylic/blur** platform effect when opacity &lt; 100% (Windows/macOS via Tauri `set_effects`).
- **Readability**: when opacity &lt; 100%, `html[data-window-opacity]` boosts surface contrast (CSS variables), independent of theme.
- **Persistence**: `localStorage` key `windowOpacity` (0–100).
- **Mock / web**: slider updates CSS only; native command is skipped.

## Edge cases

- Invalid stored values fall back to 100.
- Opacity 0 is clamped to 1% minimum so the window stays usable.
- Linux: GTK opacity; compositor required for see-through (standard on Ubuntu 22.04+).

## Acceptance

- Slider changes opacity immediately without restart.
- Preference survives app restart.
- All themes keep working; no dedicated glass theme required.
- Text and panels remain readable at ~70% opacity.
