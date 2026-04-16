use std::sync::{Arc, Mutex};

use tauri::State;

use super::IpcError;
use crate::models::{PtySize, SessionId};
use crate::services::pty_manager::PtyManager;

pub struct PtyManagerState(pub Arc<Mutex<PtyManager>>);

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn pty_create(
    state: State<'_, PtyManagerState>,
    session_id: SessionId,
    command: String,
    args: Vec<String>,
    cwd: String,
    env: Vec<(String, String)>,
    rows: u16,
    cols: u16,
) -> Result<u32, IpcError> {
    let mut mgr = state.0.lock().map_err(|e| format!("lock: {e}"))?;
    mgr.create(
        session_id,
        &command,
        &args,
        &cwd,
        env,
        &PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        },
    )
    .map_err(IpcError::from)
}

#[tauri::command]
pub fn pty_write(
    state: State<'_, PtyManagerState>,
    session_id: SessionId,
    data: String,
) -> Result<(), IpcError> {
    let mut mgr = state.0.lock().map_err(|e| format!("lock: {e}"))?;
    mgr.write(session_id, &data).map_err(IpcError::from)
}

#[tauri::command]
pub fn pty_resize(
    state: State<'_, PtyManagerState>,
    session_id: SessionId,
    rows: u16,
    cols: u16,
) -> Result<(), IpcError> {
    let mut mgr = state.0.lock().map_err(|e| format!("lock: {e}"))?;
    mgr.resize(
        session_id,
        &PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        },
    )
    .map_err(IpcError::from)
}

#[tauri::command]
pub fn pty_kill(
    state: State<'_, PtyManagerState>,
    session_id: SessionId,
) -> Result<(), IpcError> {
    let mut mgr = state.0.lock().map_err(|e| format!("lock: {e}"))?;
    mgr.kill(session_id).map_err(IpcError::from)
}
