use crate::ipc::IpcError;
use crate::services::mcp_config::{self, McpLaunch, MCP_STDIO_ARG};
use std::path::Path;

/// Write provider-specific MCP configs to a project directory.
/// Returns a human-readable launch description (command + args).
#[tauri::command]
pub fn setup_orchestration(project_path: String) -> Result<String, IpcError> {
    let launch = mcp_config::mcp_launch()
        .ok_or_else(|| IpcError::Other("Orbit MCP launch command not found".to_string()))?;

    mcp_config::write_orbit_mcp_configs(Path::new(&project_path), &launch)
        .map_err(IpcError::Other)?;

    Ok(format_launch(&launch))
}

/// Check if orchestration is available (MCP binary + IPC when app is running).
#[tauri::command]
pub fn check_orchestration() -> serde_json::Value {
    get_mcp_status()
}

/// MCP orchestration status for the UI (binary, IPC, unified executable mode).
#[tauri::command]
pub fn get_mcp_status() -> serde_json::Value {
    let launch = mcp_config::mcp_launch();
    let binary_available = launch.is_some();
    let binary_path = launch.as_ref().map(|l| l.command.clone());
    let ipc_listening = crate::mcp_transport::is_ipc_listening();
    let unified_binary = launch
        .as_ref()
        .is_some_and(|l| l.args.iter().any(|a| a == MCP_STDIO_ARG));

    serde_json::json!({
        "binaryAvailable": binary_available,
        "binaryPath": binary_path,
        "ipcListening": ipc_listening,
        "orchestrationReady": binary_available && ipc_listening,
        "unifiedBinary": unified_binary,
        "stdioArg": MCP_STDIO_ARG,
    })
}

fn format_launch(launch: &McpLaunch) -> String {
    if launch.args.is_empty() {
        launch.command.clone()
    } else {
        format!("{} {}", launch.command, launch.args.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestCase;

    #[test]
    fn should_report_mcp_status_fields() {
        let mut t = TestCase::new("should_report_mcp_status_fields");
        t.phase("Act");
        let status = get_mcp_status();
        t.phase("Assert");
        t.ok(
            "has binaryAvailable",
            status.get("binaryAvailable").is_some(),
        );
        t.ok("has ipcListening", status.get("ipcListening").is_some());
        t.ok(
            "has orchestrationReady",
            status.get("orchestrationReady").is_some(),
        );
        t.ok("has unifiedBinary", status.get("unifiedBinary").is_some());
    }
}
