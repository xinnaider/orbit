use crate::ipc::IpcError;
use std::path::Path;

/// Find the orbit-mcp binary path. Checks:
/// 1. Next to the current executable (production install)
/// 2. In the cargo target directory (dev mode)
/// 3. In PATH
fn find_orbit_mcp() -> Option<String> {
    // 1. Next to current exe
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name(orbit_mcp_filename());
        if sibling.is_file() {
            return Some(sibling.to_string_lossy().into_owned());
        }
    }

    // 2. Cargo target dir (dev)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(target_dir) = exe.parent() {
            let dev_path = target_dir.join(orbit_mcp_filename());
            if dev_path.is_file() {
                return Some(dev_path.to_string_lossy().into_owned());
            }
        }
    }

    // 3. PATH lookup
    crate::services::spawn_manager::find_cli_in_path("orbit-mcp")
}

fn orbit_mcp_filename() -> &'static str {
    if cfg!(windows) {
        "orbit-mcp.exe"
    } else {
        "orbit-mcp"
    }
}

/// Write `.mcp.json` to a project directory, configuring orbit-mcp as an MCP server.
/// Returns the path to the orbit-mcp binary used.
#[tauri::command]
pub fn setup_orchestration(project_path: String) -> Result<String, IpcError> {
    let mcp_bin = find_orbit_mcp()
        .ok_or_else(|| IpcError::Other("orbit-mcp binary not found".to_string()))?;

    let mcp_config = serde_json::json!({
        "mcpServers": {
            "orbit": {
                "command": mcp_bin,
            }
        }
    });

    let config_path = Path::new(&project_path).join(".mcp.json");
    let content =
        serde_json::to_string_pretty(&mcp_config).map_err(|e| IpcError::Other(e.to_string()))?;

    std::fs::write(&config_path, content)
        .map_err(|e| IpcError::Other(format!("failed to write .mcp.json: {e}")))?;

    Ok(mcp_bin)
}

/// Check if orchestration is available (orbit-mcp binary exists).
#[tauri::command]
pub fn check_orchestration() -> serde_json::Value {
    match find_orbit_mcp() {
        Some(path) => serde_json::json!({ "available": true, "path": path }),
        None => serde_json::json!({ "available": false, "path": null }),
    }
}
