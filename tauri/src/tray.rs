//! System tray: hide-on-close, show/focus from tray, quit from menu.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

static NOTIFICATIONS_ENABLED: AtomicBool = AtomicBool::new(true);

/// Handle to update the tray "mute/unmute" menu label from commands and menu events.
pub struct TrayNotifyMenu(pub MenuItem<tauri::Wry>);

pub fn notifications_enabled() -> bool {
    NOTIFICATIONS_ENABLED.load(Ordering::Relaxed)
}

pub fn set_notifications_enabled(enabled: bool) {
    NOTIFICATIONS_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn notify_menu_label(enabled: bool) -> &'static str {
    if enabled {
        "Mute notifications"
    } else {
        "Unmute notifications"
    }
}

pub fn sync_notify_menu_label(item: &MenuItem<tauri::Wry>) {
    let _ = item.set_text(notify_menu_label(notifications_enabled()));
}

pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "tray-show", "Show Orbit", true, None::<&str>)?;
    let notify_toggle = MenuItem::with_id(
        app,
        "tray-toggle-notify",
        notify_menu_label(notifications_enabled()),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "tray-quit", "Quit Orbit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &notify_toggle, &quit])?;

    let notify_for_menu = notify_toggle.clone();
    app.manage(TrayNotifyMenu(notify_toggle));

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))
        .expect("orbit tray icon bytes");

    let _tray = TrayIconBuilder::with_id("orbit-tray")
        .icon(icon)
        .menu(&menu)
        .tooltip("Orbit")
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            let id = event.id().as_ref();
            match id {
                "tray-show" => show_main_window(app),
                "tray-toggle-notify" => {
                    let next = !notifications_enabled();
                    set_notifications_enabled(next);
                    sync_notify_menu_label(&notify_for_menu);
                    let _ = app.emit("desktop:notifications-changed", next);
                }
                "tray-quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                toggle_main_window(app);
            }
        })
        .build(app)?;

    if let Some(window) = app.get_webview_window("main") {
        let window_clone = window.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window_clone.hide();
            }
        });
    }

    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

fn toggle_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let visible = window.is_visible().unwrap_or(true);
    if visible {
        let _ = window.hide();
    } else {
        show_main_window(app);
    }
}
