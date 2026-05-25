# Desktop tray and notifications

## Behavior

- Closing the main window **hides** Orbit to the system tray; the app keeps running.
- Tray icon uses the Orbit app icon (same as window).
- **Left-click** tray: show/hide main window.
- **Right-click** tray menu: Show Orbit, Mute/Unmute notifications, Quit Orbit.
- **Quit** from the tray menu exits the process.

## Notifications

Native OS notifications (via `tauri-plugin-notification`) when:

| Event | Title |
|-------|--------|
| Session completed | Orbit — session finished |
| Permission required | Orbit — approval needed |
| Session error | Orbit — session error |
| Rate limit | Orbit — rate limit |
| Task list update | Orbit — tasks updated (throttled 8s per session) |
| Spawn failure | Orbit — session failed |

Respects:

- Global **notify** toggle (sidebar footer + tray menu + `notificationsEnabled` in localStorage).
- Per-session **mute** (existing sidebar context menu).

## Platform compatibility

| Platform | Tray | Notifications | Notes |
|----------|------|---------------|--------|
| **Windows 10+** | System tray | Toast notifications | Full support. Dev builds may show PowerShell branding until installed MSI. |
| **macOS** (Intel / Apple Silicon) | Menu bar | Notification Center | Grant notification permission on first prompt. Close button hides; use tray **Quit** or app menu to exit. |
| **Ubuntu 22.04+** / Linux | Status notifier (Ayatana/AppIndicator) | `libnotify` | Install `libayatana-appindicator3-1` or `libappindicator3-1` if tray icon missing. Tray requires a menu on some DEs (we always set one). |

### Linux dependencies (CI / user machines)

```bash
# Debian/Ubuntu
sudo apt install libayatana-appindicator3-1 libnotify-bin

# Fedora
sudo dnf install libappindicator-gtk3 libnotify
```

### Known limitations

- **Web mode** (`dev:mock` / browser): no tray or desktop notifications.
- **WSL**: tray/notifications depend on Windows host integration; not officially supported.
- Tray menu label for mute does not refresh until next launch (toggle still works).
- Task-update notifications are throttled to avoid spam during rapid tool bursts.

## IPC

- `get_desktop_notifications_enabled` / `set_desktop_notifications_enabled` — sync Rust atomic with UI store.
- Event `desktop:notifications-changed` — tray menu toggles mute; frontend updates checkbox.
