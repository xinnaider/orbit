use crate::tray::{self, TrayNotifyMenu};
use tauri::State;

#[tauri::command]
pub fn get_desktop_notifications_enabled() -> bool {
    tray::notifications_enabled()
}

#[tauri::command]
pub fn set_desktop_notifications_enabled(enabled: bool, menu: State<'_, TrayNotifyMenu>) {
    tray::set_notifications_enabled(enabled);
    tray::sync_notify_menu_label(&menu.0);
}
