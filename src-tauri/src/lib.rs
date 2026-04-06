pub mod models;
pub mod journal_reader;
pub mod diff_builder;
pub mod agent_tree;
pub mod commands;
pub mod services;
pub mod ipc;

// Legacy observer modules — kept until Phase 5 cleanup
pub mod session_watcher;
pub mod keystroke_sender;
pub mod polling;

use std::sync::{Arc, Mutex};
use tauri::Manager;
use services::database::DatabaseService;
use services::session_manager::SessionManager;
use ipc::session::SessionState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Resolve app data directory for SQLite DB
            let data_dir = app.path().app_data_dir()
                .expect("Could not resolve app data dir");
            std::fs::create_dir_all(&data_dir)
                .expect("Could not create app data dir");

            let db_path = data_dir.join("agent-dashboard.db");
            let db = Arc::new(
                DatabaseService::open(&db_path)
                    .expect("Could not open database")
            );

            let session_manager = {
                let mut sm = SessionManager::new(db);
                sm.restore_from_db();
                Arc::new(Mutex::new(sm))
            };
            app.manage(SessionState(session_manager));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // New orchestrator commands
            ipc::session::create_session,
            ipc::session::list_sessions,
            ipc::session::stop_session,
            ipc::session::send_session_message,
            ipc::session::get_session_journal,
            ipc::project::create_project,
            ipc::project::list_projects,
            // Keep existing read-only commands
            commands::get_diff,
            commands::get_file_versions,
            commands::get_subagent_journal,
            commands::get_slash_commands,
            commands::list_project_files,
            commands::get_tasks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
