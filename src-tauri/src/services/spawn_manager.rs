use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

pub struct SpawnConfig {
    pub session_id: crate::models::SessionId,
    pub cwd: PathBuf,
    pub permission_mode: String,   // "ignore" | "approve"
    pub model: Option<String>,
}

pub struct PtyHandle {
    pub pid: u32,
    pub writer: Box<dyn Write + Send>,
    pub reader: Box<dyn std::io::Read + Send>,
}

/// Spawn a Claude Code process via PTY.
/// Returns a PtyHandle with the process PID, a writer (for stdin), and a reader (for stdout).
///
/// The initial prompt is NOT sent here — caller writes it to PtyHandle.writer after spawn.
pub fn spawn_claude(config: SpawnConfig) -> Result<PtyHandle, String> {
    let pty_system = native_pty_system();

    let pair = pty_system.openpty(PtySize {
        rows: 50,
        cols: 220,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e| format!("openpty failed: {e}"))?;

    let mut cmd = CommandBuilder::new("claude");
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

    let child = pair.slave.spawn_command(cmd)
        .map_err(|e| format!("spawn failed: {e}"))?;

    // IMPORTANT: drop slave after spawn so reader gets EOF when process exits
    drop(pair.slave);

    let pid = child.process_id().unwrap_or(0);

    let writer = pair.master.take_writer()
        .map_err(|e| format!("take_writer failed: {e}"))?;

    let reader = pair.master.try_clone_reader()
        .map_err(|e| format!("clone_reader failed: {e}"))?;

    // Keep child alive by leaking it — it will be reaped when the process exits.
    // We track lifecycle via PTY EOF instead of explicit child management.
    std::mem::forget(child);

    Ok(PtyHandle { pid, writer, reader })
}
