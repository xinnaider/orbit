pub mod agent_tree;
pub mod commands;
pub mod diff_builder;
pub mod ipc;
pub mod journal;
pub mod mcp_proxy;
pub mod mcp_transport;
pub mod models;
pub mod providers;
pub mod services;

#[cfg(test)]
pub mod test_utils;

use ipc::session::{ProviderRegistryState, SessionState};
use ipc::terminal::PtyManagerState;
use providers::ProviderRegistry;
use services::database::DatabaseService;
use services::http_server::{HttpState, StreamEvent};
use services::pty_manager::PtyManager;
use services::session_manager::SessionManager;
use std::sync::{Arc, Mutex, RwLock};
use tauri::{Listener, Manager};
use tokio::sync::broadcast;

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
            app.manage(SessionState(Arc::clone(&session_manager)));

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
            let registry = Arc::new(registry);
            app.manage(ProviderRegistryState(Arc::clone(&registry)));

            // Start embedded MCP server — shares SessionManager and ProviderRegistry
            let mcp_handler = Arc::new(ipc::mcp::McpHandler::new(
                Arc::clone(&session_manager),
                Arc::clone(&registry),
                app.handle().clone(),
            ));
            let handler_fn = {
                let h = Arc::clone(&mcp_handler);
                move |request: &str| -> String { h.handle(request) }
            };
            let _mcp_transport = mcp_transport::McpTransport::start(Arc::new(handler_fn));

            // Start HTTP API server if enabled
            let (stream_tx, _) = broadcast::channel::<StreamEvent>(256);
            {
                let sm = Arc::clone(&session_manager);
                let reg = Arc::clone(&registry);
                let app_handle = app.handle().clone();
                let db_ref = {
                    let m = sm.read().unwrap_or_else(|e| e.into_inner());
                    Arc::clone(&m.db)
                };
                let tx = stream_tx.clone();

                let enabled = db_ref
                    .get_http_setting("enabled")
                    .unwrap_or(None)
                    .map(|v| v == "true")
                    .unwrap_or(false);

                if enabled {
                    // Set env vars so SSH spawns auto-inject HTTP connection info
                    std::env::set_var("_ORBIT_HTTP_ENABLED", "true");

                    let host = db_ref
                        .get_http_setting("host")
                        .unwrap_or(None)
                        .unwrap_or_else(|| "127.0.0.1".to_string());
                    let port: u16 = db_ref
                        .get_http_setting("port")
                        .unwrap_or(None)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(9999);

                    std::env::set_var("_ORBIT_HTTP_PORT", port.to_string());

                    // Auto-generate a key for SSH sessions if none exists
                    {
                        use crate::services::http_server::hash_api_key;
                        let key = format!("orbit_ssh_{}", uuid::Uuid::new_v4().simple());
                        let key_hash = hash_api_key(&key);
                        let _ = db_ref.create_api_key("ssh-auto", "SSH auto-generated", &key_hash);
                        std::env::set_var("_ORBIT_HTTP_SSH_KEY", &key);
                    }

                    // Resolve frontend build dir for web UI serving
                    let frontend_dir = {
                        let exe = std::env::current_exe().unwrap_or_default();
                        let exe_dir = exe.parent().unwrap_or(std::path::Path::new("."));
                        let candidates = [
                            // Prod: next to exe
                            exe_dir.join("build"),
                            // Dev: exe is in tauri/target/debug/
                            exe_dir.join("../../../build"),
                            exe_dir.join("../../build"),
                            exe_dir.join("../build"),
                            // CWD-relative fallback
                            std::path::PathBuf::from("build"),
                        ];
                        let found = candidates
                            .iter()
                            .find(|p| p.join("index.html").exists())
                            .cloned();
                        if found.is_none() {
                            eprintln!(
                                "[orbit:http] WARNING: no build/ dir found (exe={})",
                                exe.display()
                            );
                        }
                        found
                    };

                    if let Some(ref dir) = frontend_dir {
                        eprintln!("[orbit:http] serving web UI from {}", dir.display());
                    }

                    let state = HttpState {
                        session_manager: sm,
                        registry: reg,
                        app: app_handle,
                        db: db_ref,
                        stream_tx: tx,
                        frontend_dir,
                    };

                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .expect("tokio runtime");
                        if let Err(e) =
                            rt.block_on(services::http_server::start(state, &host, port))
                        {
                            eprintln!("[orbit:http] server error: {e}");
                        }
                    });
                }
            }

            // Bridge Tauri events → HTTP broadcast channel for WebSocket clients
            {
                let events = [
                    "session:created",
                    "session:running",
                    "session:output",
                    "session:state",
                    "session:stopped",
                    "session:error",
                    "session:rate-limit",
                    "session:task-update",
                    "session:subagent-created",
                ];
                for event_name in events {
                    let tx = stream_tx.clone();
                    let name = event_name.to_string();
                    app.listen(event_name, move |event| {
                        let payload_str = event.payload();
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(payload_str) {
                            let session_id =
                                data.get("sessionId").and_then(|v| v.as_i64()).unwrap_or(0);
                            let _ = tx.send(StreamEvent {
                                session_id,
                                event_type: name.clone(),
                                data,
                            });
                        }
                    });
                }
            }

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
            commands::git::git_overview,
            commands::git::git_diff_file,
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
            ipc::http_api::generate_api_key,
            ipc::http_api::list_api_keys,
            ipc::http_api::revoke_api_key,
            ipc::http_api::get_http_settings,
            ipc::http_api::set_http_settings,
            ipc::http_api::get_lan_ip,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
