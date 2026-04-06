use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};

use crate::models::{Session, SessionId, JournalEntry};
use crate::services::session_manager::SessionManager;
use crate::services::spawn_manager::find_claude;

pub struct SessionState(pub Arc<Mutex<SessionManager>>);

/// Create a session: returns immediately after creating the DB record (status = "initializing").
/// The actual Claude process spawns in a background thread — non-blocking.
/// Frontend should listen to "session:running" (ready) or "session:error" (spawn failed).
#[tauri::command]
pub fn create_session(
    project_path: String,
    prompt: String,
    model: Option<String>,
    permission_mode: Option<String>,
    session_name: Option<String>,
    state: State<SessionState>,
    app: AppHandle,
) -> Result<Session, String> {
    let mode = permission_mode.unwrap_or_else(|| "ignore".to_string());

    // Phase 1: fast — create DB record, return session immediately
    let session = {
        let mut m = state.0.lock().unwrap();
        m.init_session(
            &project_path,
            session_name.as_deref(),
            &mode,
            model.as_deref(),
        )?
    };

    // Emit session:created immediately so frontend shows the session
    use tauri::Emitter;
    let _ = app.emit("session:created", &session);

    // Phase 2: spawn Claude in background with -p flag
    let manager = Arc::clone(&state.0);
    let session_id = session.id;
    std::thread::spawn(move || {
        SessionManager::do_spawn(manager, app, session_id, prompt);
    });

    Ok(session)
}

#[tauri::command]
pub fn list_sessions(state: State<SessionState>) -> Vec<Session> {
    state.0.lock().unwrap().get_sessions()
}

#[tauri::command]
pub fn stop_session(session_id: SessionId, state: State<SessionState>) -> Result<(), String> {
    state.0.lock().unwrap().stop_session(session_id)
}

#[tauri::command]
pub fn send_session_message(
    session_id: SessionId,
    message: String,
    state: State<SessionState>,
    app: AppHandle,
) -> Result<(), String> {
    SessionManager::send_message(Arc::clone(&state.0), app, session_id, message)
}

#[tauri::command]
pub fn get_session_journal(session_id: SessionId, state: State<SessionState>) -> Vec<JournalEntry> {
    state.0.lock().unwrap().get_journal(session_id)
}

/// Diagnostic: check if claude CLI is available and return its path or an error message.
#[tauri::command]
pub fn check_claude() -> serde_json::Value {
    match find_claude() {
        Some(path) => serde_json::json!({ "found": true, "path": path }),
        None => {
            let path_var = std::env::var("PATH").unwrap_or_default();
            serde_json::json!({
                "found": false,
                "path": null,
                "searchedPath": path_var,
                "hint": "Install with: npm install -g @anthropic-ai/claude-code"
            })
        }
    }
}

/// Diagnostic: fast check of claude availability using regular process commands (no PTY).
/// Does NOT block the UI.
#[tauri::command]
pub fn diagnose_spawn() -> serde_json::Value {
    use std::process::Command;

    // 1. find_claude() — static path search
    let claude_path = find_claude();

    // 2. `where claude` (Windows) or `which claude` (Unix) — 2s timeout
    #[cfg(windows)]
    let where_out = Command::new("where")
        .arg("claude")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|e| format!("where failed: {e}"));

    #[cfg(not(windows))]
    let where_out = Command::new("which")
        .arg("claude")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|e| format!("which failed: {e}"));

    // 3. `claude --version` using augmented PATH
    let aug_path = {
        let current = std::env::var("PATH").unwrap_or_default();
        #[cfg(windows)]
        if let Some(home) = dirs::home_dir() {
            format!("{};{};{};{}",
                home.join("AppData").join("Roaming").join("npm").to_string_lossy(),
                home.join("AppData").join("Local").join("pnpm").to_string_lossy(),
                home.join("AppData").join("Local").join("nvm").to_string_lossy(),
                current)
        } else { current }
        #[cfg(not(windows))]
        current
    };

    #[cfg(windows)]
    let version_out = Command::new("cmd")
        .args(["/c", "claude", "--version"])
        .env("PATH", &aug_path)
        .output()
        .map(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            if !stdout.is_empty() { stdout } else { stderr }
        })
        .unwrap_or_else(|e| format!("version check failed: {e}"));

    #[cfg(not(windows))]
    let version_out = Command::new("claude")
        .arg("--version")
        .env("PATH", &aug_path)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|e| format!("version check failed: {e}"));

    serde_json::json!({
        "claudeFound": claude_path.is_some() || !where_out.is_empty(),
        "claudePath": claude_path,
        "whereOutput": where_out,
        "versionOutput": version_out,
        "augmentedPath": aug_path.chars().take(400).collect::<String>(),
        "processPath": std::env::var("PATH").unwrap_or_default().chars().take(200).collect::<String>(),
    })
}

/// Rename a session.
#[tauri::command]
pub fn rename_session(
    session_id: SessionId,
    name: String,
    state: State<SessionState>,
) -> Result<(), String> {
    state.0.lock().unwrap().rename_session(session_id, &name)
}

/// Delete a session (removes from DB, stops if running).
#[tauri::command]
pub fn delete_session(
    session_id: SessionId,
    state: State<SessionState>,
) -> Result<(), String> {
    state.0.lock().unwrap().delete_session(session_id)
}
