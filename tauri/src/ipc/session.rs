use std::sync::{Arc, RwLock};
use tauri::{AppHandle, State};

use crate::ipc::IpcError;
use crate::models::{JournalEntry, Session, SessionId};
use crate::providers::ProviderRegistry;
use crate::services::session_manager::SessionManager;
use crate::services::spawn_manager::find_claude;

pub struct SessionState(pub Arc<RwLock<SessionManager>>);
pub struct ProviderRegistryState(pub Arc<ProviderRegistry>);

impl SessionState {
    /// Acquire a write guard, recovering from a poisoned RwLock.
    pub fn write(&self) -> std::sync::RwLockWriteGuard<'_, SessionManager> {
        self.0.write().unwrap_or_else(|e| e.into_inner())
    }

    /// Acquire a read guard, recovering from a poisoned RwLock.
    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, SessionManager> {
        self.0.read().unwrap_or_else(|e| e.into_inner())
    }
}

/// Create a session: returns immediately after creating the DB record (status = "initializing").
/// The actual CLI process spawns in a background thread — non-blocking.
/// Frontend should listen to "session:running" (ready) or "session:error" (spawn failed).
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn create_session(
    project_path: String,
    prompt: String,
    model: Option<String>,
    permission_mode: Option<String>,
    session_name: Option<String>,
    use_worktree: Option<bool>,
    provider: Option<String>,
    api_key: Option<String>,
    ssh_host: Option<String>,
    ssh_user: Option<String>,
    ssh_password: Option<String>,
    state: State<SessionState>,
    registry: State<ProviderRegistryState>,
    app: AppHandle,
) -> Result<Session, IpcError> {
    let mode = permission_mode.unwrap_or_else(|| "ignore".to_string());

    let session = {
        let mut m = state.write();
        let s = m.init_session(
            &project_path,
            session_name.as_deref(),
            &mode,
            model.as_deref(),
            use_worktree.unwrap_or(false),
            provider.as_deref(),
            ssh_host.as_deref(),
            ssh_user.as_deref(),
            ssh_password,
        )?;
        // Set API key before spawn thread starts — avoids race condition
        if let Some(key) = api_key {
            m.set_api_key(s.id, key);
        }
        s
    };

    // Emit session:created immediately so frontend shows the session
    use tauri::Emitter;
    let _ = app.emit("session:created", &session);

    let manager = Arc::clone(&state.0);
    let reg = Arc::clone(&registry.0);
    let session_id = session.id;
    std::thread::spawn(move || {
        SessionManager::do_spawn(manager, app, session_id, prompt, &reg);
    });

    Ok(session)
}

#[tauri::command]
pub fn list_sessions(state: State<SessionState>) -> Vec<Session> {
    state.write().get_sessions()
}

#[tauri::command]
pub fn stop_session(
    session_id: SessionId,
    state: State<SessionState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    state.write().stop_session(session_id)?;
    use tauri::Emitter;
    let _ = app.emit(
        "session:stopped",
        serde_json::json!({ "sessionId": session_id }),
    );
    Ok(())
}

#[tauri::command]
pub fn send_session_message(
    session_id: SessionId,
    message: String,
    state: State<SessionState>,
    registry: State<ProviderRegistryState>,
    app: AppHandle,
) -> Result<(), IpcError> {
    let trimmed = message.trim();

    // Intercept /model — changes model for the next message
    if trimmed.eq_ignore_ascii_case("/model") || trimmed.to_lowercase().starts_with("/model ") {
        let arg = trimmed.get(7..).unwrap_or("").trim();
        if arg.is_empty() {
            return Err(IpcError::Other("Usage: /model <name>".to_string()));
        }
        state.write().update_session_model(session_id, arg)?;
        return Ok(());
    }

    // Intercept /effort — only for providers that support it
    if trimmed.eq_ignore_ascii_case("/effort") || trimmed.to_lowercase().starts_with("/effort ") {
        let provider_id = state
            .read()
            .get_session_provider(session_id)
            .unwrap_or_default();
        let supports = registry
            .0
            .resolve(&provider_id)
            .is_some_and(|p| p.supports_effort());
        if !supports {
            return Err(IpcError::Other(
                "/effort is not supported for this provider".to_string(),
            ));
        }
        let arg = trimmed.get(8..).unwrap_or("").trim();
        if arg.is_empty() {
            return Err(IpcError::Other(
                "Usage: /effort <level> (low, medium, high, max)".to_string(),
            ));
        }
        state.write().update_session_effort(session_id, arg)?;
        return Ok(());
    }

    Ok(SessionManager::send_message(
        Arc::clone(&state.0),
        app,
        session_id,
        message,
        Arc::clone(&registry.0),
    )?)
}

#[tauri::command]
pub fn get_session_journal(session_id: SessionId, state: State<SessionState>) -> Vec<JournalEntry> {
    state.write().get_journal(session_id)
}

/// Get a paginated slice of journal entries for a session.
/// `cursor` is the seq number to start from, `limit` is max entries to return.
/// `direction` is "forward" (newer) or "backward" (older).
#[tauri::command]
pub fn get_session_journal_page(
    session_id: SessionId,
    cursor: Option<u32>,
    limit: Option<usize>,
    direction: Option<String>,
    state: State<SessionState>,
) -> Vec<JournalEntry> {
    let entries = state.write().get_journal(session_id);
    let limit = limit.unwrap_or(100);
    let is_backward = direction.as_deref() == Some("backward");

    match cursor {
        Some(seq) => {
            if is_backward {
                entries
                    .into_iter()
                    .filter(|e| e.seq < seq)
                    .rev()
                    .take(limit)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect()
            } else {
                entries
                    .into_iter()
                    .filter(|e| e.seq > seq)
                    .take(limit)
                    .collect()
            }
        }
        None => {
            // No cursor: return latest entries
            let len = entries.len();
            if len > limit {
                entries[len - limit..].to_vec()
            } else {
                entries
            }
        }
    }
}

/// Diagnostic: check if claude CLI is available and return its path or an error message.
#[tauri::command]
pub fn check_claude() -> serde_json::Value {
    match find_claude() {
        Some(path) => serde_json::json!({ "found": true, "path": path }),
        None => {
            let path_var = std::env::var("PATH").unwrap_or_default();
            serde_json::json!({
                "found": false,
                "path": null,
                "searchedPath": path_var,
                "hint": "Install with: npm install -g @anthropic-ai/claude-code"
            })
        }
    }
}

/// Diagnostic: fast check of claude availability using regular process commands (no PTY).
/// Does NOT block the UI.
#[tauri::command]
pub fn diagnose_spawn() -> serde_json::Value {
    use std::process::Command;

    // 1. find_claude() — static path search
    let claude_path = find_claude();

    // 2. `where claude` (Windows) or `which claude` (Unix) — 2s timeout
    #[cfg(windows)]
    let where_out = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        Command::new("where")
            .arg("claude")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|e| format!("where failed: {e}"))
    };

    #[cfg(not(windows))]
    let where_out = Command::new("which")
        .arg("claude")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|e| format!("which failed: {e}"));

    // 3. `claude --version` using augmented PATH
    let aug_path = crate::services::spawn_manager::extended_path();

    #[cfg(windows)]
    let version_out = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        Command::new("cmd")
            .args(["/c", "claude", "--version"])
            .env("PATH", &aug_path)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
    }
    .map(|o| {
        let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
        if !stdout.is_empty() {
            stdout
        } else {
            stderr
        }
    })
    .unwrap_or_else(|e| format!("version check failed: {e}"));

    #[cfg(not(windows))]
    let version_out = Command::new("claude")
        .arg("--version")
        .env("PATH", &aug_path)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|e| format!("version check failed: {e}"));

    serde_json::json!({
        "claudeFound": claude_path.is_some() || !where_out.is_empty(),
        "claudePath": claude_path,
        "whereOutput": where_out,
        "versionOutput": version_out,
        "augmentedPath": aug_path.chars().take(400).collect::<String>(),
        "processPath": std::env::var("PATH").unwrap_or_default().chars().take(200).collect::<String>(),
    })
}

/// Rename a session.
#[tauri::command]
pub fn rename_session(
    session_id: SessionId,
    name: String,
    state: State<SessionState>,
) -> Result<(), IpcError> {
    state.write().rename_session(session_id, &name)?;
    Ok(())
}

/// Delete a session (removes from DB, stops if running).
#[tauri::command]
pub fn delete_session(session_id: SessionId, state: State<SessionState>) -> Result<(), IpcError> {
    state.write().delete_session(session_id)?;
    Ok(())
}

/// Respond to a pending ACP permission request (approve or deny).
#[tauri::command]
pub fn respond_permission(
    session_id: SessionId,
    allow: bool,
    state: State<SessionState>,
) -> Result<(), IpcError> {
    state.write().respond_permission(session_id, allow)?;
    Ok(())
}

/// Clear attention flag for a session (called when user focuses/clicks it).
#[tauri::command]
pub fn clear_attention(session_id: SessionId, state: State<SessionState>) -> Result<(), IpcError> {
    state.write().clear_attention(session_id)?;
    Ok(())
}

/// Update the model for a session. Takes effect on the next message.
#[tauri::command]
pub fn update_session_model(
    session_id: SessionId,
    model: String,
    state: State<SessionState>,
) -> Result<(), IpcError> {
    state.write().update_session_model(session_id, &model)?;
    Ok(())
}

/// Update the effort level for a session. Takes effect on the next message.
#[tauri::command]
pub fn update_session_effort(
    session_id: SessionId,
    effort: String,
    state: State<SessionState>,
) -> Result<(), IpcError> {
    state.write().update_session_effort(session_id, &effort)?;
    Ok(())
}

/// Set the API key for an OpenRouter session (in-memory only, never persisted).
#[tauri::command]
pub fn set_session_api_key(
    session_id: SessionId,
    api_key: String,
    state: State<SessionState>,
) -> Result<(), IpcError> {
    state.write().set_api_key(session_id, api_key);
    Ok(())
}

use crate::services::ssh;

/// Test SSH connectivity to a remote host without creating a session.
#[tauri::command]
pub fn test_ssh(host: String, user: String, password: Option<String>) -> ssh::SshTestResult {
    ssh::test_ssh_connection(&host, &user, password.as_deref())
}
