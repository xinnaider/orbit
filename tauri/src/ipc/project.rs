use tauri::State;

use crate::ipc::session::SessionState;
use crate::ipc::IpcError;
use crate::models::Project;

#[tauri::command]
pub fn create_project(
    name: String,
    path: String,
    state: State<SessionState>,
) -> Result<Project, IpcError> {
    state
        .read()
        .db
        .create_project(&name, &path)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_projects(state: State<SessionState>) -> Vec<Project> {
    state.read().db.get_projects().unwrap_or_default()
}
