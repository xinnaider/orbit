pub mod agent_tree;
pub mod commands;
pub mod diff_builder;
pub mod ipc;
pub mod journal;
pub mod models;
pub mod providers;
pub mod services;

#[cfg(test)]
pub mod test_utils;

use ipc::session::{ProviderRegistryState, SessionState};
use ipc::terminal::PtyManagerState;
use providers::ProviderRegistry;
use services::database::DatabaseService;
use services::pty_manager::PtyManager;
use services::session_manager::SessionManager;
use std::sync::{Arc, Mutex, RwLock};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // Resolve app data directory for SQLite DB
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("Could not resolve app data dir");
            std::fs::create_dir_all(&data_dir).expect("Could not create app data dir");

            // Initialize encryption key for storing secrets (stored at {app_data}/orbit.key)
            services::crypto::init(&data_dir);

            let db_path = data_dir.join("agent-dashboard.db");
            let db = Arc::new(DatabaseService::open(&db_path).expect("Could not open database"));

            let session_manager = Arc::new(RwLock::new(SessionManager::new(db)));
            app.manage(SessionState(session_manager));

            // Provider registry — maps provider IDs to trait implementations
            let mut registry = ProviderRegistry::new();
            registry.register(Box::new(providers::claude::ClaudeProvider));
            registry.register(Box::new(providers::codex::CodexProvider));
            registry.register(Box::new(providers::opencode::OpenCodeProvider));
            registry.register(Box::new(providers::acp::AcpProvider::new(
                "gemini-cli",
                "Gemini CLI",
                "gemini",
                &["--acp"],
            )));
            registry.register(Box::new(providers::acp::AcpProvider::new(
                "copilot-cli",
                "Copilot CLI",
                "copilot",
                &["--acp"],
            )));
            app.manage(ProviderRegistryState(Arc::new(registry)));

            let pty_manager = Arc::new(Mutex::new(PtyManager::new(app.handle().clone())));
            app.manage(PtyManagerState(pty_manager));

            // Set window icon programmatically — bypasses Windows icon cache in dev mode
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(icon) =
                    tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))
                {
                    let _ = window.set_icon(icon);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::session::create_session,
            ipc::session::list_sessions,
            ipc::session::stop_session,
            ipc::session::send_session_message,
            ipc::session::get_session_journal,
            ipc::session::get_session_journal_page,
            ipc::session::check_claude,
            ipc::session::diagnose_spawn,
            ipc::session::rename_session,
            ipc::session::delete_session,
            ipc::session::update_session_model,
            ipc::session::update_session_effort,
            ipc::session::set_session_api_key,
            ipc::session::save_provider_key,
            ipc::session::load_provider_key,
            ipc::session::delete_provider_key,
            ipc::session::test_ssh,
            ipc::session::clear_attention,
            ipc::session::respond_permission,
            ipc::project::create_project,
            ipc::project::list_projects,
            commands::agents::get_subagents,
            commands::diff::get_diff,
            commands::diff::get_file_versions,
            commands::files::get_subagent_journal,
            commands::plugins::get_slash_commands,
            commands::files::list_project_files,
            commands::files::read_file_content,
            commands::tasks::get_tasks,
            commands::stats::get_claude_usage_stats,
            commands::stats::get_rate_limits,
            ipc::updater::check_update,
            ipc::updater::install_update,
            commands::stats::get_changelog,
            commands::providers::get_providers,
            commands::providers::check_env_var,
            commands::providers::diagnose_provider,
            commands::orchestration::setup_orchestration,
            commands::orchestration::check_orchestration,
            ipc::terminal::pty_create,
            ipc::terminal::pty_write,
            ipc::terminal::pty_resize,
            ipc::terminal::pty_kill,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
