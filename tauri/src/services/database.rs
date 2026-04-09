use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::models::{Project, Session, SessionId};

enum WorkerMsg {
    Row(SessionId, String),
    Flush(std::sync::mpsc::SyncSender<()>),
}

pub struct DatabaseService {
    conn: Arc<Mutex<Connection>>,
    output_tx: std::sync::mpsc::SyncSender<WorkerMsg>,
}

fn flush_batch(conn: &Mutex<Connection>, buf: &mut Vec<(SessionId, String)>) {
    if buf.is_empty() {
        return;
    }
    let conn = conn.lock().unwrap_or_else(|e| e.into_inner());
    if let Err(e) = conn.execute_batch("BEGIN") {
        eprintln!("[orbit] flush_batch: BEGIN failed: {e}");
        buf.clear();
        return;
    }
    for (session_id, data) in buf.drain(..) {
        if let Err(e) = conn.execute(
            "INSERT INTO session_outputs (session_id, data) VALUES (?1, ?2)",
            rusqlite::params![session_id, data],
        ) {
            eprintln!("[orbit] flush_batch: INSERT failed for session {session_id}: {e}");
        }
    }
    if let Err(e) = conn.execute_batch("COMMIT") {
        eprintln!("[orbit] flush_batch: COMMIT failed: {e}");
    }
}

fn start_output_worker(conn: Arc<Mutex<Connection>>) -> std::sync::mpsc::SyncSender<WorkerMsg> {
    let (tx, rx) = std::sync::mpsc::sync_channel::<WorkerMsg>(1024);
    std::thread::spawn(move || {
        let mut buf: Vec<(SessionId, String)> = Vec::with_capacity(64);
        loop {
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
            loop {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                if remaining.is_zero() {
                    break;
                }
                match rx.recv_timeout(remaining) {
                    Ok(WorkerMsg::Row(sid, data)) => buf.push((sid, data)),
                    Ok(WorkerMsg::Flush(reply)) => {
                        flush_batch(&conn, &mut buf);
                        let _ = reply.send(());
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        flush_batch(&conn, &mut buf);
                        return;
                    }
                }
            }
            if !buf.is_empty() {
                flush_batch(&conn, &mut buf);
            }
        }
    });
    tx
}

impl DatabaseService {
    pub fn open(path: &Path) -> SqlResult<Self> {
        let conn = Arc::new(Mutex::new(Connection::open(path)?));
        let output_tx = start_output_worker(Arc::clone(&conn));
        let db = DatabaseService { conn, output_tx };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> SqlResult<Self> {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory()?));
        let output_tx = start_output_worker(Arc::clone(&conn));
        let db = DatabaseService { conn, output_tx };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        // Run schema migrations (errors ignored — column may already exist)
        let _ = conn.execute_batch("ALTER TABLE sessions ADD COLUMN claude_session_id TEXT");
        let _ = conn.execute_batch("ALTER TABLE sessions ADD COLUMN cwd TEXT");

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS projects (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                name       TEXT NOT NULL,
                path       TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id                INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id        INTEGER REFERENCES projects(id),
                name              TEXT,
                status            TEXT NOT NULL DEFAULT 'initializing',
                worktree_path     TEXT,
                branch_name       TEXT,
                permission_mode   TEXT NOT NULL DEFAULT 'ignore',
                model             TEXT,
                pid               INTEGER,
                cwd               TEXT,
                claude_session_id TEXT,
                created_at        TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at        TEXT NOT NULL DEFAULT (datetime('now'))
            );
            -- Add claude_session_id column if upgrading from older schema
            CREATE TABLE IF NOT EXISTS _migrations (name TEXT PRIMARY KEY);

            CREATE TABLE IF NOT EXISTS session_outputs (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL REFERENCES sessions(id),
                data       TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_session_outputs_session_id
                ON session_outputs(session_id);
        ",
        )?;
        Ok(())
    }

    pub fn create_project(&self, name: &str, path: &str) -> SqlResult<Project> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR IGNORE INTO projects (name, path) VALUES (?1, ?2)",
            params![name, path],
        )?;
        let project = conn.query_row(
            "SELECT id, name, path, created_at FROM projects WHERE path = ?1",
            params![path],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )?;
        Ok(project)
    }

    pub fn create_session(
        &self,
        project_id: Option<i64>,
        name: Option<&str>,
        cwd: &str,
        permission_mode: &str,
        model: Option<&str>,
    ) -> SqlResult<SessionId> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO sessions (project_id, name, cwd, status, permission_mode, model)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                project_id,
                name,
                cwd,
                crate::models::SessionStatus::Initializing,
                permission_mode,
                model
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_session_status(
        &self,
        id: SessionId,
        status: crate::models::SessionStatus,
    ) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "UPDATE sessions SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }

    pub fn update_session_pid(&self, id: SessionId, pid: i32) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "UPDATE sessions SET pid = ?1, status = ?2, updated_at = datetime('now') WHERE id = ?3",
            params![pid, crate::models::SessionStatus::Running, id],
        )?;
        Ok(())
    }

    pub fn update_session_worktree(
        &self,
        id: SessionId,
        worktree_path: &str,
        branch_name: &str,
    ) -> SqlResult<()> {
        self.conn
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .execute(
                "UPDATE sessions SET worktree_path = ?1, branch_name = ?2, \
             updated_at = datetime('now') WHERE id = ?3",
                params![worktree_path, branch_name, id],
            )?;
        Ok(())
    }

    pub fn get_sessions(&self) -> SqlResult<Vec<Session>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, status, worktree_path, branch_name,
                    permission_mode, model, pid, cwd, created_at, updated_at
             FROM sessions ORDER BY created_at DESC",
        )?;
        let sessions = stmt
            .query_map([], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    status: row.get(3)?,
                    worktree_path: row.get(4)?,
                    branch_name: row.get(5)?,
                    permission_mode: row.get(6)?,
                    model: row.get(7)?,
                    pid: row.get(8)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                    cwd: row.get(9)?,
                    project_name: None,
                    git_branch: None,
                    tokens: None,
                    context_percent: None,
                    pending_approval: None,
                    mini_log: None,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(sessions)
    }

    pub fn get_session(&self, id: SessionId) -> SqlResult<Option<Session>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, status, worktree_path, branch_name,
                    permission_mode, model, pid, cwd, created_at, updated_at
             FROM sessions WHERE id = ?1",
        )?;
        let session = stmt
            .query_row(params![id], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    status: row.get(3)?,
                    worktree_path: row.get(4)?,
                    branch_name: row.get(5)?,
                    permission_mode: row.get(6)?,
                    model: row.get(7)?,
                    pid: row.get(8)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                    cwd: row.get(9)?,
                    project_name: None,
                    git_branch: None,
                    tokens: None,
                    context_percent: None,
                    pending_approval: None,
                    mini_log: None,
                })
            })
            .optional()?;
        Ok(session)
    }

    pub fn get_projects(&self) -> SqlResult<Vec<Project>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt =
            conn.prepare("SELECT id, name, path, created_at FROM projects ORDER BY name ASC")?;
        let projects = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(projects)
    }

    pub fn insert_output(&self, session_id: SessionId, data: &str) -> SqlResult<()> {
        let _ = self
            .output_tx
            .send(WorkerMsg::Row(session_id, data.to_string()));
        Ok(())
    }

    /// Block until all pending output rows are written. Required before calling get_outputs in tests.
    pub fn flush_outputs(&self) {
        let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(0);
        let _ = self.output_tx.send(WorkerMsg::Flush(reply_tx));
        let _ = reply_rx.recv();
    }

    pub fn update_claude_session_id(&self, id: SessionId, claude_id: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "UPDATE sessions SET claude_session_id = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![claude_id, id],
        )?;
        Ok(())
    }

    pub fn get_claude_session_id(&self, id: SessionId) -> SqlResult<Option<String>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn
            .query_row(
                "SELECT claude_session_id FROM sessions WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(result)
    }

    pub fn rename_session(&self, id: SessionId, name: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "UPDATE sessions SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![name, id],
        )?;
        Ok(())
    }

    pub fn delete_session(&self, id: SessionId) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute_batch("BEGIN")?;
        conn.execute(
            "DELETE FROM session_outputs WHERE session_id = ?1",
            params![id],
        )?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        conn.execute_batch("COMMIT")?;
        Ok(())
    }

    pub fn get_outputs(&self, session_id: SessionId) -> SqlResult<Vec<String>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt =
            conn.prepare("SELECT data FROM session_outputs WHERE session_id = ?1 ORDER BY id ASC")?;
        let rows = stmt
            .query_map(params![session_id], |row| row.get(0))?
            .collect::<SqlResult<Vec<String>>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assistant_text, seed_session, user_text, TestCase};

    fn make_db() -> DatabaseService {
        DatabaseService::open_in_memory().expect("test setup: failed to open in-memory DB")
    }

    // ── Projects ─────────────────────────────────────────────────────────

    #[test]
    fn should_create_tables_on_migrate() {
        let mut t = TestCase::new("should_create_tables_on_migrate");
        t.phase("Act");
        let db = make_db();
        t.phase("Assert");
        let sessions = db.get_sessions().expect("get_sessions failed");
        t.empty("sessions table exists and is empty", &sessions);
    }

    #[test]
    fn should_create_project_with_correct_fields() {
        let mut t = TestCase::new("should_create_project_with_correct_fields");
        t.phase("Act");
        let db = make_db();
        let p = db
            .create_project("my-app", "/home/user/my-app")
            .expect("create_project failed");
        t.phase("Assert");
        t.eq("name matches", p.name.as_str(), "my-app");
        t.eq("path matches", p.path.as_str(), "/home/user/my-app");
        t.ok("id is positive", p.id > 0);
    }

    #[test]
    fn should_return_same_project_when_path_already_exists() {
        let mut t = TestCase::new("should_return_same_project_when_path_already_exists");
        t.phase("Seed");
        let db = make_db();
        let first = db
            .create_project("my-app", "/home/user/my-app")
            .expect("first failed");
        t.phase("Act");
        let second = db
            .create_project("my-app", "/home/user/my-app")
            .expect("second failed");
        t.phase("Assert");
        t.eq("same ID (idempotent)", first.id, second.id);
    }

    #[test]
    fn should_list_all_projects_ordered_by_name() {
        let mut t = TestCase::new("should_list_all_projects_ordered_by_name");
        t.phase("Seed");
        let db = make_db();
        db.create_project("beta", "/beta").expect("seed beta");
        db.create_project("alpha", "/alpha").expect("seed alpha");
        t.phase("Act");
        let projects = db.get_projects().expect("get_projects failed");
        t.phase("Assert");
        t.len("two projects", &projects, 2);
        t.eq("first is alpha (ASC)", projects[0].name.as_str(), "alpha");
    }

    // ── Sessions ─────────────────────────────────────────────────────────

    #[test]
    fn should_create_session_with_initializing_status() {
        let mut t = TestCase::new("should_create_session_with_initializing_status");
        t.phase("Act");
        let db = make_db();
        let id = db
            .create_session(None, Some("task 1"), "/tmp/proj", "ignore", None)
            .expect("create_session failed");
        t.phase("Assert");
        t.ok("id is positive", id > 0);
        let sessions = db.get_sessions().expect("get_sessions failed");
        t.len("one session", &sessions, 1);
        t.eq(
            "status is initializing",
            &sessions[0].status,
            &crate::models::SessionStatus::Initializing,
        );
    }

    #[test]
    fn should_store_cwd_on_session_create() {
        let mut t = TestCase::new("should_store_cwd_on_session_create");
        t.phase("Act");
        let db = make_db();
        let id = db
            .create_session(None, None, "/tmp/proj", "ignore", None)
            .expect("create failed");
        t.phase("Assert");
        let session = db
            .get_session(id)
            .expect("get failed")
            .expect("session missing");
        t.eq("cwd stored", session.cwd.as_deref(), Some("/tmp/proj"));
    }

    #[test]
    fn should_update_session_status() {
        let mut t = TestCase::new("should_update_session_status");
        t.phase("Seed");
        let db = make_db();
        let id = seed_session(&db);
        t.phase("Act");
        db.update_session_status(id, crate::models::SessionStatus::Running)
            .expect("update failed");
        t.phase("Assert");
        let sessions = db.get_sessions().expect("get_sessions failed");
        t.eq(
            "status updated to running",
            &sessions[0].status,
            &crate::models::SessionStatus::Running,
        );
    }

    #[test]
    fn should_set_running_status_and_pid_on_update_pid() {
        let mut t = TestCase::new("should_set_running_status_and_pid_on_update_pid");
        t.phase("Seed");
        let db = make_db();
        let id = seed_session(&db);
        t.phase("Act");
        db.update_session_pid(id, 12345).expect("update_pid failed");
        t.phase("Assert");
        let s = db.get_session(id).expect("get failed").expect("missing");
        t.eq(
            "status is running",
            &s.status,
            &crate::models::SessionStatus::Running,
        );
        t.eq("pid stored", s.pid, Some(12345));
    }

    #[test]
    fn should_return_none_for_missing_session_id() {
        let mut t = TestCase::new("should_return_none_for_missing_session_id");
        t.phase("Act");
        let db = make_db();
        let result = db.get_session(999).expect("get_session failed");
        t.phase("Assert");
        t.none("returns None for unknown id", &result);
    }

    #[test]
    fn should_associate_session_with_project_via_foreign_key() {
        let mut t = TestCase::new("should_associate_session_with_project_via_foreign_key");
        t.phase("Seed");
        let db = make_db();
        let project = db.create_project("myapp", "/myapp").expect("seed project");
        t.phase("Act");
        let id = db
            .create_session(
                Some(project.id),
                Some("feat"),
                "/myapp",
                "approve",
                Some("claude-sonnet-4-6"),
            )
            .expect("create failed");
        t.phase("Assert");
        let s = db.get_session(id).expect("get failed").expect("missing");
        t.eq("project_id stored", s.project_id, Some(project.id));
        t.eq(
            "model stored",
            s.model.as_deref(),
            Some("claude-sonnet-4-6"),
        );
    }

    #[test]
    fn should_rename_session() {
        let mut t = TestCase::new("should_rename_session");
        t.phase("Seed");
        let db = make_db();
        let id = seed_session(&db);
        t.phase("Act");
        db.rename_session(id, "new-name").expect("rename failed");
        t.phase("Assert");
        let s = db.get_session(id).expect("get failed").expect("missing");
        t.eq("name updated", s.name.as_deref(), Some("new-name"));
    }

    #[test]
    fn should_store_and_retrieve_worktree_path() {
        let mut t = TestCase::new("should_store_and_retrieve_worktree_path");
        t.phase("Seed");
        let db = make_db();
        let id = seed_session(&db);
        t.phase("Act");
        db.update_session_worktree(id, "/tmp/wt/branch", "orbit/my-branch")
            .expect("update_worktree failed");
        t.phase("Assert");
        let s = db.get_session(id).expect("get failed").expect("missing");
        t.eq(
            "worktree_path stored",
            s.worktree_path.as_deref(),
            Some("/tmp/wt/branch"),
        );
        t.eq(
            "branch_name stored",
            s.branch_name.as_deref(),
            Some("orbit/my-branch"),
        );
    }

    #[test]
    fn should_persist_and_retrieve_claude_session_id() {
        let mut t = TestCase::new("should_persist_and_retrieve_claude_session_id");
        t.phase("Seed");
        let db = make_db();
        let id = seed_session(&db);
        t.phase("Act");
        db.update_claude_session_id(id, "claude-abc-123")
            .expect("update failed");
        let result = db.get_claude_session_id(id).expect("get failed");
        t.phase("Assert");
        t.eq(
            "claude_session_id stored",
            result.as_deref(),
            Some("claude-abc-123"),
        );
    }

    // ── Outputs ──────────────────────────────────────────────────────────

    #[test]
    fn should_insert_and_retrieve_outputs_in_order() {
        let mut t = TestCase::new("should_insert_and_retrieve_outputs_in_order");
        t.phase("Seed");
        let db = make_db();
        let id = seed_session(&db);
        let line1 = assistant_text("first message");
        let line2 = user_text("second message");
        db.insert_output(id, &line1).expect("insert 1");
        db.insert_output(id, &line2).expect("insert 2");
        db.flush_outputs();
        t.phase("Act");
        let rows = db.get_outputs(id).expect("get_outputs failed");
        t.phase("Assert");
        t.len("two rows", &rows, 2);
        t.eq("first row matches", rows[0].as_str(), line1.as_str());
    }

    #[test]
    fn should_isolate_outputs_per_session() {
        let mut t = TestCase::new("should_isolate_outputs_per_session");
        t.phase("Seed");
        let db = make_db();
        let id1 = db
            .create_session(None, None, "/a", "ignore", None)
            .expect("s1");
        let id2 = db
            .create_session(None, None, "/b", "ignore", None)
            .expect("s2");
        db.insert_output(id1, &assistant_text("session 1 msg"))
            .expect("o1");
        db.insert_output(id2, &assistant_text("session 2 msg"))
            .expect("o2");
        db.flush_outputs();
        t.phase("Assert");
        t.len("session 1 has 1 output", &db.get_outputs(id1).unwrap(), 1);
        t.len("session 2 has 1 output", &db.get_outputs(id2).unwrap(), 1);
    }

    #[test]
    fn should_round_trip_session_status_as_enum() {
        let mut t = TestCase::new("should_round_trip_session_status_as_enum");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None)
            .expect("session");
        db.update_session_status(sid, "stopped").expect("update");
        t.phase("Act");
        let sessions = db.get_sessions().expect("get");
        t.phase("Assert");
        t.eq(
            "status is SessionStatus::Stopped",
            sessions[0].status.clone(),
            crate::models::SessionStatus::Stopped,
        );
    }

    // ── Delete (atomicity) ────────────────────────────────────────────────

    #[test]
    fn should_delete_session_and_its_outputs_together() {
        let mut t = TestCase::new("should_delete_session_and_its_outputs_together");
        t.phase("Seed");
        let db = make_db();
        let id = seed_session(&db);
        db.insert_output(id, &assistant_text("first"))
            .expect("insert");
        db.insert_output(id, &user_text("second"))
            .expect("insert 2");
        db.flush_outputs();
        t.phase("Act");
        db.delete_session(id).expect("delete failed");
        t.phase("Assert");
        t.none("session row removed", &db.get_session(id).expect("get"));
        t.empty("outputs removed", &db.get_outputs(id).expect("outputs"));
    }

    #[test]
    fn should_round_trip_session_status_as_enum() {
        let mut t = TestCase::new("should_round_trip_session_status_as_enum");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None)
            .expect("session");
        db.update_session_status(sid, crate::models::SessionStatus::Stopped)
            .expect("update");
        t.phase("Act");
        let sessions = db.get_sessions().expect("get");
        t.phase("Assert");
        t.eq(
            "status is SessionStatus::Stopped",
            &sessions[0].status,
            &crate::models::SessionStatus::Stopped,
        );
    }
}
