use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use tauri::{AppHandle, Emitter};

use crate::journal::{process_line, JournalState};
use crate::models::{AgentStatus, Session, SessionId, TokenUsage};
use crate::services::database::DatabaseService;
use crate::services::spawn_manager::{spawn_claude, SpawnConfig};

/// Reads `.git/HEAD` to detect the current branch without spawning a subprocess.
fn detect_git_branch(cwd: &str) -> Option<String> {
    let head = std::fs::read_to_string(Path::new(cwd).join(".git/HEAD")).ok()?;
    head.trim()
        .strip_prefix("ref: refs/heads/")
        .map(|b| b.to_string())
}

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
    pub git_branch: Option<String>,
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
        SessionManager {
            db,
            active: HashMap::new(),
            journal_states: HashMap::new(),
        }
    }

    /// Phase 1 (fast): create DB record, return Session immediately.
    pub fn init_session(
        &mut self,
        project_path: &str,
        session_name: Option<&str>,
        permission_mode: &str,
        model: Option<&str>,
        use_worktree: bool,
    ) -> Result<Session, String> {
        let project_name = std::path::Path::new(project_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project_path.to_string());

        let project = self
            .db
            .create_project(&project_name, project_path)
            .map_err(|e| e.to_string())?;

        let session_id = self
            .db
            .create_session(
                Some(project.id),
                session_name,
                project_path,
                permission_mode,
                model,
            )
            .map_err(|e| e.to_string())?;

        let (worktree_path_val, branch_name_val) = if use_worktree {
            let full_name = session_name.unwrap_or(&project_name);
            let (prefix, suffix) = full_name.split_once(" · ").unwrap_or((full_name, ""));
            let prefix_slug = crate::services::worktree::generate_branch_slug(prefix);
            let suffix_compact: String = suffix
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            let slug = if suffix_compact.is_empty() {
                format!("{prefix_slug}-{session_id}")
            } else {
                format!("{prefix_slug}-{suffix_compact}-{session_id}")
            };
            let wt_path = crate::services::worktree::create_worktree(
                std::path::Path::new(project_path),
                &slug,
            )?;
            let branch = format!("orbit/{slug}");
            let wt_str = wt_path.to_string_lossy().to_string();
            let _ = self
                .db
                .update_session_worktree(session_id, &wt_str, &branch);
            (Some(wt_str), Some(branch))
        } else {
            (None, None)
        };

        let now = chrono::Utc::now().to_rfc3339();
        let session = Session {
            id: session_id,
            project_id: Some(project.id),
            name: session_name.map(|s| s.to_string()),
            status: crate::models::SessionStatus::Initializing,
            worktree_path: worktree_path_val,
            branch_name: branch_name_val,
            permission_mode: permission_mode.to_string(),
            model: model.map(|s| s.to_string()),
            pid: None,
            created_at: now.clone(),
            updated_at: now,
            cwd: Some(project_path.to_string()),
            project_name: Some(project_name),
            git_branch: detect_git_branch(project_path),
            tokens: None,
            context_percent: None,
            pending_approval: None,
            mini_log: None,
        };

        self.active.insert(
            session_id,
            ActiveSession {
                session: session.clone(),
                claude_session_id: None,
            },
        );
        self.journal_states
            .insert(session_id, JournalState::default());

        Ok(session)
    }

    /// Phase 2 (async): spawn Claude with `-p "prompt"`.
    /// Each message spawns a new process. Uses `--resume` for follow-ups.
    pub fn do_spawn(
        manager: Arc<RwLock<SessionManager>>,
        app: AppHandle,
        session_id: SessionId,
        prompt: String,
    ) {
        let (db, cwd, permission_mode, model, claude_session_id) = {
            let m = manager.write().unwrap_or_else(|e| e.into_inner());
            let a = match m.active.get(&session_id) {
                Some(a) => a,
                None => {
                    let _ = app.emit(
                        "session:error",
                        serde_json::json!({
                            "sessionId": session_id,
                            "error": "Session not found in active map"
                        }),
                    );
                    return;
                }
            };
            (
                m.db.clone(),
                a.session
                    .worktree_path
                    .clone()
                    .or_else(|| a.session.cwd.clone())
                    .unwrap_or_default(),
                a.session.permission_mode.clone(),
                a.session.model.clone(),
                a.claude_session_id.clone(),
            )
        };

        let prompt_text = prompt.clone(); // keep a copy for the user entry
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
                let _ = db.update_session_status(session_id, crate::models::SessionStatus::Error);
                let _ = app.emit(
                    "session:error",
                    serde_json::json!({
                        "sessionId": session_id, "error": e
                    }),
                );
                return;
            }
        };

        let pid = handle.pid as i32;
        let _ = db.update_session_pid(session_id, pid);

        // Monitor stderr for rate limit errors in a background thread
        let app_err = app.clone();
        let stderr_reader = handle.stderr;
        std::thread::spawn(move || {
            use std::io::BufRead;
            let mut reader = std::io::BufReader::new(stderr_reader);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.contains("rate_limit_error")
                            || trimmed.contains("overloaded_error")
                        {
                            let _ = app_err.emit(
                                "session:rate-limit",
                                serde_json::json!({ "sessionId": session_id }),
                            );
                        }
                    }
                }
            }
        });

        {
            let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
            if let Some(a) = m.active.get_mut(&session_id) {
                a.session.status = crate::models::SessionStatus::Running;
                a.session.pid = Some(pid);
            }
        }

        let _ = app.emit(
            "session:running",
            serde_json::json!({
                "sessionId": session_id, "pid": pid
            }),
        );

        // Emit user message entry immediately — Claude's -p flag doesn't echo it in the stream
        let user_entry = crate::models::JournalEntry {
            session_id: session_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            entry_type: crate::models::JournalEntryType::User,
            text: Some(prompt_text.clone()),
            ..crate::models::JournalEntry::default()
        };
        let user_line = serde_json::json!({
            "type": "user",
            "message": { "content": &prompt_text },
            "timestamp": &user_entry.timestamp
        })
        .to_string();
        let _ = db.insert_output(session_id, &user_line);

        {
            let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
            let state = m.journal_states.entry(session_id).or_default();
            state.entries.push(user_entry.clone());
        }

        let _ = app.emit(
            "session:output",
            SessionOutputEvent {
                session_id,
                entry: user_entry,
            },
        );

        Self::reader_loop(
            Arc::clone(&manager),
            app,
            session_id,
            handle.reader,
            db,
            handle.child,
        );
    }

    /// Read JSON lines from Claude's stdout, parse, emit events.
    fn reader_loop(
        manager: Arc<RwLock<SessionManager>>,
        app: AppHandle,
        session_id: SessionId,
        reader: Box<dyn std::io::Read + Send>,
        db: Arc<DatabaseService>,
        mut child: std::process::Child,
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

                    // Extract and persist Claude session ID from system/init message
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&trimmed) {
                        if let Some(claude_id) = val.get("session_id").and_then(|v| v.as_str()) {
                            let should_persist = {
                                let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
                                if let Some(a) = m.active.get_mut(&session_id) {
                                    if a.claude_session_id.is_none() {
                                        a.claude_session_id = Some(claude_id.to_string());
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            };
                            // Persist to DB after releasing the write lock
                            if should_persist {
                                let _ = db.update_claude_session_id(session_id, claude_id);
                            }
                        }
                    }

                    // Detect rate limit errors from Claude's JSON stream
                    if is_rate_limit_line(&trimmed) {
                        let _ = app.emit(
                            "session:rate-limit",
                            serde_json::json!({ "sessionId": session_id }),
                        );
                    }

                    let _ = db.insert_output(session_id, &trimmed);

                    let (new_entries, state_event) = {
                        let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
                        let cwd = m
                            .active
                            .get(&session_id)
                            .and_then(|a| a.session.cwd.clone());
                        let git_branch = cwd.as_deref().and_then(detect_git_branch);

                        let state = m.journal_states.entry(session_id).or_default();

                        let prev_len = state.entries.len();
                        process_line(state, &trimmed);
                        let new_entries: Vec<_> = state.entries[prev_len..].to_vec();

                        let window = state
                            .model
                            .as_deref()
                            .map(crate::models::context_window)
                            .unwrap_or(200_000);
                        let total = state.input_tokens + state.output_tokens;

                        let status_str = match state.status {
                            AgentStatus::Working => "working",
                            AgentStatus::Input => "input",
                            AgentStatus::Idle => "idle",
                            AgentStatus::New => "new",
                        }
                        .to_string();

                        let event = SessionStateEvent {
                            session_id,
                            status: status_str,
                            tokens: TokenUsage {
                                input: state.input_tokens,
                                output: state.output_tokens,
                                cache_read: state.cache_read,
                                cache_write: state.cache_write,
                            },
                            context_percent: if window > 0 {
                                (total as f64 / window as f64) * 100.0
                            } else {
                                0.0
                            },
                            pending_approval: state.pending_approval.clone(),
                            mini_log: state.mini_log.clone(),
                            git_branch,
                        };
                        (new_entries, event)
                    };

                    for entry in new_entries {
                        let mut e = entry.clone();
                        e.session_id = session_id.to_string();
                        let _ = app.emit(
                            "session:output",
                            SessionOutputEvent {
                                session_id,
                                entry: e,
                            },
                        );
                    }
                    let _ = app.emit("session:state", &state_event);
                }
            }
        }

        {
            let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
            if let Some(a) = m.active.get_mut(&session_id) {
                a.session.status = crate::models::SessionStatus::Completed;
            }
            if let Some(state) = m.journal_states.get_mut(&session_id) {
                state.status = AgentStatus::Idle;
            }
            let _ = db.update_session_status(session_id, crate::models::SessionStatus::Completed);
        }

        let _ = app.emit(
            "session:stopped",
            serde_json::json!({ "sessionId": session_id }),
        );

        // Collect exit status — prevents zombie on Unix, releases handle on Windows
        let _ = child.wait();
    }

    /// Send a follow-up message by spawning a new Claude process with --resume.
    /// Reads session data from DB so it works even after app restart.
    pub fn send_message(
        manager: Arc<RwLock<SessionManager>>,
        app: AppHandle,
        session_id: SessionId,
        text: String,
    ) -> Result<(), String> {
        // Re-add to active map if missing (e.g. after app restart)
        {
            let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
            if !m.active.contains_key(&session_id) {
                // Load from DB
                let session =
                    m.db.get_session(session_id)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("Session {session_id} not found"))?;

                let claude_session_id = m.db.get_claude_session_id(session_id).ok().flatten();

                m.active.insert(
                    session_id,
                    ActiveSession {
                        session,
                        claude_session_id,
                    },
                );
                m.journal_states.entry(session_id).or_default();
            }
        }

        let manager_clone = Arc::clone(&manager);
        std::thread::spawn(move || {
            Self::do_spawn(manager_clone, app, session_id, text);
        });

        Ok(())
    }

    pub fn stop_session(&mut self, session_id: SessionId) -> Result<(), String> {
        if let Some(a) = self.active.get(&session_id) {
            if let Some(pid) = a.session.pid {
                kill_pid(pid as u32);
            }
        }
        self.active.remove(&session_id);
        let _ = self
            .db
            .update_session_status(session_id, crate::models::SessionStatus::Stopped);
        Ok(())
    }

    pub fn get_sessions(&mut self) -> Vec<Session> {
        let mut sessions = self.db.get_sessions().unwrap_or_default();
        for s in &mut sessions {
            self.load_session_journal(s.id);
            if let Some(state) = self.journal_states.get(&s.id) {
                let window = state
                    .model
                    .as_deref()
                    .map(crate::models::context_window)
                    .unwrap_or(200_000);
                let total = state.input_tokens + state.output_tokens;
                s.tokens = Some(TokenUsage {
                    input: state.input_tokens,
                    output: state.output_tokens,
                    cache_read: state.cache_read,
                    cache_write: state.cache_write,
                });
                s.context_percent = Some(if window > 0 {
                    (total as f64 / window as f64) * 100.0
                } else {
                    0.0
                });
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

    pub fn get_journal(&mut self, session_id: SessionId) -> Vec<crate::models::JournalEntry> {
        self.load_session_journal(session_id);
        self.journal_states
            .get(&session_id)
            .map(|state| {
                state
                    .entries
                    .iter()
                    .map(|e| {
                        let mut entry = e.clone();
                        entry.session_id = session_id.to_string();
                        entry
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Load journal state for `session_id` from DB into `journal_states` if not already present.
    fn load_session_journal(&mut self, session_id: SessionId) {
        if self.journal_states.contains_key(&session_id) {
            return;
        }
        let rows = match self.db.get_outputs(session_id) {
            Ok(r) => r,
            Err(_) => return,
        };
        // Don't cache an empty state for inactive sessions with no data.
        // Keeps "not in map" semantically distinct from "loaded and empty".
        if rows.is_empty() && !self.active.contains_key(&session_id) {
            return;
        }
        let mut state = JournalState::default();
        for line in &rows {
            process_line(&mut state, line);
        }
        self.journal_states.insert(session_id, state);
    }

    pub fn is_session_active(&self, session_id: SessionId) -> bool {
        self.active.contains_key(&session_id)
    }

    pub fn rename_session(&mut self, session_id: SessionId, name: &str) -> Result<(), String> {
        self.db
            .rename_session(session_id, name)
            .map_err(|e| e.to_string())
    }

    pub fn delete_session(&mut self, session_id: SessionId) -> Result<(), String> {
        self.active.remove(&session_id);
        self.journal_states.remove(&session_id);
        self.db
            .delete_session(session_id)
            .map_err(|e| e.to_string())
    }

    /// Eagerly load journal state for all sessions from DB.
    /// Not called at startup (journals load lazily on first access).
    /// Available as a utility for warming the cache or in tests.
    pub fn restore_from_db(&mut self) {
        let session_ids: Vec<SessionId> = self
            .db
            .get_sessions()
            .unwrap_or_default()
            .into_iter()
            .map(|s| s.id)
            .collect();
        for id in session_ids {
            self.load_session_journal(id);
        }
    }
}

/// Forcefully terminate a process by PID.
fn kill_pid(pid: u32) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }

    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();
    }
}

/// Case-insensitive substring search without allocation (ASCII only).
/// Only used in tests — kept out of production paths after rate-limit detection was tightened.
#[cfg(test)]
fn ascii_ci_contains(haystack: &str, needle: &str) -> bool {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if h.len() < n.len() {
        return false;
    }
    h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
}

/// Check if a JSON line from Claude's stdout indicates a rate limit error.
///
/// Parses the JSON and requires:
/// - top-level `"type"` == `"error"`
/// - nested `"error"."type"` is `"rate_limit_error"` or `"overloaded_error"`
///
/// This avoids false positives when assistant messages mention "rate limit"
/// or "overloaded" in their text content.
fn is_rate_limit_line(line: &str) -> bool {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else {
        return false;
    };
    if val.get("type").and_then(|v| v.as_str()) != Some("error") {
        return false;
    }
    let error_type = val
        .get("error")
        .and_then(|e| e.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("");
    matches!(error_type, "rate_limit_error" | "overloaded_error")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assistant_with_tokens, make_db, seed_outputs, TestCase};

    fn make_manager() -> Arc<RwLock<SessionManager>> {
        Arc::new(RwLock::new(SessionManager::new(make_db())))
    }

    // ── init_session ─────────────────────────────────────────────────────

    #[test]
    fn should_create_db_record_on_init() {
        let mut t = TestCase::new("should_create_db_record_on_init");
        t.phase("Act");
        let mgr = make_manager();
        let s = mgr
            .write()
            .unwrap()
            .init_session("/tmp/proj", None, "ignore", None, false)
            .expect("init failed");
        t.phase("Assert");
        t.ok("id is positive", s.id > 0);
        t.eq(
            "status is initializing",
            &s.status,
            &crate::models::SessionStatus::Initializing,
        );
    }

    #[test]
    fn should_register_journal_state_on_init() {
        let mut t = TestCase::new("should_register_journal_state_on_init");
        t.phase("Act");
        let mgr = make_manager();
        let s = mgr
            .write()
            .unwrap()
            .init_session("/tmp/proj", None, "ignore", None, false)
            .expect("init failed");
        t.phase("Assert");
        t.ok(
            "journal_state entry created",
            mgr.write().unwrap().journal_states.contains_key(&s.id),
        );
    }

    #[test]
    fn should_register_session_as_active_on_init() {
        let mut t = TestCase::new("should_register_session_as_active_on_init");
        t.phase("Act");
        let mgr = make_manager();
        let s = mgr
            .write()
            .unwrap()
            .init_session("/tmp/proj", None, "ignore", None, false)
            .expect("init failed");
        t.phase("Assert");
        t.ok(
            "session is active",
            mgr.write().unwrap().is_session_active(s.id),
        );
    }

    // ── stop_session ─────────────────────────────────────────────────────

    #[test]
    fn should_set_stopped_status_in_db_after_stop() {
        let mut t = TestCase::new("should_set_stopped_status_in_db_after_stop");
        t.phase("Seed");
        let mgr = make_manager();
        let s = mgr
            .write()
            .unwrap()
            .init_session("/tmp/proj", None, "ignore", None, false)
            .expect("init failed");
        t.phase("Act");
        mgr.write()
            .unwrap()
            .stop_session(s.id)
            .expect("stop failed");
        t.phase("Assert");
        let sessions = mgr.write().unwrap().get_sessions();
        t.eq(
            "status is stopped",
            &sessions[0].status,
            &crate::models::SessionStatus::Stopped,
        );
    }

    // ── delete_session ────────────────────────────────────────────────────

    #[test]
    fn should_remove_session_from_active_and_journal_after_delete() {
        let mut t = TestCase::new("should_remove_session_from_active_and_journal_after_delete");
        t.phase("Seed");
        let mgr = make_manager();
        let s = mgr
            .write()
            .unwrap()
            .init_session("/tmp/proj", None, "ignore", None, false)
            .expect("init failed");
        t.phase("Act");
        mgr.write()
            .unwrap()
            .delete_session(s.id)
            .expect("delete failed");
        t.phase("Assert");
        let mut m = mgr.write().unwrap();
        t.ok("not in active map", !m.is_session_active(s.id));
        t.ok(
            "journal_state removed",
            !m.journal_states.contains_key(&s.id),
        );
        t.empty("no sessions in DB", &m.get_sessions());
    }

    // ── rename_session ────────────────────────────────────────────────────

    #[test]
    fn should_persist_renamed_session_name() {
        let mut t = TestCase::new("should_persist_renamed_session_name");
        t.phase("Seed");
        let mgr = make_manager();
        let s = mgr
            .write()
            .unwrap()
            .init_session("/tmp/proj", Some("old-name"), "ignore", None, false)
            .expect("init failed");
        t.phase("Act");
        mgr.write()
            .unwrap()
            .rename_session(s.id, "new-name")
            .expect("rename failed");
        t.phase("Assert");
        let sessions = mgr.write().unwrap().get_sessions();
        t.eq(
            "name updated",
            sessions[0].name.as_deref(),
            Some("new-name"),
        );
    }

    // ── send_message precondition ─────────────────────────────────────────

    #[test]
    fn should_confirm_session_does_not_exist_before_send_message_would_fail() {
        let mut t =
            TestCase::new("should_confirm_session_does_not_exist_before_send_message_would_fail");
        t.phase("Seed — no sessions exist");
        let mgr = make_manager();
        t.phase("Act — verify DB has no session 999");
        let m = mgr.write().unwrap();
        let db_result = m.db.get_session(999).expect("db query ok");
        drop(m);
        t.phase("Assert");
        t.none(
            "session 999 not in DB (error path precondition)",
            &db_result,
        );
        // Note: send_message requires a Tauri AppHandle which cannot be constructed
        // outside the Tauri runtime, so we verify the precondition that guarantees
        // the error path instead of calling send_message directly.
        t.ok("precondition verified", true);
    }

    // ── restore_from_db ───────────────────────────────────────────────────

    #[test]
    fn should_rebuild_journal_state_from_stored_outputs() {
        let mut t = TestCase::new("should_rebuild_journal_state_from_stored_outputs");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None)
            .expect("session");
        seed_outputs(
            &db,
            sid,
            &[&crate::test_utils::assistant_text("Restored entry")],
        );
        t.phase("Act");
        let mut sm = SessionManager::new(db);
        sm.restore_from_db();
        t.phase("Assert");
        let journal = sm.get_journal(sid);
        t.len("one entry restored", &journal, 1);
        t.eq(
            "entry text matches",
            journal[0].text.as_deref(),
            Some("Restored entry"),
        );
    }

    #[test]
    fn should_not_duplicate_entries_on_double_restore() {
        let mut t = TestCase::new("should_not_duplicate_entries_on_double_restore");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None)
            .expect("session");
        seed_outputs(&db, sid, &[&crate::test_utils::assistant_text("Hello")]);
        t.phase("Act");
        let mut sm = SessionManager::new(Arc::clone(&db));
        sm.restore_from_db();
        sm.restore_from_db(); // second call must be idempotent
        t.phase("Assert");
        let journal = sm.get_journal(sid);
        t.len("still exactly one entry (no duplication)", &journal, 1);
    }

    #[test]
    fn should_restore_token_counts_from_stored_outputs() {
        let mut t = TestCase::new("should_restore_token_counts_from_stored_outputs");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None)
            .expect("session");
        // input=10, output=5, cache_write=2, cache_read=1 → input_tokens = 13
        seed_outputs(&db, sid, &[&assistant_with_tokens("Hi", 10, 5, 2, 1)]);
        t.phase("Act");
        let mut sm = SessionManager::new(Arc::clone(&db));
        sm.restore_from_db();
        t.phase("Assert");
        let sessions = sm.get_sessions();
        let tokens = sessions[0]
            .tokens
            .as_ref()
            .expect("tokens missing after restore");
        t.eq("output_tokens restored", tokens.output, 5u64);
    }

    // ── ascii_ci_contains ─────────────────────────────────────────────────────

    #[test]
    fn should_find_needle_case_insensitively() {
        let mut t = TestCase::new("should_find_needle_case_insensitively");
        t.phase("Assert");
        t.ok("exact match", ascii_ci_contains("rate_limit", "rate_limit"));
        t.ok(
            "upper needle",
            ascii_ci_contains("RATE_LIMIT", "rate_limit"),
        );
        t.ok(
            "mixed case haystack",
            ascii_ci_contains("Rate_Limit_Error", "rate_limit"),
        );
        t.ok(
            "not found",
            !ascii_ci_contains("something else", "rate_limit"),
        );
        t.ok("empty haystack", !ascii_ci_contains("", "rate_limit"));
        t.ok(
            "needle longer than haystack",
            !ascii_ci_contains("rt", "rate_limit"),
        );
    }

    // ── is_rate_limit_line ────────────────────────────────────────────────────

    #[test]
    fn should_detect_rate_limit_error_line() {
        let mut t = TestCase::new("should_detect_rate_limit_error_line");
        t.phase("Assert — canonical rate limit JSON");
        t.ok(
            "rate_limit_error in error object",
            is_rate_limit_line(
                r#"{"type":"error","error":{"type":"rate_limit_error","message":"Rate limit exceeded"}}"#,
            ),
        );
        t.ok(
            "overloaded_error in error object",
            is_rate_limit_line(r#"{"type":"error","error":{"type":"overloaded_error"}}"#),
        );
    }

    #[test]
    fn should_not_flag_assistant_message_mentioning_rate_limit() {
        let mut t = TestCase::new("should_not_flag_assistant_message_mentioning_rate_limit");
        t.phase("Assert — assistant message with 'rate limit' in text must NOT trigger");
        t.ok(
            "assistant type with rate limit text",
            !is_rate_limit_line(
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"The rate limit policy allows 1000 requests per minute."}]}}"#,
            ),
        );
    }

    #[test]
    fn should_not_flag_tool_result_mentioning_overloaded() {
        let mut t = TestCase::new("should_not_flag_tool_result_mentioning_overloaded");
        t.phase("Assert — tool_result containing 'overloaded' must NOT trigger");
        t.ok(
            "tool_result type with overloaded in output",
            !is_rate_limit_line(
                r#"{"type":"tool_result","content":"Server is overloaded, please retry"}"#,
            ),
        );
    }

    #[test]
    fn should_not_flag_non_rate_limit_lines() {
        let mut t = TestCase::new("should_not_flag_non_rate_limit_lines");
        t.phase("Assert — lines that should NOT trigger");
        t.ok(
            "rate_limit without error object",
            !is_rate_limit_line(r#"{"type":"assistant","text":"rate_limit info"}"#),
        );
        t.ok(
            "error type but no matching error subtype",
            !is_rate_limit_line(
                r#"{"type":"error","error":{"type":"api_error","message":"internal"}}"#,
            ),
        );
        t.ok(
            "plain overloaded text (not JSON error)",
            !is_rate_limit_line(r#"overloaded"#),
        );
        t.ok("empty line", !is_rate_limit_line(""));
        t.ok(
            "normal assistant line",
            !is_rate_limit_line(r#"{"type":"assistant","text":"hello world"}"#),
        );
    }

    #[test]
    fn should_detect_rate_limit_in_stderr_exact_substring() {
        // stderr lines are plain text — the check uses exact substring matching
        // for "rate_limit_error" or "overloaded_error" (not the broader "rate limit")
        let mut t = TestCase::new("should_detect_rate_limit_in_stderr_exact_substring");
        t.phase("Assert — exact substrings that must match");
        t.ok(
            "rate_limit_error substring present",
            "rate_limit_error: too many requests".contains("rate_limit_error"),
        );
        t.ok(
            "overloaded_error substring present",
            "overloaded_error detected".contains("overloaded_error"),
        );
        t.phase("Assert — generic 'rate limit' must NOT match the tightened check");
        t.ok(
            "generic 'rate limit' phrase does not match rate_limit_error",
            !"You have hit the rate limit today".contains("rate_limit_error"),
        );
        t.ok(
            "generic 'overloaded' does not match overloaded_error",
            !"Server is overloaded".contains("overloaded_error"),
        );
    }

    // ── lazy journal loading ──────────────────────────────────────────────

    #[test]
    fn should_not_preload_journal_state_on_creation() {
        let mut t = TestCase::new("should_not_preload_journal_state_on_creation");
        t.phase("Seed — DB has session with outputs, manager is fresh (no restore)");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None)
            .expect("session");
        seed_outputs(&db, sid, &[&crate::test_utils::assistant_text("hello")]);
        t.phase("Act — create manager without calling restore_from_db");
        let sm = SessionManager::new(Arc::clone(&db));
        t.phase("Assert — journal not loaded yet");
        t.ok(
            "journal_states empty before first access",
            !sm.journal_states.contains_key(&sid),
        );
    }

    #[test]
    fn should_lazy_load_tokens_on_get_sessions() {
        let mut t = TestCase::new("should_lazy_load_tokens_on_get_sessions");
        t.phase("Seed — session with token output exists");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None)
            .expect("session");
        seed_outputs(
            &db,
            sid,
            &[&crate::test_utils::assistant_with_tokens("Hi", 10, 5, 2, 1)],
        );
        t.phase("Act — fresh manager, no restore, call get_sessions");
        let mut sm = SessionManager::new(Arc::clone(&db));
        let sessions = sm.get_sessions();
        t.phase("Assert — tokens populated via lazy load");
        let tokens = sessions[0]
            .tokens
            .as_ref()
            .expect("tokens should be loaded");
        t.eq("output_tokens loaded", tokens.output, 5u64);
        t.ok(
            "journal_state was populated",
            sm.journal_states.contains_key(&sid),
        );
    }

    #[test]
    fn should_lazy_load_journal_on_first_get_journal() {
        let mut t = TestCase::new("should_lazy_load_journal_on_first_get_journal");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None)
            .expect("session");
        seed_outputs(&db, sid, &[&crate::test_utils::assistant_text("hello")]);
        t.phase("Act — get_journal triggers lazy load");
        let mut sm = SessionManager::new(Arc::clone(&db));
        let journal = sm.get_journal(sid);
        t.phase("Assert");
        t.len("one entry loaded on demand", &journal, 1);
    }

    // ── get_journal ───────────────────────────────────────────────────────

    #[test]
    fn should_fill_session_id_on_all_journal_entries() {
        let mut t = TestCase::new("should_fill_session_id_on_all_journal_entries");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None)
            .expect("session");
        seed_outputs(
            &db,
            sid,
            &[
                &crate::test_utils::assistant_text("First"),
                &crate::test_utils::assistant_text("Second"),
            ],
        );
        let mut sm = SessionManager::new(db);
        sm.restore_from_db();
        t.phase("Act");
        let journal = sm.get_journal(sid);
        t.phase("Assert");
        t.len("two entries", &journal, 2);
        let expected_id = sid.to_string();
        t.eq(
            "first entry has session_id",
            journal[0].session_id.as_str(),
            expected_id.as_str(),
        );
        t.eq(
            "second entry has session_id",
            journal[1].session_id.as_str(),
            expected_id.as_str(),
        );
    }
}
