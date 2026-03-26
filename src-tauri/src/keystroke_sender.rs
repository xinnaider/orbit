use std::path::PathBuf;
use std::process::Command;

/// Send keystrokes to a running Claude CLI process via the send-keys sidecar.
pub fn send_keys(pid: i32, text: &str) -> Result<(), String> {
    let helper = find_send_keys_binary()?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        Command::new(&helper)
            .args([&pid.to_string(), text])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| format!("Failed to spawn send-keys at {}: {}", helper.display(), e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new(&helper)
            .args([&pid.to_string(), text])
            .spawn()
            .map_err(|e| format!("Failed to spawn send-keys at {}: {}", helper.display(), e))?;
    }

    Ok(())
}

fn find_send_keys_binary() -> Result<PathBuf, String> {
    let exe_name = format!("send-keys{}", if cfg!(windows) { ".exe" } else { "" });

    // Check next to the main executable (works in both dev and production)
    // In dev: target/debug/agent-dashboard-v2.exe → target/debug/send-keys.exe
    // In prod: install_dir/agent-dashboard-v2.exe → install_dir/send-keys.exe
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(&exe_name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    Err(format!(
        "send-keys binary not found. Build it with: cargo build --bin send-keys"
    ))
}
