/// SSH utilities for Orbit: askpass helpers, host/user validation, connection testing,
/// and spawning remote commands via SSH tunnel.
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;

// ── AskpassGuard ─────────────────────────────────────────────────────────────

/// RAII guard that removes the temporary askpass directory on drop.
pub struct AskpassGuard {
    dir: PathBuf,
}

impl Drop for AskpassGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

// ── SpawnMode ─────────────────────────────────────────────────────────────────

/// Whether to run a process locally or through an SSH tunnel.
#[derive(Debug, Clone)]
pub enum SpawnMode {
    /// Run the process on the local machine.
    Local,
    /// Run the process on a remote host via SSH.
    Ssh {
        /// Hostname or IP address of the remote machine.
        host: String,
        /// SSH username.
        user: String,
    },
}

// ── askpass helpers ───────────────────────────────────────────────────────────

/// Creates a temporary directory containing an SSH_ASKPASS helper script and a
/// password file. Returns an [`AskpassGuard`] (deletes the dir on drop) and the
/// path to the script that SSH should invoke.
pub fn create_askpass(password: &str) -> std::io::Result<(AskpassGuard, PathBuf)> {
    let dir = std::env::temp_dir().join(format!("orbit-askpass-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;

    let pw_path = dir.join("pw");
    std::fs::write(&pw_path, password)?;

    #[cfg(target_os = "windows")]
    let script_path = {
        let script = dir.join("askpass.cmd");
        // `%~dp0` expands to the directory of the batch file (with trailing backslash).
        let content = "@type \"%~dp0pw\"\r\n".to_string();
        std::fs::write(&script, content)?;
        script
    };

    #[cfg(not(target_os = "windows"))]
    let script_path = {
        use std::os::unix::fs::PermissionsExt;

        let script = dir.join("askpass.sh");
        // Use single-quoted path to avoid shell injection; the dir is ours.
        let pw_str = pw_path.to_string_lossy();
        let content = format!("#!/bin/sh\ncat '{}'\n", pw_str);
        std::fs::write(&script, &content)?;

        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700))?;
        std::fs::set_permissions(&pw_path, std::fs::Permissions::from_mode(0o600))?;

        script
    };

    Ok((AskpassGuard { dir }, script_path))
}

/// Attaches SSH_ASKPASS environment variables to `cmd` so that SSH uses the
/// provided password without prompting. Returns the [`AskpassGuard`] — the
/// caller must keep it alive for as long as `cmd` is running.
pub fn apply_askpass(cmd: &mut Command, password: &str) -> std::io::Result<AskpassGuard> {
    let (guard, script_path) = create_askpass(password)?;

    cmd.env("SSH_ASKPASS", &script_path);
    cmd.env("SSH_ASKPASS_REQUIRE", "force");

    // SSH_ASKPASS is only honoured when DISPLAY is set on Unix.
    #[cfg(not(target_os = "windows"))]
    if std::env::var_os("DISPLAY").is_none() {
        cmd.env("DISPLAY", ":0");
    }

    Ok(guard)
}

// ── validation ────────────────────────────────────────────────────────────────

/// Returns `true` when `host` contains only characters safe for an SSH
/// hostname: alphanumerics, `.`, `-`, `:`, `[`, `]`. An empty string is
/// rejected.
pub fn validate_ssh_host(host: &str) -> bool {
    if host.is_empty() {
        return false;
    }
    host.chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'))
}

/// Returns `true` when `user` contains only characters safe for an SSH
/// username: alphanumerics, `-`, `_`, `.`. An empty string is rejected.
pub fn validate_ssh_user(user: &str) -> bool {
    if user.is_empty() {
        return false;
    }
    user.chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

// ── POSIX shell escaping ──────────────────────────────────────────────────────

/// Wraps `s` in POSIX single quotes so it can be safely embedded in a shell
/// command. Embedded single quotes are escaped using the `'\''` idiom.
pub fn posix_escape(s: &str) -> String {
    let escaped = s.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

// ── SshTestResult ─────────────────────────────────────────────────────────────

/// Result returned by [`test_ssh_connection`].
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTestResult {
    /// Whether the connection succeeded.
    pub ok: bool,
    /// Round-trip latency in milliseconds (only meaningful when `ok` is true).
    pub latency_ms: u64,
    /// Human-readable error message (empty string when `ok` is true).
    pub error: String,
}

// ── SSH options helpers ───────────────────────────────────────────────────────

/// Appends the baseline SSH `-o` options that are common to all SSH invocations
/// made by Orbit.
fn push_base_options(args: &mut Vec<String>, with_password: bool) {
    let opts: &[&str] = &[
        "ConnectTimeout=10",
        "StrictHostKeyChecking=accept-new",
        "ControlMaster=no",
        "ControlPath=none",
    ];
    for o in opts {
        args.push("-o".into());
        args.push(o.to_string());
    }
    if with_password {
        args.push("-o".into());
        args.push("PreferredAuthentications=keyboard-interactive,password".into());
        args.push("-o".into());
        args.push("NumberOfPasswordPrompts=1".into());
    }
}

/// Builds the `ssh` [`Command`] used for diagnostics (test connection).
fn build_test_command(host: &str, user: &str, with_password: bool) -> Command {
    let mut args: Vec<String> = vec![
        "-o".into(),
        "BatchMode=no".into(),
        "-o".into(),
        "LogLevel=ERROR".into(),
    ];
    push_base_options(&mut args, with_password);

    args.push(format!("{}@{}", user, host));
    args.push("echo __orbit_ok__".into());

    let mut cmd = Command::new("ssh");
    cmd.args(&args);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd
}

// ── test_ssh_connection ───────────────────────────────────────────────────────

/// Tests whether an SSH connection to `host` as `user` (optionally
/// authenticated with `password`) succeeds. Uses a 15-second hard timeout.
pub fn test_ssh_connection(host: &str, user: &str, password: Option<&str>) -> SshTestResult {
    let with_password = password.is_some();
    let mut cmd = build_test_command(host, user, with_password);

    // Attach askpass before spawning so the script file exists.
    let _guard: Option<AskpassGuard> = if let Some(pw) = password {
        match apply_askpass(&mut cmd, pw) {
            Ok(g) => Some(g),
            Err(e) => {
                return SshTestResult {
                    ok: false,
                    latency_ms: 0,
                    error: format!("failed to create askpass helper: {}", e),
                };
            }
        }
    } else {
        None
    };

    let start = Instant::now();

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return SshTestResult {
                ok: false,
                latency_ms: 0,
                error: format!("failed to spawn ssh: {}", e),
            };
        }
    };

    // Poll with 100 ms intervals up to 15 s.
    let timeout = Duration::from_secs(15);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                if !status.success() {
                    // Collect stderr for a useful error message.
                    let stderr = child
                        .stderr
                        .take()
                        .and_then(|mut r| {
                            use std::io::Read;
                            let mut s = String::new();
                            r.read_to_string(&mut s).ok().map(|_| s)
                        })
                        .unwrap_or_default();
                    return SshTestResult {
                        ok: false,
                        latency_ms,
                        error: if stderr.trim().is_empty() {
                            format!("ssh exited with status {}", status)
                        } else {
                            stderr.trim().to_string()
                        },
                    };
                }
                // Verify the sentinel is present in stdout.
                let stdout = child
                    .stdout
                    .take()
                    .and_then(|mut r| {
                        use std::io::Read;
                        let mut s = String::new();
                        r.read_to_string(&mut s).ok().map(|_| s)
                    })
                    .unwrap_or_default();
                let ok = stdout.contains("__orbit_ok__");
                return SshTestResult {
                    ok,
                    latency_ms,
                    error: if ok {
                        String::new()
                    } else {
                        "sentinel not found in ssh output".into()
                    },
                };
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return SshTestResult {
                        ok: false,
                        latency_ms: timeout.as_millis() as u64,
                        error: "ssh connection timed out (15 s)".into(),
                    };
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return SshTestResult {
                    ok: false,
                    latency_ms: start.elapsed().as_millis() as u64,
                    error: format!("error polling ssh process: {}", e),
                };
            }
        }
    }
}

// ── spawn_via_ssh ─────────────────────────────────────────────────────────────

/// Spawns `remote_script` on `host` as `user` through an SSH tunnel. Returns
/// the [`Child`] process handle together with the optional [`AskpassGuard`]
/// (which must be kept alive until the child exits).
///
/// `remote_script` should be a **plain** shell command with values already
/// individually escaped via [`posix_escape`]. This function wraps it in a
/// single `posix_escape` layer for `bash -lc '<script>'` so the remote login
/// profile is loaded.
pub fn spawn_via_ssh(
    host: &str,
    user: &str,
    password: Option<&str>,
    remote_script: &str,
) -> std::io::Result<(Child, Option<AskpassGuard>)> {
    let with_password = password.is_some();

    let mut args: Vec<String> = Vec::new();

    // BatchMode: yes when using key auth, no when using password.
    args.push("-o".into());
    args.push(if with_password {
        "BatchMode=no".into()
    } else {
        "BatchMode=yes".into()
    });

    push_base_options(&mut args, with_password);

    args.push(format!("{}@{}", user, host));

    // Wrap in bash -lc so the remote login profile is loaded (PATH includes
    // ~/.local/bin, nvm, npm globals, etc.). Without -l, user-installed CLIs
    // like claude/codex won't be found.
    // Use double quotes for the outer layer — the script has no pre-escaped
    // single quotes (providers pass raw values, no posix_escape).
    // Escape only $ ` \ " inside the script to prevent remote shell expansion.
    let safe = remote_script
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`");
    args.push(format!("bash -lc \"{safe}\""));

    if cfg!(debug_assertions) {
        eprintln!(
            "[orbit:debug] spawn_via_ssh: ssh {}",
            args.iter()
                .map(|a| if a.contains(' ') { format!("'{a}'") } else { a.clone() })
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    let mut cmd = Command::new("ssh");
    cmd.args(&args);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let guard: Option<AskpassGuard> = if let Some(pw) = password {
        Some(apply_askpass(&mut cmd, pw)?)
    } else {
        None
    };

    let child = cmd.spawn()?;
    Ok((child, guard))
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
}
