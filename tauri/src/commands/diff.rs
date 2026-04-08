use crate::diff_builder;
use crate::models::*;

#[tauri::command]
pub fn get_diff(
    session_id: String,
    file_hash: String,
    from_version: u32,
    to_version: u32,
) -> Result<DiffResult, String> {
    diff_builder::build_diff(&session_id, &file_hash, from_version, to_version)
        .ok_or_else(|| "Could not build diff".to_string())
}

#[tauri::command]
pub fn get_file_versions(session_id: String) -> Vec<diff_builder::FileVersionInfo> {
    diff_builder::get_file_versions(&session_id)
}
