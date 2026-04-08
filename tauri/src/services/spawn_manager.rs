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
    pub stderr: Box<dyn std::io::Read + Send>,
}

/// Build a PATH string that includes common Claude/Node installation directories.
pub(crate) fn extended_path() -> String {
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
                let mut paths = vec![
                    h.join(".local").join("bin").to_string_lossy().into_owned(),
                    h.join(".npm-global")
                        .join("bin")
                        .to_string_lossy()
                        .into_owned(),
                    "/usr/local/bin".to_string(),
                ];
                // nvm: add every installed node version's bin dir
                let nvm_root = h.join(".nvm").join("versions").join("node");
                if let Ok(entries) = std::fs::read_dir(&nvm_root) {
                    for entry in entries.flatten() {
                        let bin = entry.path().join("bin");
                        if bin.exists() {
                            paths.push(bin.to_string_lossy().into_owned());
                        }
                    }
                }
                paths
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

    // 2. Common static locations
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
        // Static fallbacks for Linux and macOS
        let mut candidates = vec![
            "/usr/local/bin/claude".to_string(),
            "/opt/homebrew/bin/claude".to_string(), // macOS Homebrew
        ];
        // Add ~/.local/bin/claude (common npm global prefix on Linux)
        if let Some(home) = dirs::home_dir() {
            candidates.push(
                home.join(".local")
                    .join("bin")
                    .join("claude")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        for p in &candidates {
            if std::path::Path::new(p).exists() {
                return Some(p.to_string());
            }
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

    cmd.current_dir(&config.cwd);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestCase;

    #[test]
    fn should_include_current_path_in_extended_path() {
        let mut t = TestCase::new("should_include_current_path_in_extended_path");
        t.phase("Act");
        let path = extended_path();
        let current = std::env::var("PATH").unwrap_or_default();
        t.phase("Assert");
        if !current.is_empty() {
            t.ok(
                "current PATH is contained in extended PATH",
                path.contains(&current),
            );
        } else {
            t.ok(
                "extended PATH is non-empty even without current PATH",
                !path.is_empty(),
            );
        }
    }

    #[test]
    fn should_include_local_bin_in_extended_path() {
        let mut t = TestCase::new("should_include_local_bin_in_extended_path");
        t.phase("Act");
        let path = extended_path();
        t.phase("Assert");
        if let Some(home) = dirs::home_dir() {
            let local_bin = home
                .join(".local")
                .join("bin")
                .to_string_lossy()
                .into_owned();
            t.ok(
                "~/.local/bin is in extended PATH",
                path.contains(&local_bin),
            );
        } else {
            t.ok("no home dir — skip", true);
        }
    }

    #[test]
    fn should_not_panic_when_find_claude_is_called() {
        let mut t = TestCase::new("should_not_panic_when_find_claude_is_called");
        t.phase("Act");
        let _result = find_claude(); // may return None if not installed — that's fine
        t.phase("Assert");
        t.ok("find_claude completes without panic", true);
    }

    #[test]
    fn should_return_err_with_descriptive_message_when_claude_not_found() {
        let mut t =
            TestCase::new("should_return_err_with_descriptive_message_when_claude_not_found");
        t.phase("Act");
        // Use a cwd that doesn't exist to guarantee spawn failure regardless of claude install
        let result = spawn_claude(SpawnConfig {
            session_id: 0,
            cwd: std::path::PathBuf::from("/nonexistent/path/xyz"),
            permission_mode: "ignore".to_string(),
            model: None,
            prompt: "test".to_string(),
            claude_session_id: None,
        });
        t.phase("Assert");
        // Either claude is installed and spawn fails on bad cwd (Err), or claude is not installed
        // (Err). Either way, we must get an Err with a non-empty message.
        if let Err(ref msg) = result {
            t.ok("error message is non-empty", !msg.is_empty());
        } else {
            // Claude installed AND somehow accepted /nonexistent path — log and skip
            t.ok(
                "spawn succeeded (claude installed, cwd error deferred)",
                true,
            );
        }
    }

    #[test]
    #[cfg(not(windows))]
    fn should_include_nvm_bin_dirs_in_extended_path_when_present() {
        let mut t = TestCase::new("should_include_nvm_bin_dirs_in_extended_path_when_present");
        t.phase("Act");
        let path = extended_path();
        t.phase("Assert");
        if let Some(home) = dirs::home_dir() {
            let nvm_root = home.join(".nvm").join("versions").join("node");
            if nvm_root.exists() {
                let has_nvm = std::fs::read_dir(&nvm_root)
                    .map(|entries| {
                        entries.flatten().any(|e| {
                            let bin = e.path().join("bin").to_string_lossy().into_owned();
                            path.contains(&bin)
                        })
                    })
                    .unwrap_or(false);
                t.ok("nvm bin dirs present in extended PATH", has_nvm);
            } else {
                t.ok("nvm not installed — skip", true);
            }
        } else {
            t.ok("no home dir — skip", true);
        }
    }
}
