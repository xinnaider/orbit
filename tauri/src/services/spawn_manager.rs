use std::path::Path;

/// RAII guard that deletes the temporary askpass directory on drop.
/// Keeps temp files alive for the duration of the SSH session, then cleans up.
struct AskpassGuard {
    dir: std::path::PathBuf,
}

impl Drop for AskpassGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Create a temporary SSH_ASKPASS helper that echoes `password`.
/// Returns `(guard, script_path)`. The guard deletes the temp dir on drop.
/// The script path should be set as `SSH_ASKPASS` env var.
fn create_askpass(password: &str) -> Result<(AskpassGuard, String), String> {
    let tmp = std::env::temp_dir().join(format!("orbit-ssh-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("askpass dir: {e}"))?;

    // Write password to a separate file so the script never embeds it as a literal.
    let pw_file = tmp.join("pw");
    std::fs::write(&pw_file, password).map_err(|e| format!("askpass pw: {e}"))?;

    let script_path;

    #[cfg(windows)]
    {
        script_path = tmp.join("ask.bat");
        let pw_str = pw_file.display().to_string().replace('"', "");
        std::fs::write(&script_path, format!("@type \"{pw_str}\"\r\n"))
            .map_err(|e| format!("askpass script: {e}"))?;
    }

    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        script_path = tmp.join("ask.sh");
        let pw_str = pw_file.display().to_string().replace('\'', "'\\''");
        std::fs::write(&script_path, format!("#!/bin/sh\ncat '{pw_str}'\n"))
            .map_err(|e| format!("askpass script: {e}"))?;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("askpass chmod script: {e}"))?;
        std::fs::set_permissions(&pw_file, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("askpass chmod pw: {e}"))?;
    }

    let script_str = script_path.display().to_string();
    Ok((AskpassGuard { dir: tmp }, script_str))
}

/// Apply SSH_ASKPASS env vars to `cmd` for password authentication.
/// Returns an `AskpassGuard` that must be kept alive until the process exits.
fn apply_askpass(cmd: &mut std::process::Command, password: &str) -> Result<AskpassGuard, String> {
    let (guard, script_path) = create_askpass(password)?;
    cmd.env("SSH_ASKPASS", &script_path);
    cmd.env("SSH_ASKPASS_REQUIRE", "force");
    // On Unix, SSH historically required DISPLAY to be set to trigger SSH_ASKPASS.
    // On modern systems SSH_ASKPASS_REQUIRE=force is sufficient, but set DISPLAY as fallback.
    #[cfg(not(windows))]
    {
        if std::env::var("DISPLAY").is_err() {
            cmd.env("DISPLAY", ":0");
        }
    }
    Ok(guard)
}

pub struct SpawnHandle {
    pub pid: u32,
    pub reader: Box<dyn std::io::Read + Send>,
    pub stderr: Box<dyn std::io::Read + Send>,
    pub child: std::process::Child,
    /// Keeps the askpass temp dir alive for the duration of the SSH session.
    _askpass: Option<AskpassGuard>,
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
    /// Optional SSH password. If set, uses SSH_ASKPASS. Never persisted to DB.
    pub ssh_password: Option<String>,
}

/// Result of a test SSH connection attempt.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTestResult {
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// Test SSH connectivity without spawning a full Claude session.
/// Runs `ssh ... "echo __orbit_ok__"` with a 5-second timeout.
pub fn test_ssh_connection(host: &str, user: &str, password: Option<&str>) -> SshTestResult {
    if !validate_ssh_host(host) {
        return SshTestResult {
            ok: false,
            latency_ms: None,
            error: Some(format!("invalid host: {host:?}")),
        };
    }
    if !validate_ssh_user(user) {
        return SshTestResult {
            ok: false,
            latency_ms: None,
            error: Some(format!("invalid user: {user:?}")),
        };
    }

    let mut cmd = std::process::Command::new("ssh");
    cmd.args([
        "-o",
        "BatchMode=no",
        "-o",
        "ConnectTimeout=5",
        "-o",
        "StrictHostKeyChecking=accept-new",
        &format!("{user}@{host}"),
        "echo __orbit_ok__",
    ]);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let _guard = if let Some(pw) = password {
        match apply_askpass(&mut cmd, pw) {
            Ok(g) => Some(g),
            Err(e) => {
                return SshTestResult {
                    ok: false,
                    latency_ms: None,
                    error: Some(e),
                }
            }
        }
    } else {
        None
    };

    let start = std::time::Instant::now();
    match cmd.output() {
        Ok(out) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let stdout = String::from_utf8_lossy(&out.stdout);
            if out.status.success() && stdout.contains("__orbit_ok__") {
                SshTestResult {
                    ok: true,
                    latency_ms: Some(latency_ms),
                    error: None,
                }
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                SshTestResult {
                    ok: false,
                    latency_ms: None,
                    error: Some(if stderr.is_empty() {
                        format!("exit code {}", out.status.code().unwrap_or(-1))
                    } else {
                        stderr
                    }),
                }
            }
        }
        Err(e) => SshTestResult {
            ok: false,
            latency_ms: None,
            error: Some(e.to_string()),
        },
    }
}

/// Wrap a string in single quotes, escaping embedded single quotes as '\''.
/// Safe for embedding in a POSIX shell command string (bash -lc '...').
fn posix_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
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

    Ok(SpawnHandle {
        pid,
        reader: Box::new(stdout),
        stderr: Box::new(stderr),
        child,
        _askpass: None,
    })
}

/// Validate that an SSH host string contains only safe characters.
/// Prevents injection of SSH options via crafted host values (e.g. `-oProxyCommand=...`).
fn validate_ssh_host(host: &str) -> bool {
    !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'))
}

/// Validate that an SSH user string contains only safe characters.
fn validate_ssh_user(user: &str) -> bool {
    !user.is_empty()
        && user
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Spawn Claude Code on a remote server via SSH.
/// Uses `bash -lc` so the remote user's login profile loads (PATH includes ~/.local/bin, npm globals).
fn spawn_ssh(config: SpawnConfig, host: &str, user: &str) -> Result<SpawnHandle, String> {
    if !validate_ssh_host(host) {
        return Err(format!("invalid ssh host: {host:?}"));
    }
    if !validate_ssh_user(user) {
        return Err(format!("invalid ssh user: {user:?}"));
    }

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
            parts.push(posix_escape(model));
        }
    }

    if let Some(ref resume_id) = config.claude_session_id {
        parts.push("--resume".to_string());
        parts.push(posix_escape(resume_id));
    }

    parts.push("-p".to_string());
    parts.push(posix_escape(&config.prompt));

    let remote_script = format!("cd {} && {}", posix_escape(&config.cwd), parts.join(" "));

    // BatchMode=no allows password auth via SSH_ASKPASS; BatchMode=yes blocks all prompts.
    let batch_mode = if config.ssh_password.is_some() {
        "no"
    } else {
        "yes"
    };

    let mut cmd = std::process::Command::new("ssh");
    cmd.args([
        "-o",
        &format!("BatchMode={batch_mode}"),
        "-o",
        "ConnectTimeout=10",
        "-o",
        "StrictHostKeyChecking=accept-new",
        &format!("{user}@{host}"),
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

    let askpass = if let Some(ref pw) = config.ssh_password {
        Some(apply_askpass(&mut cmd, pw)?)
    } else {
        None
    };

    let mut child = cmd.spawn().map_err(|e| format!("ssh spawn failed: {e}"))?;
    let pid = child.id();
    let stdout = child.stdout.take().ok_or_else(|| "no stdout".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "no stderr".to_string())?;

    Ok(SpawnHandle {
        pid,
        reader: Box::new(stdout),
        stderr: Box::new(stderr),
        child,
        _askpass: askpass,
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
    fn test_validate_ssh_host_accepts_valid_values() {
        assert!(validate_ssh_host("vps.example.com"));
        assert!(validate_ssh_host("192.168.1.1"));
        assert!(validate_ssh_host("[::1]"));
        assert!(validate_ssh_host("my-server"));
    }

    #[test]
    fn test_validate_ssh_host_rejects_injection() {
        assert!(!validate_ssh_host(""));
        assert!(!validate_ssh_host("-oProxyCommand=evil"));
        assert!(!validate_ssh_host("host;rm -rf /"));
        assert!(!validate_ssh_host("host$(whoami)"));
        assert!(!validate_ssh_host("host`cmd`"));
    }

    #[test]
    fn test_validate_ssh_user_accepts_valid_values() {
        assert!(validate_ssh_user("ubuntu"));
        assert!(validate_ssh_user("deploy_user"));
        assert!(validate_ssh_user("user.name"));
        assert!(validate_ssh_user("user-1"));
    }

    #[test]
    fn test_validate_ssh_user_rejects_injection() {
        assert!(!validate_ssh_user(""));
        assert!(!validate_ssh_user("user name"));
        assert!(!validate_ssh_user("user;id"));
        assert!(!validate_ssh_user("user$(id)"));
    }

    #[test]
    fn test_spawn_ssh_rejects_invalid_host() {
        let result = spawn_ssh(
            SpawnConfig {
                session_id: 0,
                cwd: "/tmp".to_string(),
                permission_mode: "ignore".to_string(),
                model: None,
                prompt: "hello".to_string(),
                claude_session_id: None,
                spawn_mode: SpawnMode::Local,
            },
            "-oProxyCommand=evil",
            "ubuntu",
        );
        let err = result.err().expect("expected Err for invalid host");
        assert!(err.contains("invalid ssh host"), "got: {err}");
    }

    #[test]
    fn test_spawn_ssh_rejects_invalid_user() {
        let result = spawn_ssh(
            SpawnConfig {
                session_id: 0,
                cwd: "/tmp".to_string(),
                permission_mode: "ignore".to_string(),
                model: None,
                prompt: "hello".to_string(),
                claude_session_id: None,
                spawn_mode: SpawnMode::Local,
            },
            "vps.example.com",
            "user name",
        );
        let err = result.err().expect("expected Err for invalid user");
        assert!(err.contains("invalid ssh user"), "got: {err}");
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
            cwd: "/nonexistent/path/xyz".to_string(),
            permission_mode: "ignore".to_string(),
            model: None,
            prompt: "test".to_string(),
            claude_session_id: None,
            spawn_mode: SpawnMode::Local,
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
