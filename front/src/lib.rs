pub mod agent_tree;
pub mod commands;
pub mod diff_builder;
pub mod ipc;
pub mod journal_reader;
pub mod models;
pub mod services;

use ipc::session::SessionState;
use services::database::DatabaseService;
use services::session_manager::SessionManager;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // Resolve app data directory for SQLite DB
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("Could not resolve app data dir");
            std::fs::create_dir_all(&data_dir).expect("Could not create app data dir");

            let db_path = data_dir.join("agent-dashboard.db");
            let db = Arc::new(DatabaseService::open(&db_path).expect("Could not open database"));

            let session_manager = {
                let mut sm = SessionManager::new(db);
                sm.restore_from_db();
                Arc::new(Mutex::new(sm))
            };
            app.manage(SessionState(session_manager));

            // Set window icon programmatically — bypasses Windows icon cache in dev mode
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(icon) =
                    tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))
                {
                    let _ = window.set_icon(icon);
                }
            }

            // Set up Orbit statusline capture (non-fatal)
            let _ = commands::setup_orbit_statusline();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::session::create_session,
            ipc::session::list_sessions,
            ipc::session::stop_session,
            ipc::session::send_session_message,
            ipc::session::get_session_journal,
            ipc::session::check_claude,
            ipc::session::diagnose_spawn,
            ipc::session::rename_session,
            ipc::session::delete_session,
            ipc::project::create_project,
            ipc::project::list_projects,
            commands::get_diff,
            commands::get_file_versions,
            commands::get_subagent_journal,
            commands::get_slash_commands,
            commands::list_project_files,
            commands::get_tasks,
            commands::get_claude_usage_stats,
            commands::setup_orbit_statusline,
            commands::read_session_status,
            ipc::updater::check_update,
            ipc::updater::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
