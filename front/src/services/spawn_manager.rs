use std::path::Path;

pub struct SpawnHandle {
    pub pid: u32,
    pub reader: Box<dyn std::io::Read + Send>,
    pub stderr: Box<dyn std::io::Read + Send>,
}

/// How to spawn Claude: locally or via SSH tunnel.
#[derive(Debug, Clone)]
pub enum SpawnMode {
    Local,
    Ssh { host: String, user: String },
}

pub struct SpawnConfig {
    pub session_id: crate::models::SessionId,
    pub cwd: String, // String not PathBuf — remote Linux paths can't be PathBuf on Windows
    pub permission_mode: String,
    pub model: Option<String>,
    pub prompt: String,
    /// For follow-up messages: the Claude session ID from the previous run
    pub claude_session_id: Option<String>,
    pub spawn_mode: SpawnMode,
}

/// Wrap a string in single quotes, escaping embedded single quotes as '\''.
/// Safe for embedding in a POSIX shell command string (bash -lc '...').
fn posix_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Build a PATH string that includes common Claude/Node installation directories.
fn extended_path() -> String {
    let current = std::env::var("PATH").unwrap_or_default();

    #[cfg(windows)]
    {
        let extra: Vec<String> = dirs::home_dir()
            .map(|h| {
                vec![
                    h.join(".local").join("bin").to_string_lossy().into_owned(),
                    h.join("AppData")
                        .join("Roaming")
                        .join("npm")
                        .to_string_lossy()
                        .into_owned(),
                    h.join("AppData")
                        .join("Local")
                        .join("pnpm")
                        .to_string_lossy()
                        .into_owned(),
                    h.join("AppData")
                        .join("Roaming")
                        .join("nvm")
                        .to_string_lossy()
                        .into_owned(),
                ]
            })
            .unwrap_or_default();
        format!("{};{}", extra.join(";"), current)
    }

    #[cfg(not(windows))]
    {
        let extra: Vec<String> = dirs::home_dir()
            .map(|h| {
                vec![
                    h.join(".local").join("bin").to_string_lossy().into_owned(),
                    h.join(".npm-global")
                        .join("bin")
                        .to_string_lossy()
                        .into_owned(),
                    "/usr/local/bin".to_string(),
                ]
            })
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
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let out = std::process::Command::new("where")
            .arg("claude")
            .env("PATH", &aug)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        if out.status.success() {
            if let Some(line) = String::from_utf8_lossy(&out.stdout).lines().next() {
                let p = line.trim().to_string();
                if !p.is_empty() {
                    return Some(p);
                }
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
            if !p.is_empty() {
                return Some(p);
            }
        }
    }

    // 2. Common locations
    #[cfg(windows)]
    if let Some(home) = dirs::home_dir() {
        let candidates = [
            home.join(".local").join("bin").join("claude.exe"),
            home.join(".local").join("bin").join("claude"),
            home.join("AppData")
                .join("Roaming")
                .join("npm")
                .join("claude.cmd"),
            home.join("AppData")
                .join("Local")
                .join("pnpm")
                .join("claude.cmd"),
        ];
        for p in &candidates {
            if p.exists() {
                return Some(p.to_string_lossy().into_owned());
            }
        }
    }

    #[cfg(not(windows))]
    {
        for p in &["/usr/local/bin/claude", "/opt/homebrew/bin/claude"] {
            if std::path::Path::new(p).exists() {
                return Some(p.to_string());
            }
        }
    }

    None
}

/// Dispatch to local or SSH spawn based on SpawnMode.
pub fn spawn_claude(config: SpawnConfig) -> Result<SpawnHandle, String> {
    match config.spawn_mode.clone() {
        SpawnMode::Local => spawn_local(config),
        SpawnMode::Ssh { host, user } => spawn_ssh(config, &host, &user),
    }
}

/// Spawn Claude Code locally.
fn spawn_local(config: SpawnConfig) -> Result<SpawnHandle, String> {
    let claude = find_claude().ok_or_else(|| {
        "claude not found — install with: npm i -g @anthropic-ai/claude-code".to_string()
    })?;

    let mut cmd = std::process::Command::new(&claude);
    cmd.args([
        "--output-format",
        "stream-json",
        "--verbose",
        "--dangerously-skip-permissions",
    ]);

    if let Some(ref model) = config.model {
        if model != "auto" {
            cmd.args(["--model", model]);
        }
    }

    if let Some(ref resume_id) = config.claude_session_id {
        cmd.args(["--resume", resume_id]);
    }

    // Pass prompt as flag — non-interactive, no stdin needed
    cmd.args(["-p", &config.prompt]);

    cmd.current_dir(Path::new(&config.cwd));
    cmd.env("PATH", extended_path());

    // Piped stdout and stderr — no PTY needed
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // Windows: suppress the console window that flashes on every spawn
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;

    let pid = child.id();
    let stdout = child.stdout.take().ok_or_else(|| "no stdout".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "no stderr".to_string())?;

    // Keep child alive until it exits naturally
    std::mem::forget(child);

    Ok(SpawnHandle {
        pid,
        reader: Box::new(stdout),
        stderr: Box::new(stderr),
    })
}

/// Spawn Claude Code on a remote server via SSH.
/// Uses `bash -lc` so the remote user's login profile loads (PATH includes ~/.local/bin, npm globals).
fn spawn_ssh(config: SpawnConfig, host: &str, user: &str) -> Result<SpawnHandle, String> {
    let mut parts = vec![
        "claude".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--verbose".to_string(),
        "--dangerously-skip-permissions".to_string(),
    ];

    if let Some(ref model) = config.model {
        if model != "auto" {
            parts.push("--model".to_string());
            parts.push(model.clone());
        }
    }

    if let Some(ref resume_id) = config.claude_session_id {
        parts.push("--resume".to_string());
        parts.push(resume_id.clone());
    }

    parts.push("-p".to_string());
    parts.push(posix_escape(&config.prompt));

    let remote_script = format!("cd {} && {}", posix_escape(&config.cwd), parts.join(" "));

    let mut cmd = std::process::Command::new("ssh");
    cmd.args([
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=10",
        "-o",
        "StrictHostKeyChecking=accept-new",
        &format!("{}@{}", user, host),
    ]);
    cmd.arg(format!("bash -lc {}", posix_escape(&remote_script)));
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd.spawn().map_err(|e| format!("ssh spawn failed: {e}"))?;
    let pid = child.id();
    let stdout = child.stdout.take().ok_or_else(|| "no stdout".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "no stderr".to_string())?;
    std::mem::forget(child);

    Ok(SpawnHandle {
        pid,
        reader: Box::new(stdout),
        stderr: Box::new(stderr),
    })
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
    fn test_posix_escape_simple() {
        assert_eq!(posix_escape("hello world"), "'hello world'");
    }

    #[test]
    fn test_posix_escape_with_single_quote() {
        assert_eq!(posix_escape("it's a test"), "'it'\\''s a test'");
    }

    #[test]
    fn test_posix_escape_empty() {
        assert_eq!(posix_escape(""), "''");
    }

    #[test]
    fn test_posix_escape_dollar_not_expanded() {
        // Dollar sign inside single-quotes is not interpreted by shell
        assert_eq!(posix_escape("$HOME"), "'$HOME'");
    }

    #[test]
    fn test_posix_escape_newline_preserved() {
        assert_eq!(posix_escape("line1\nline2"), "'line1\nline2'");
    }

    #[test]
    fn test_spawn_bad_path_returns_error() {
        let result = spawn_claude(SpawnConfig {
            session_id: 0,
            cwd: std::env::temp_dir().to_string_lossy().into_owned(),
            permission_mode: "ignore".to_string(),
            model: None,
            prompt: "test".to_string(),
            claude_session_id: None,
            spawn_mode: SpawnMode::Local,
        });
        // Either succeeds (claude installed) or returns descriptive error
        if let Err(e) = result {
            assert!(!e.is_empty());
        }
    }

    #[test]
    fn test_spawn_local_config_compiles() {
        let config = SpawnConfig {
            session_id: 0,
            cwd: std::env::temp_dir().to_string_lossy().into_owned(),
            permission_mode: "ignore".to_string(),
            model: None,
            prompt: "test".to_string(),
            claude_session_id: None,
            spawn_mode: SpawnMode::Local,
        };
        let _ = spawn_claude(config);
    }
}
