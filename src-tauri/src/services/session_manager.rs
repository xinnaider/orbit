use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter};

use crate::journal_reader::{JournalState, process_line};
use crate::models::{Session, SessionId, AgentStatus, TokenUsage};
use crate::services::database::DatabaseService;
use crate::services::spawn_manager::{SpawnConfig, PtyHandle, spawn_claude};

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
    writer: Box<dyn Write + Send>,
}

pub struct SessionManager {
    pub db: Arc<DatabaseService>,
    active: HashMap<SessionId, ActiveSession>,
    journal_states: HashMap<SessionId, JournalState>,
}

impl SessionManager {
    pub fn new(db: Arc<DatabaseService>) -> Self {
        SessionManager {
            db,
            active: HashMap::new(),
            journal_states: HashMap::new(),
        }
    }

    /// Phase 1 (fast): create DB records and return Session immediately.
    /// Does NOT spawn any process. Call do_spawn in a background thread after this.
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
            Some(project.id),
            session_name,
            project_path,
            permission_mode,
            model,
        ).map_err(|e| e.to_string())?;

        let now = chrono::Utc::now().to_rfc3339();
        let session = Session {
            id: session_id,
            project_id: Some(project.id),
            name: session_name.map(|s| s.to_string()),
            status: crate::models::SessionStatus::Initializing.as_str().to_string(),
            worktree_path: None,
            branch_name: None,
            permission_mode: permission_mode.to_string(),
            model: model.map(|s| s.to_string()),
            pid: None,
            created_at: now.clone(),
            updated_at: now,
            cwd: Some(project_path.to_string()),
            project_name: Some(project_name),
            git_branch: None,
            tokens: None,
            context_percent: None,
            pending_approval: None,
            mini_log: None,
        };

        self.journal_states.insert(session_id, JournalState::default());

        Ok(session)
    }

    /// Phase 2 (slow): spawn the Claude PTY process for an already-initialised session.
    /// Runs in a background thread. Emits session:error if spawn fails.
    pub fn do_spawn(
        manager: Arc<Mutex<SessionManager>>,
        app: AppHandle,
        session: Session,
        prompt: String,
    ) {
        let session_id = session.id;

        let (db, permission_mode, model, cwd) = {
            let m = manager.lock().unwrap();
            (
                m.db.clone(),
                session.permission_mode.clone(),
                session.model.clone(),
                session.cwd.clone().unwrap_or_default(),
            )
        };

        // Spawn the PTY
        let handle = match spawn_claude(SpawnConfig {
            session_id,
            cwd: std::path::PathBuf::from(&cwd),
            permission_mode: permission_mode.clone(),
            model: model.clone(),
        }) {
            Ok(h) => h,
            Err(e) => {
                let _ = db.update_session_status(session_id, crate::models::SessionStatus::Error.as_str());
                let _ = app.emit("session:error", serde_json::json!({
                    "sessionId": session_id,
                    "error": e
                }));
                return;
            }
        };

        let pid = handle.pid as i32;
        let _ = db.update_session_pid(session_id, pid);

        // Register writer and update session status
        {
            let mut m = manager.lock().unwrap();
            let mut updated = session.clone();
            updated.status = crate::models::SessionStatus::Running.as_str().to_string();
            updated.pid = Some(pid);
            m.active.insert(session_id, ActiveSession {
                session: updated,
                writer: handle.writer,
            });
        }

        // Emit session:running so frontend updates status
        let _ = app.emit("session:running", serde_json::json!({
            "sessionId": session_id,
            "pid": pid
        }));

        // Write initial prompt to PTY stdin
        {
            let mut m = manager.lock().unwrap();
            if let Some(active) = m.active.get_mut(&session_id) {
                // ConPTY on Windows expects \r (carriage return) as Enter key
                let _ = write!(active.writer, "{}\r", prompt);
            }
        }

        // Start PTY reader loop (blocks until process exits)
        Self::pty_reader_loop(Arc::clone(&manager), app, session_id, handle.reader);
    }

    fn pty_reader_loop(
        manager: Arc<Mutex<SessionManager>>,
        app: AppHandle,
        session_id: SessionId,
        reader: Box<dyn std::io::Read + Send>,
    ) {
        use std::io::BufRead;
        let mut reader = std::io::BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF — process exited
                Ok(_) => {
                    let trimmed = line.trim().to_string();
                    if trimmed.is_empty() {
                        continue;
                    }

                    // Update in-memory journal state and collect new entries + state event
                    let (new_entries, state_event, db) = {
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
                        let context_percent = if window > 0 {
                            (total as f64 / window as f64) * 100.0
                        } else {
                            0.0
                        };

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
                            context_percent,
                            pending_approval: state.pending_approval.clone(),
                            mini_log: state.mini_log.clone(),
                        };

                        let db = m.db.clone();
                        (new_entries, event, db)
                    };

                    // Persist raw line to DB outside of lock
                    let _ = db.insert_output(session_id, &trimmed);

                    // Emit new journal entries as individual events
                    for entry in new_entries {
                        let mut e = entry.clone();
                        e.session_id = session_id.to_string();
                        let _ = app.emit("session:output", SessionOutputEvent {
                            session_id,
                            entry: e,
                        });
                    }

                    // Emit state update
                    let _ = app.emit("session:state", &state_event);
                }
                Err(_) => break,
            }
        }

        // PTY exited — mark session as completed
        {
            let mut m = manager.lock().unwrap();
            if let Some(active) = m.active.get_mut(&session_id) {
                active.session.status = crate::models::SessionStatus::Completed.as_str().to_string();
            }
            if let Some(state) = m.journal_states.get_mut(&session_id) {
                state.status = AgentStatus::Idle;
            }
            let db = m.db.clone();
            let _ = db.update_session_status(session_id, crate::models::SessionStatus::Completed.as_str());
        }

        let _ = app.emit("session:stopped", serde_json::json!({ "sessionId": session_id }));
    }

    /// Write a message to the session's PTY stdin.
    /// Uses \r (ConPTY Enter key on Windows).
    pub fn send_message(&mut self, session_id: SessionId, text: &str) -> Result<(), String> {
        let active = self.active.get_mut(&session_id)
            .ok_or_else(|| format!("Session {session_id} not active — it may still be spawning"))?;
        write!(active.writer, "{}\r", text)
            .map_err(|e| e.to_string())
    }

    /// Stop a running session by removing it from active map and updating DB.
    pub fn stop_session(&mut self, session_id: SessionId) -> Result<(), String> {
        self.active.remove(&session_id);
        let _ = self.db.update_session_status(session_id, crate::models::SessionStatus::Stopped.as_str());
        Ok(())
    }

    /// Get all sessions from DB, enriched with in-memory runtime state.
    pub fn get_sessions(&self) -> Vec<Session> {
        let mut sessions = self.db.get_sessions().unwrap_or_default();
        for session in &mut sessions {
            if let Some(state) = self.journal_states.get(&session.id) {
                let window = state.model.as_deref()
                    .map(crate::models::context_window)
                    .unwrap_or(200_000);
                let total = state.input_tokens + state.output_tokens;
                session.tokens = Some(TokenUsage {
                    input: state.input_tokens,
                    output: state.output_tokens,
                    cache_read: state.cache_read,
                    cache_write: state.cache_write,
                });
                session.context_percent = Some(if window > 0 {
                    (total as f64 / window as f64) * 100.0
                } else {
                    0.0
                });
                session.pending_approval = state.pending_approval.clone();
                session.mini_log = Some(state.mini_log.clone());
            }
            if let Some(active) = self.active.get(&session.id) {
                session.status = active.session.status.clone();
            }
        }
        sessions
    }

    /// Get journal entries for a session.
    pub fn get_journal(&self, session_id: SessionId) -> Vec<crate::models::JournalEntry> {
        match self.journal_states.get(&session_id) {
            Some(state) => state.entries.iter().map(|e| {
                let mut entry = e.clone();
                entry.session_id = session_id.to_string();
                entry
            }).collect(),
            None => vec![],
        }
    }

    /// Load journal states for all existing sessions from DB on startup.
    /// Call this once from lib.rs setup after SessionManager is created.
    pub fn restore_from_db(&mut self) {
        let sessions = match self.db.get_sessions() {
            Ok(s) => s,
            Err(_) => return,
        };

        for session in sessions {
            if self.journal_states.contains_key(&session.id) {
                continue; // already loaded (active session)
            }

            let rows = match self.db.get_outputs(session.id) {
                Ok(r) => r,
                Err(_) => continue,
            };

            if rows.is_empty() {
                continue;
            }

            let mut state = JournalState::default();
            for line in &rows {
                process_line(&mut state, line);
            }
            self.journal_states.insert(session.id, state);
        }
    }
}
