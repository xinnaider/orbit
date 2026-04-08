use tauri::State;

use crate::ipc::session::SessionState;
use crate::models::Project;

#[tauri::command]
pub fn create_project(
    name: String,
    path: String,
    state: State<SessionState>,
) -> Result<Project, String> {
    state
        .write()
        .db
        .create_project(&name, &path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_projects(state: State<SessionState>) -> Vec<Project> {
    state.read().db.get_projects().unwrap_or_default()
}
