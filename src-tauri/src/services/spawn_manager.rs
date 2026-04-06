use std::io::{Write};
use std::path::PathBuf;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

pub struct SpawnConfig {
    pub session_id: crate::models::SessionId,
    pub cwd: PathBuf,
    pub permission_mode: String,
    pub model: Option<String>,
}

pub struct PtyHandle {
    pub pid: u32,
    pub writer: Box<dyn Write + Send>,
    pub reader: Box<dyn std::io::Read + Send>,
}

/// Build a PATH string that includes common Claude/Node installation directories.
/// On Windows, npm/pnpm global bins are NOT in the default Tauri process PATH.
fn extended_path() -> String {
    let current = std::env::var("PATH").unwrap_or_default();

    #[cfg(windows)]
    {
        let extra: Vec<String> = dirs::home_dir()
            .map(|h| vec![
                h.join("AppData").join("Roaming").join("npm").to_string_lossy().into_owned(),
                h.join("AppData").join("Local").join("pnpm").to_string_lossy().into_owned(),
                h.join(".local").join("bin").to_string_lossy().into_owned(),
                // nvm for Windows
                h.join("AppData").join("Roaming").join("nvm").to_string_lossy().into_owned(),
            ])
            .unwrap_or_default();

        let sep = ";";
        format!("{}{}{}", extra.join(sep), sep, current)
    }

    #[cfg(not(windows))]
    {
        let extra: Vec<String> = dirs::home_dir()
            .map(|h| vec![
                h.join(".local").join("bin").to_string_lossy().into_owned(),
                h.join(".npm-global").join("bin").to_string_lossy().into_owned(),
                "/usr/local/bin".to_string(),
            ])
            .unwrap_or_default();

        format!("{}:{}", extra.join(":"), current)
    }
}

/// Find the full path to the claude executable, trying common locations.
pub fn find_claude() -> Option<String> {
    // 1. Check PATH via `where` (Windows) or `which` (Unix)
    #[cfg(windows)]
    let finder = std::process::Command::new("where").arg("claude").output();
    #[cfg(not(windows))]
    let finder = std::process::Command::new("which").arg("claude").output();

    if let Ok(out) = finder {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            if path.is_some() { return path; }
        }
    }

    // 2. Common Windows locations (check .exe first, then .cmd)
    #[cfg(windows)]
    if let Some(home) = dirs::home_dir() {
        let candidates = [
            home.join(".local").join("bin").join("claude.exe"),
            home.join(".local").join("bin").join("claude"),
            home.join("AppData").join("Roaming").join("npm").join("claude.cmd"),
            home.join("AppData").join("Local").join("pnpm").join("claude.cmd"),
            home.join("AppData").join("Roaming").join("npm").join("claude"),
        ];
        for p in &candidates {
            if p.exists() { return Some(p.to_string_lossy().into_owned()); }
        }
    }

    // 3. Common Unix/macOS locations
    #[cfg(not(windows))]
    {
        let candidates = [
            "/usr/local/bin/claude",
            "/opt/homebrew/bin/claude",
        ];
        for p in &candidates {
            if std::path::Path::new(p).exists() { return Some(p.to_string()); }
        }
    }

    None
}

/// Build the CommandBuilder for claude.
/// - .exe files are spawned directly (no wrapper needed)
/// - .cmd/.bat files require `cmd /c` on Windows (CreateProcess doesn't expand .cmd)
fn claude_command() -> Result<CommandBuilder, String> {
    let path = find_claude();

    #[cfg(windows)]
    {
        if let Some(ref p) = path {
            let lower = p.to_lowercase();
            if lower.ends_with(".exe") || lower.ends_with(".com") {
                // Direct spawn — no cmd /c needed
                return Ok(CommandBuilder::new(p));
            }
            if lower.ends_with(".cmd") || lower.ends_with(".bat") {
                let mut cmd = CommandBuilder::new("cmd");
                cmd.args(["/c", p.as_str()]);
                return Ok(cmd);
            }
        }
        // Fallback: use cmd /c claude (relies on PATH)
        let mut cmd = CommandBuilder::new("cmd");
        cmd.args(["/c", "claude"]);
        Ok(cmd)
    }

    #[cfg(not(windows))]
    {
        Ok(CommandBuilder::new(path.as_deref().unwrap_or("claude")))
    }
}


/// Spawn a Claude Code process via PTY.
/// Returns a PtyHandle with the process PID, a writer (for stdin), and a reader (for stdout).
/// The initial prompt is NOT sent here — caller writes it via PtyHandle.writer after spawn.
pub fn spawn_claude(config: SpawnConfig) -> Result<PtyHandle, String> {
    let pty_system = native_pty_system();

    let pair = pty_system.openpty(PtySize {
        rows: 50,
        cols: 220,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e| format!("openpty failed: {e}"))?;

    let mut cmd = claude_command()?;

    cmd.args(["--output-format", "stream-json", "--verbose"]);

    if config.permission_mode == "ignore" {
        cmd.args(["--dangerously-skip-permissions"]);
    }

    if let Some(ref model) = config.model {
        if model != "auto" {
            cmd.args(["--model", model]);
        }
    }

    cmd.cwd(&config.cwd);

    // Inject augmented PATH so npm/pnpm binaries are found
    cmd.env("PATH", extended_path());

    let child = pair.slave.spawn_command(cmd)
        .map_err(|e| format!("spawn failed — claude not found or failed to start: {e}\nTip: make sure 'claude' is installed (npm i -g @anthropic-ai/claude-code)"))?;

    // Must drop slave after spawn so reader gets EOF when process exits
    drop(pair.slave);

    let pid = child.process_id().unwrap_or(0);

    let writer = pair.master.take_writer()
        .map_err(|e| format!("take_writer failed: {e}"))?;

    let reader = pair.master.try_clone_reader()
        .map_err(|e| format!("clone_reader failed: {e}"))?;

    std::mem::forget(child);

    Ok(PtyHandle { pid, writer, reader })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extended_path_includes_current_path() {
        let path = extended_path();
        // Should contain the current PATH somewhere
        let current = std::env::var("PATH").unwrap_or_default();
        if !current.is_empty() {
            assert!(path.contains(&current), "extended PATH should include current PATH");
        }
    }

    #[test]
    fn test_extended_path_not_empty() {
        let path = extended_path();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_find_claude_returns_option() {
        // Just verifies it doesn't panic — result depends on system
        let _result = find_claude();
        // If claude is installed, it should return Some
        // If not, None is valid
    }

    #[cfg(windows)]
    #[test]
    fn test_claude_command_uses_cmd_on_windows() {
        let cmd = claude_command();
        // On Windows, we wrap with cmd /c
        // We can't directly inspect CommandBuilder, but we can verify
        // spawn_claude returns an appropriate error when claude isn't found
        // rather than a cryptic OS error
        let result = spawn_claude(SpawnConfig {
            session_id: 0,
            cwd: std::env::temp_dir(),
            permission_mode: "ignore".to_string(),
            model: None,
        });
        // Either succeeds (claude found) or fails with a clear message
        if let Err(e) = result {
            assert!(
                e.contains("spawn failed") || e.contains("openpty"),
                "Error should be descriptive, got: {e}"
            );
        }
    }
}
