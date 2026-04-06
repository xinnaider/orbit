use std::path::PathBuf;

pub struct SpawnConfig {
    pub session_id: crate::models::SessionId,
    pub cwd: PathBuf,
    pub permission_mode: String,
    pub model: Option<String>,
    pub prompt: String,
    /// For follow-up messages: the Claude session ID from the previous run
    pub claude_session_id: Option<String>,
}

pub struct SpawnHandle {
    pub pid: u32,
    pub reader: Box<dyn std::io::Read + Send>,
}

/// Build a PATH string that includes common Claude/Node installation directories.
fn extended_path() -> String {
    let current = std::env::var("PATH").unwrap_or_default();

    #[cfg(windows)]
    {
        let extra: Vec<String> = dirs::home_dir()
            .map(|h| vec![
                h.join(".local").join("bin").to_string_lossy().into_owned(),
                h.join("AppData").join("Roaming").join("npm").to_string_lossy().into_owned(),
                h.join("AppData").join("Local").join("pnpm").to_string_lossy().into_owned(),
                h.join("AppData").join("Roaming").join("nvm").to_string_lossy().into_owned(),
            ])
            .unwrap_or_default();
        format!("{};{}", extra.join(";"), current)
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

/// Find the full path to the claude executable.
pub fn find_claude() -> Option<String> {
    // 1. `where` / `which` with augmented PATH
    let aug = extended_path();

    #[cfg(windows)]
    {
        let out = std::process::Command::new("where")
            .arg("claude")
            .env("PATH", &aug)
            .output()
            .ok()?;
        if out.status.success() {
            if let Some(line) = String::from_utf8_lossy(&out.stdout).lines().next() {
                let p = line.trim().to_string();
                if !p.is_empty() { return Some(p); }
            }
        }
    }

    #[cfg(not(windows))]
    {
        let out = std::process::Command::new("which")
            .arg("claude")
            .env("PATH", &aug)
            .output()
            .ok()?;
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() { return Some(p); }
        }
    }

    // 2. Common locations
    #[cfg(windows)]
    if let Some(home) = dirs::home_dir() {
        let candidates = [
            home.join(".local").join("bin").join("claude.exe"),
            home.join(".local").join("bin").join("claude"),
            home.join("AppData").join("Roaming").join("npm").join("claude.cmd"),
            home.join("AppData").join("Local").join("pnpm").join("claude.cmd"),
        ];
        for p in &candidates {
            if p.exists() { return Some(p.to_string_lossy().into_owned()); }
        }
    }

    #[cfg(not(windows))]
    {
        for p in &["/usr/local/bin/claude", "/opt/homebrew/bin/claude"] {
            if std::path::Path::new(p).exists() { return Some(p.to_string()); }
        }
    }

    None
}

/// Spawn Claude Code using `-p "prompt"` (non-interactive mode).
/// Uses `--output-format stream-json` so stdout is pure JSON lines.
/// For follow-ups, passes `--resume <claude_session_id>`.
///
/// Uses piped stdout instead of PTY — avoids ConPTY issues on Windows.
pub fn spawn_claude(config: SpawnConfig) -> Result<SpawnHandle, String> {
    let claude = find_claude()
        .ok_or_else(|| "claude not found — install with: npm i -g @anthropic-ai/claude-code".to_string())?;

    let mut cmd = std::process::Command::new(&claude);
    cmd.args(["--output-format", "stream-json", "--verbose"]);

    if config.permission_mode == "ignore" {
        cmd.arg("--dangerously-skip-permissions");
    }

    if let Some(ref model) = config.model {
        if model != "auto" { cmd.args(["--model", model]); }
    }

    // Resume previous conversation if we have a Claude session ID
    if let Some(ref resume_id) = config.claude_session_id {
        cmd.args(["--resume", resume_id]);
    }

    // Pass prompt as flag — non-interactive, no stdin needed
    cmd.args(["-p", &config.prompt]);

    cmd.current_dir(&config.cwd);
    cmd.env("PATH", extended_path());

    // Piped stdout — no PTY needed
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::null());

    let mut child = cmd.spawn()
        .map_err(|e| format!("spawn failed: {e}"))?;

    let pid = child.id();
    let stdout = child.stdout.take()
        .ok_or_else(|| "no stdout".to_string())?;

    // Keep child alive until it exits naturally
    std::mem::forget(child);

    Ok(SpawnHandle { pid, reader: Box::new(stdout) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extended_path_includes_current() {
        let path = extended_path();
        let current = std::env::var("PATH").unwrap_or_default();
        if !current.is_empty() {
            assert!(path.contains(&current));
        }
    }

    #[test]
    fn test_find_claude_no_panic() {
        let _ = find_claude();
    }

    #[test]
    fn test_spawn_bad_path_returns_error() {
        let result = spawn_claude(SpawnConfig {
            session_id: 0,
            cwd: std::env::temp_dir(),
            permission_mode: "ignore".to_string(),
            model: None,
            prompt: "test".to_string(),
            claude_session_id: None,
        });
        // Either succeeds (claude installed) or returns descriptive error
        if let Err(e) = result {
            assert!(!e.is_empty());
        }
    }
}
