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

    /// Create a new session: persist to DB, spawn PTY, start reader thread.
    pub fn create_session(
        manager: Arc<Mutex<SessionManager>>,
        app: AppHandle,
        project_path: String,
        prompt: String,
        model: Option<String>,
        permission_mode: String,
        session_name: Option<String>,
    ) -> Result<Session, String> {
        // 1. Resolve project name from path
        let project_name = std::path::Path::new(&project_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project_path.clone());

        let db = {
            let m = manager.lock().unwrap();
            m.db.clone()
        };

        // 2. Create or get project record
        let project = db.create_project(&project_name, &project_path)
            .map_err(|e| e.to_string())?;

        // 3. Create session record in DB
        let session_id = db.create_session(
            Some(project.id),
            session_name.as_deref(),
            &project_path,
            &permission_mode,
            model.as_deref(),
        ).map_err(|e| e.to_string())?;

        // 4. Spawn PTY process
        let handle = spawn_claude(SpawnConfig {
            session_id,
            cwd: std::path::PathBuf::from(&project_path),
            permission_mode: permission_mode.clone(),
            model: model.clone(),
        })?;

        let pid = handle.pid as i32;

        // 5. Update DB with PID and status = "running"
        db.update_session_pid(session_id, pid)
            .map_err(|e| e.to_string())?;

        // 6. Build in-memory Session struct
        let now = chrono::Utc::now().to_rfc3339();
        let session = Session {
            id: session_id,
            project_id: Some(project.id),
            name: session_name,
            status: "running".to_string(),
            worktree_path: None,
            branch_name: None,
            permission_mode: permission_mode.clone(),
            model,
            pid: Some(pid),
            created_at: now.clone(),
            updated_at: now,
            cwd: Some(project_path.clone()),
            project_name: Some(project_name),
            git_branch: None,
            tokens: None,
            context_percent: None,
            pending_approval: None,
            mini_log: None,
        };

        // 7. Register session + writer in active map
        {
            let mut m = manager.lock().unwrap();
            m.active.insert(session_id, ActiveSession {
                session: session.clone(),
                writer: handle.writer,
            });
            m.journal_states.insert(session_id, JournalState::default());
        }

        // 8. Send initial prompt via PTY stdin
        {
            let mut m = manager.lock().unwrap();
            if let Some(active) = m.active.get_mut(&session_id) {
                let _ = write!(active.writer, "{}\n", prompt);
            }
        }

        // 9. Spawn PTY reader thread
        let manager_clone = Arc::clone(&manager);
        let app_clone = app.clone();
        std::thread::spawn(move || {
            Self::pty_reader_loop(manager_clone, app_clone, session_id, handle.reader);
        });

        // 10. Emit session:created event
        let _ = app.emit("session:created", &session);

        Ok(session)
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
                active.session.status = "completed".to_string();
            }
            if let Some(state) = m.journal_states.get_mut(&session_id) {
                state.status = AgentStatus::Idle;
            }
            let db = m.db.clone();
            let _ = db.update_session_status(session_id, "completed");
        }

        let _ = app.emit("session:stopped", serde_json::json!({ "sessionId": session_id }));
    }

    /// Write a message to the session's PTY stdin.
    pub fn send_message(&mut self, session_id: SessionId, text: &str) -> Result<(), String> {
        let active = self.active.get_mut(&session_id)
            .ok_or_else(|| format!("Session {session_id} not active"))?;
        write!(active.writer, "{}\n", text)
            .map_err(|e| e.to_string())
    }

    /// Stop a running session by removing it from active map and updating DB.
    pub fn stop_session(&mut self, session_id: SessionId) -> Result<(), String> {
        self.active.remove(&session_id);
        let _ = self.db.update_session_status(session_id, "stopped");
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
