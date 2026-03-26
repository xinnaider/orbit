use tauri::State;

use crate::journal_reader;
use crate::keystroke_sender;
use crate::models::*;
use crate::polling::AppState;
use crate::session_watcher;
use crate::diff_builder;

#[tauri::command]
pub fn send_keystroke(session_id: String, key: String, state: State<AppState>) -> Result<(), String> {
    let _journal_states = state.journal_states.lock().map_err(|e| e.to_string())?;

    // Find the PID for this session
    let live_sessions = session_watcher::discover_live_sessions();
    let session = live_sessions.iter()
        .find(|s| s.session_id == session_id)
        .ok_or("Session not found")?;

    keystroke_sender::send_keys(session.pid, &key)?;
    Ok(())
}

#[tauri::command]
pub fn send_message(session_id: String, message: String, state: State<AppState>) -> Result<(), String> {
    // send_keys already appends Enter
    send_keystroke(session_id, message, state)
}

#[tauri::command]
pub fn get_journal(session_id: String, state: State<AppState>) -> Result<Vec<JournalEntry>, String> {
    let journal_states = state.journal_states.lock().map_err(|e| e.to_string())?;

    match journal_states.get(&session_id) {
        Some(js) => {
            let mut entries = js.entries.clone();
            for entry in &mut entries {
                entry.session_id = session_id.clone();
            }
            Ok(entries)
        }
        None => Ok(vec![]),
    }
}

#[tauri::command]
pub fn get_diff(session_id: String, file_hash: String, from_version: u32, to_version: u32) -> Result<DiffResult, String> {
    diff_builder::build_diff(&session_id, &file_hash, from_version, to_version)
        .ok_or_else(|| "Could not build diff".to_string())
}

#[tauri::command]
pub fn get_file_versions(session_id: String) -> Vec<diff_builder::FileVersionInfo> {
    diff_builder::get_file_versions(&session_id)
}

#[tauri::command]
pub fn get_subagent_journal(session_id: String, subagent_id: String) -> Vec<JournalEntry> {
    // Find the subagent JSONL path
    let projects_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("projects"),
        None => return vec![],
    };

    let entries = match std::fs::read_dir(&projects_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for project_entry in entries.flatten() {
        let jsonl_path = project_entry.path()
            .join(&session_id)
            .join("subagents")
            .join(format!("{}.jsonl", &subagent_id));

        if jsonl_path.exists() {
            let state = journal_reader::parse_journal(&jsonl_path, 0, None);
            let mut result = state.entries;
            for entry in &mut result {
                entry.session_id = subagent_id.clone();
            }
            return result;
        }
    }

    vec![]
}
