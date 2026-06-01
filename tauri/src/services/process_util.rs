//! Cross-platform helpers to spawn subprocesses without flashing a console on Windows.

use std::process::Command;

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Apply `CREATE_NO_WINDOW` on Windows (no-op elsewhere).
pub fn apply_silent(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

fn is_windows_script_shim(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".cmd") || lower.ends_with(".bat")
}

/// Build a `Command` for an external program. On Windows, npm `.cmd` shims run via `cmd /C`
/// with `CREATE_NO_WINDOW` so a console does not flash on every spawn.
pub fn command_for_program(program: &str) -> Command {
    #[cfg(windows)]
    if is_windows_script_shim(program) {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(program);
        apply_silent(&mut cmd);
        return cmd;
    }

    let mut cmd = Command::new(program);
    apply_silent(&mut cmd);
    cmd
}

/// Prefer a real `.exe` over npm `.cmd` shims when `where` returns multiple paths.
pub fn pick_windows_cli_path(lines: &[&str]) -> Option<String> {
    if let Some(exe) = lines.iter().find(|l| l.to_lowercase().ends_with(".exe")) {
        return Some((*exe).to_string());
    }
    if let Some(cmd) = lines.iter().find(|l| is_windows_script_shim(l)) {
        return Some((*cmd).to_string());
    }
    lines.first().map(|l| (*l).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_prefer_exe_over_cmd_shim() {
        let lines = [
            r"C:\nvm4w\nodejs\claude.cmd",
            r"C:\Users\me\AppData\Roaming\npm\claude.exe",
        ];
        let picked = pick_windows_cli_path(&lines).expect("path");
        assert!(picked.ends_with("claude.exe"));
    }
}
