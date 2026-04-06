use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};

use crate::models::{Session, SessionId, JournalEntry};
use crate::services::session_manager::SessionManager;

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

    // Phase 2: slow — spawn PTY in background (non-blocking)
    let manager = Arc::clone(&state.0);
    let session_clone = session.clone();
    std::thread::spawn(move || {
        SessionManager::do_spawn(manager, app, session_clone, prompt);
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
) -> Result<(), String> {
    state.0.lock().unwrap().send_message(session_id, &message)
}

#[tauri::command]
pub fn get_session_journal(session_id: SessionId, state: State<SessionState>) -> Vec<JournalEntry> {
    state.0.lock().unwrap().get_journal(session_id)
}
