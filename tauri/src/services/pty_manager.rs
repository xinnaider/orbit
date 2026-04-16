use std::collections::HashMap;
use std::io::{Read, Write};

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize as PtySizeCrate};
use tauri::{AppHandle, Emitter};

use crate::models::{PtySize, SessionId};

struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

pub struct PtyManager {
    sessions: HashMap<SessionId, PtySession>,
    app: AppHandle,
}

impl PtyManager {
    pub fn new(app: AppHandle) -> Self {
        PtyManager {
            sessions: HashMap::new(),
            app,
        }
    }

    pub fn create(
        &mut self,
        session_id: SessionId,
        command: &str,
        args: &[String],
        cwd: &str,
        env: Vec<(String, String)>,
        size: &PtySize,
    ) -> Result<u32, String> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySizeCrate {
                rows: size.rows,
                cols: size.cols,
                pixel_width: size.pixel_width,
                pixel_height: size.pixel_height,
            })
            .map_err(|e| format!("pty open failed: {e}"))?;

        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        cmd.cwd(cwd);

        for (k, v) in &env {
            cmd.env(k, v);
        }

        cmd.env(
            "TERM",
            if cfg!(windows) {
                "cygwin"
            } else {
                "xterm-256color"
            },
        );

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("pty spawn failed: {e}"))?;

        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("pty reader clone failed: {e}"))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("pty writer take failed: {e}"))?;

        let pid = child.process_id().unwrap_or(0);

        let app = self.app.clone();
        let sid = session_id;
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            let mut reader = reader;
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = app.emit(
                            "pty:output",
                            serde_json::json!({
                                "sessionId": sid,
                                "data": "",
                                "eof": true,
                            }),
                        );
                        break;
                    }
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).into_owned();
                        let _ = app.emit(
                            "pty:output",
                            serde_json::json!({
                                "sessionId": sid,
                                "data": data,
                                "eof": false,
                            }),
                        );
                    }
                    Err(_) => break,
                }
            }
        });

        self.sessions.insert(
            session_id,
            PtySession {
                master: pair.master,
                writer,
                _child: child,
            },
        );

        Ok(pid)
    }

    pub fn write(&mut self, session_id: SessionId, data: &str) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or("pty session not found")?;
        session
            .writer
            .write_all(data.as_bytes())
            .map_err(|e| format!("pty write failed: {e}"))?;
        session
            .writer
            .flush()
            .map_err(|e| format!("pty flush failed: {e}"))?;
        Ok(())
    }

    pub fn resize(&mut self, session_id: SessionId, size: &PtySize) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or("pty session not found")?;
        session
            .master
            .resize(PtySizeCrate {
                rows: size.rows,
                cols: size.cols,
                pixel_width: size.pixel_width,
                pixel_height: size.pixel_height,
            })
            .map_err(|e| format!("pty resize failed: {e}"))?;
        Ok(())
    }

    pub fn kill(&mut self, session_id: SessionId) -> Result<(), String> {
        if self.sessions.remove(&session_id).is_some() {
            Ok(())
        } else {
            Err("pty session not found".to_string())
        }
    }

    pub fn is_active(&self, session_id: SessionId) -> bool {
        self.sessions.contains_key(&session_id)
    }
}
