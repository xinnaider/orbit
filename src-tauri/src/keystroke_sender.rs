use std::path::PathBuf;
use std::process::Command;

/// Send keystrokes to a running Claude CLI process via the send-keys sidecar.
pub fn send_keys(pid: i32, text: &str) {
    let helper = find_send_keys_binary();

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new(&helper)
            .args([&pid.to_string(), text])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new(&helper)
            .args([&pid.to_string(), text])
            .spawn();
    }
}

fn find_send_keys_binary() -> PathBuf {
    // In Tauri, sidecar binaries are next to the main executable
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("send-keys").with_extension(std::env::consts::EXE_EXTENSION)))
        .unwrap_or_else(|| PathBuf::from("send-keys"))
}
