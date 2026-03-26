pub mod models;
pub mod session_watcher;
pub mod journal_reader;
pub mod diff_builder;
pub mod agent_tree;
pub mod keystroke_sender;
pub mod commands;
pub mod polling;

use polling::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::send_keystroke,
            commands::send_message,
            commands::get_journal,
            commands::get_diff,
            commands::get_file_versions,
            commands::get_subagent_journal,
        ])
        .setup(|app| {
            polling::start_polling(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
