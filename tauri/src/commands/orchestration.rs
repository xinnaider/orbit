use crate::ipc::IpcError;
use std::path::Path;

/// Write provider-specific MCP configs to a project directory, configuring orbit-mcp as an MCP server.
/// Returns the path to the orbit-mcp binary used.
#[tauri::command]
pub fn setup_orchestration(project_path: String) -> Result<String, IpcError> {
    let mcp_bin = crate::services::mcp_config::find_orbit_mcp()
        .ok_or_else(|| IpcError::Other("orbit-mcp binary not found".to_string()))?;

    crate::services::mcp_config::write_orbit_mcp_configs(Path::new(&project_path), &mcp_bin)
        .map_err(IpcError::Other)?;

    Ok(mcp_bin)
}

/// Check if orchestration is available (orbit-mcp binary exists).
#[tauri::command]
pub fn check_orchestration() -> serde_json::Value {
    match crate::services::mcp_config::find_orbit_mcp() {
        Some(path) => serde_json::json!({ "available": true, "path": path }),
        None => serde_json::json!({ "available": false, "path": null }),
    }
}
