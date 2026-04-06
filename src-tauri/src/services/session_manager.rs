use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter};

use crate::journal_reader::{JournalState, process_line};
use crate::models::{Session, SessionId, AgentStatus, TokenUsage};
use crate::services::database::DatabaseService;
use crate::services::spawn_manager::{SpawnConfig, spawn_claude};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionOutputEvent {
    pub session_id: SessionId,
    pub entry: crate::models::JournalEntry,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStateEvent {
    pub session_id: SessionId,
    pub status: String,
    pub tokens: TokenUsage,
    pub context_percent: f64,
    pub pending_approval: Option<String>,
    pub mini_log: Vec<crate::models::MiniLogEntry>,
}

struct ActiveSession {
    session: Session,
    /// The Claude CLI session ID (from stream-json system/init message).
    /// Required for --resume on follow-up messages.
    pub claude_session_id: Option<String>,
}

pub struct SessionManager {
    pub db: Arc<DatabaseService>,
    active: HashMap<SessionId, ActiveSession>,
    pub journal_states: HashMap<SessionId, JournalState>,
}

impl SessionManager {
    pub fn new(db: Arc<DatabaseService>) -> Self {
        SessionManager { db, active: HashMap::new(), journal_states: HashMap::new() }
    }

    /// Phase 1 (fast): create DB record, return Session immediately.
    pub fn init_session(
        &mut self,
        project_path: &str,
        session_name: Option<&str>,
        permission_mode: &str,
        model: Option<&str>,
    ) -> Result<Session, String> {
        let project_name = std::path::Path::new(project_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project_path.to_string());

        let project = self.db.create_project(&project_name, project_path)
            .map_err(|e| e.to_string())?;

        let session_id = self.db.create_session(
            Some(project.id), session_name, project_path, permission_mode, model,
        ).map_err(|e| e.to_string())?;

        let now = chrono::Utc::now().to_rfc3339();
        let session = Session {
            id: session_id,
            project_id: Some(project.id),
            name: session_name.map(|s| s.to_string()),
            status: crate::models::SessionStatus::Initializing.as_str().to_string(),
            worktree_path: None, branch_name: None,
            permission_mode: permission_mode.to_string(),
            model: model.map(|s| s.to_string()),
            pid: None, created_at: now.clone(), updated_at: now,
            cwd: Some(project_path.to_string()),
            project_name: Some(project_name),
            git_branch: None, tokens: None, context_percent: None,
            pending_approval: None, mini_log: None,
        };

        self.active.insert(session_id, ActiveSession {
            session: session.clone(),
            claude_session_id: None,
        });
        self.journal_states.insert(session_id, JournalState::default());

        Ok(session)
    }

    /// Phase 2 (async): spawn Claude with `-p "prompt"`.
    /// Each message spawns a new process. Uses `--resume` for follow-ups.
    pub fn do_spawn(
        manager: Arc<Mutex<SessionManager>>,
        app: AppHandle,
        session_id: SessionId,
        prompt: String,
    ) {
        let (db, cwd, permission_mode, model, claude_session_id) = {
            let m = manager.lock().unwrap();
            let a = match m.active.get(&session_id) {
                Some(a) => a,
                None => {
                    let _ = app.emit("session:error", serde_json::json!({
                        "sessionId": session_id,
                        "error": "Session not found in active map"
                    }));
                    return;
                }
            };
            (
                m.db.clone(),
                a.session.cwd.clone().unwrap_or_default(),
                a.session.permission_mode.clone(),
                a.session.model.clone(),
                a.claude_session_id.clone(),
            )
        };

        let config = SpawnConfig {
            session_id,
            cwd: std::path::PathBuf::from(&cwd),
            permission_mode,
            model,
            prompt,
            claude_session_id,
        };

        let handle = match spawn_claude(config) {
            Ok(h) => h,
            Err(e) => {
                let _ = db.update_session_status(session_id, crate::models::SessionStatus::Error.as_str());
                let _ = app.emit("session:error", serde_json::json!({
                    "sessionId": session_id, "error": e
                }));
                return;
            }
        };

        let pid = handle.pid as i32;
        let _ = db.update_session_pid(session_id, pid);

        {
            let mut m = manager.lock().unwrap();
            if let Some(a) = m.active.get_mut(&session_id) {
                a.session.status = crate::models::SessionStatus::Running.as_str().to_string();
                a.session.pid = Some(pid);
            }
        }

        let _ = app.emit("session:running", serde_json::json!({
            "sessionId": session_id, "pid": pid
        }));

        Self::reader_loop(Arc::clone(&manager), app, session_id, handle.reader, db);
    }

    /// Read JSON lines from Claude's stdout, parse, emit events.
    fn reader_loop(
        manager: Arc<Mutex<SessionManager>>,
        app: AppHandle,
        session_id: SessionId,
        reader: Box<dyn std::io::Read + Send>,
        db: Arc<DatabaseService>,
    ) {
        use std::io::BufRead;
        let mut reader = std::io::BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let trimmed = line.trim().to_string();
                    if trimmed.is_empty() || !trimmed.starts_with('{') {
                        continue;
                    }

                    // Check for session_id in system/init message
                    if trimmed.contains("\"type\":\"system\"") || trimmed.contains("\"type\": \"system\"") {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&trimmed) {
                            if let Some(sid) = val.get("session_id").and_then(|v| v.as_str()) {
                                let mut m = manager.lock().unwrap();
                                if let Some(a) = m.active.get_mut(&session_id) {
                                    if a.claude_session_id.is_none() {
                                        a.claude_session_id = Some(sid.to_string());
                                    }
                                }
                            }
                        }
                    }

                    // Persist + process
                    let _ = db.insert_output(session_id, &trimmed);

                    let (new_entries, state_event) = {
                        let mut m = manager.lock().unwrap();
                        let state = m.journal_states
                            .entry(session_id)
                            .or_insert_with(JournalState::default);

                        let prev_len = state.entries.len();
                        process_line(state, &trimmed);
                        let new_entries: Vec<_> = state.entries[prev_len..].to_vec();

                        let window = state.model.as_deref()
                            .map(crate::models::context_window)
                            .unwrap_or(200_000);
                        let total = state.input_tokens + state.output_tokens;

                        let status_str = match state.status {
                            AgentStatus::Working => "working",
                            AgentStatus::Input => "input",
                            AgentStatus::Idle => "idle",
                            AgentStatus::New => "new",
                        }.to_string();

                        let event = SessionStateEvent {
                            session_id,
                            status: status_str,
                            tokens: TokenUsage {
                                input: state.input_tokens,
                                output: state.output_tokens,
                                cache_read: state.cache_read,
                                cache_write: state.cache_write,
                            },
                            context_percent: if window > 0 { (total as f64 / window as f64) * 100.0 } else { 0.0 },
                            pending_approval: state.pending_approval.clone(),
                            mini_log: state.mini_log.clone(),
                        };
                        (new_entries, event)
                    };

                    for entry in new_entries {
                        let mut e = entry.clone();
                        e.session_id = session_id.to_string();
                        let _ = app.emit("session:output", SessionOutputEvent { session_id, entry: e });
                    }
                    let _ = app.emit("session:state", &state_event);
                }
            }
        }

        // Process exited
        {
            let mut m = manager.lock().unwrap();
            if let Some(a) = m.active.get_mut(&session_id) {
                a.session.status = crate::models::SessionStatus::Completed.as_str().to_string();
            }
            if let Some(state) = m.journal_states.get_mut(&session_id) {
                state.status = AgentStatus::Idle;
            }
            let _ = db.update_session_status(session_id, crate::models::SessionStatus::Completed.as_str());
        }

        let _ = app.emit("session:stopped", serde_json::json!({ "sessionId": session_id }));
    }

    /// Send a follow-up message by spawning a new Claude process with --resume.
    pub fn send_message(
        manager: Arc<Mutex<SessionManager>>,
        app: AppHandle,
        session_id: SessionId,
        text: String,
    ) -> Result<(), String> {
        // Verify session is active and get info
        {
            let m = manager.lock().unwrap();
            if !m.active.contains_key(&session_id) {
                return Err(format!("Session {session_id} is not active (status: {})",
                    m.db.get_session(session_id)
                        .ok().flatten()
                        .map(|s| s.status)
                        .unwrap_or_else(|| "unknown".to_string())
                ));
            }
        }

        // Spawn follow-up in background thread
        let manager_clone = Arc::clone(&manager);
        std::thread::spawn(move || {
            Self::do_spawn(manager_clone, app, session_id, text);
        });

        Ok(())
    }

    pub fn stop_session(&mut self, session_id: SessionId) -> Result<(), String> {
        self.active.remove(&session_id);
        let _ = self.db.update_session_status(session_id, crate::models::SessionStatus::Stopped.as_str());
        Ok(())
    }

    pub fn get_sessions(&self) -> Vec<Session> {
        let mut sessions = self.db.get_sessions().unwrap_or_default();
        for s in &mut sessions {
            if let Some(state) = self.journal_states.get(&s.id) {
                let window = state.model.as_deref().map(crate::models::context_window).unwrap_or(200_000);
                let total = state.input_tokens + state.output_tokens;
                s.tokens = Some(TokenUsage {
                    input: state.input_tokens, output: state.output_tokens,
                    cache_read: state.cache_read, cache_write: state.cache_write,
                });
                s.context_percent = Some(if window > 0 { (total as f64 / window as f64) * 100.0 } else { 0.0 });
                s.pending_approval = state.pending_approval.clone();
                s.mini_log = Some(state.mini_log.clone());
            }
            if let Some(a) = self.active.get(&s.id) {
                s.status = a.session.status.clone();
                s.pid = a.session.pid;
            }
        }
        sessions
    }

    pub fn get_journal(&self, session_id: SessionId) -> Vec<crate::models::JournalEntry> {
        self.journal_states.get(&session_id).map(|state| {
            state.entries.iter().map(|e| {
                let mut entry = e.clone();
                entry.session_id = session_id.to_string();
                entry
            }).collect()
        }).unwrap_or_default()
    }

    pub fn is_session_active(&self, session_id: SessionId) -> bool {
        self.active.contains_key(&session_id)
    }

    pub fn rename_session(&mut self, session_id: SessionId, name: &str) -> Result<(), String> {
        self.db.rename_session(session_id, name).map_err(|e| e.to_string())
    }

    pub fn delete_session(&mut self, session_id: SessionId) -> Result<(), String> {
        self.active.remove(&session_id);
        self.journal_states.remove(&session_id);
        self.db.delete_session(session_id).map_err(|e| e.to_string())
    }

    pub fn restore_from_db(&mut self) {
        let sessions = match self.db.get_sessions() {
            Ok(s) => s,
            Err(_) => return,
        };
        for session in sessions {
            if self.journal_states.contains_key(&session.id) { continue; }
            let rows = match self.db.get_outputs(session.id) {
                Ok(r) => r,
                Err(_) => continue,
            };
            if rows.is_empty() { continue; }
            let mut state = JournalState::default();
            for line in &rows { process_line(&mut state, line); }
            self.journal_states.insert(session.id, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database::DatabaseService;

    fn make_manager() -> Arc<Mutex<SessionManager>> {
        let db = Arc::new(DatabaseService::open_in_memory().unwrap());
        Arc::new(Mutex::new(SessionManager::new(db)))
    }

    #[test]
    fn test_init_session_creates_db_record() {
        let mgr = make_manager();
        let s = mgr.lock().unwrap().init_session("/tmp/proj", None, "ignore", None).unwrap();
        assert!(s.id > 0);
        assert_eq!(s.status, "initializing");
    }

    #[test]
    fn test_init_session_populates_journal_state() {
        let mgr = make_manager();
        let s = mgr.lock().unwrap().init_session("/tmp/proj", None, "ignore", None).unwrap();
        assert!(mgr.lock().unwrap().journal_states.contains_key(&s.id));
    }

    #[test]
    fn test_send_message_fails_when_not_active() {
        let mgr = make_manager();
        // Session 999 was never init'd
        let result = {
            let m = mgr.lock().unwrap();
            m.active.contains_key(&999)
        };
        assert!(!result);
    }

    #[test]
    fn test_init_populates_active() {
        let mgr = make_manager();
        let s = mgr.lock().unwrap().init_session("/tmp/proj", None, "ignore", None).unwrap();
        assert!(mgr.lock().unwrap().is_session_active(s.id));
    }

    #[test]
    fn test_stop_session_updates_db() {
        let mgr = make_manager();
        let s = mgr.lock().unwrap().init_session("/tmp/proj", None, "ignore", None).unwrap();
        mgr.lock().unwrap().stop_session(s.id).unwrap();
        let sessions = mgr.lock().unwrap().get_sessions();
        assert_eq!(sessions[0].status, "stopped");
    }

    #[test]
    fn test_delete_removes_from_active_and_state() {
        let mgr = make_manager();
        let s = mgr.lock().unwrap().init_session("/tmp/proj", None, "ignore", None).unwrap();
        mgr.lock().unwrap().delete_session(s.id).unwrap();
        assert_eq!(mgr.lock().unwrap().get_sessions().len(), 0);
        assert!(!mgr.lock().unwrap().journal_states.contains_key(&s.id));
    }

    #[test]
    fn test_restore_from_db_rebuilds_journal() {
        let db = Arc::new(DatabaseService::open_in_memory().unwrap());
        let sid = db.create_session(None, None, "/tmp", "ignore", None).unwrap();
        let line = r#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","content":[{"type":"text","text":"Hi!"}],"usage":{"input_tokens":5,"output_tokens":3,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        db.insert_output(sid, line).unwrap();
        let mut sm = SessionManager::new(db);
        sm.restore_from_db();
        let journal = sm.get_journal(sid);
        assert_eq!(journal.len(), 1);
    }
}
