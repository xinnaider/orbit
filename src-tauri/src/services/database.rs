use rusqlite::{Connection, Result as SqlResult, params};
use std::path::Path;
use std::sync::Mutex;

use crate::models::{Project, Session, SessionId};

pub struct DatabaseService {
    conn: Mutex<Connection>,
}

impl DatabaseService {
    pub fn open(path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = DatabaseService { conn: Mutex::new(conn) };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = DatabaseService { conn: Mutex::new(conn) };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS projects (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                name       TEXT NOT NULL,
                path       TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id      INTEGER REFERENCES projects(id),
                name            TEXT,
                status          TEXT NOT NULL DEFAULT 'initializing',
                worktree_path   TEXT,
                branch_name     TEXT,
                permission_mode TEXT NOT NULL DEFAULT 'ignore',
                model           TEXT,
                pid             INTEGER,
                cwd             TEXT,
                created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS session_outputs (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL REFERENCES sessions(id),
                data       TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_session_outputs_session_id
                ON session_outputs(session_id);
        ")?;
        Ok(())
    }

    pub fn create_project(&self, name: &str, path: &str) -> SqlResult<Project> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO projects (name, path) VALUES (?1, ?2)",
            params![name, path],
        )?;
        let project = conn.query_row(
            "SELECT id, name, path, created_at FROM projects WHERE path = ?1",
            params![path],
            |row| Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                created_at: row.get(3)?,
            }),
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
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (project_id, name, cwd, status, permission_mode, model)
             VALUES (?1, ?2, ?3, 'initializing', ?4, ?5)",
            params![project_id, name, cwd, permission_mode, model],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_session_status(&self, id: SessionId, status: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }

    pub fn update_session_pid(&self, id: SessionId, pid: i32) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET pid = ?1, status = 'running', updated_at = datetime('now') WHERE id = ?2",
            params![pid, id],
        )?;
        Ok(())
    }

    pub fn get_sessions(&self) -> SqlResult<Vec<Session>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, status, worktree_path, branch_name,
                    permission_mode, model, pid, cwd, created_at, updated_at
             FROM sessions ORDER BY created_at DESC"
        )?;
        let sessions = stmt.query_map([], |row| {
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
        })?.collect::<SqlResult<Vec<_>>>()?;
        Ok(sessions)
    }

    pub fn get_projects(&self) -> SqlResult<Vec<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, path, created_at FROM projects ORDER BY name ASC"
        )?;
        let projects = stmt.query_map([], |row| Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            created_at: row.get(3)?,
        }))?.collect::<SqlResult<Vec<_>>>()?;
        Ok(projects)
    }

    pub fn insert_output(&self, session_id: SessionId, data: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO session_outputs (session_id, data) VALUES (?1, ?2)",
            params![session_id, data],
        )?;
        Ok(())
    }

    pub fn get_outputs(&self, session_id: SessionId) -> SqlResult<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT data FROM session_outputs WHERE session_id = ?1 ORDER BY id ASC"
        )?;
        let rows = stmt.query_map(params![session_id], |row| row.get(0))?
            .collect::<SqlResult<Vec<String>>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrate_creates_tables() {
        let db = DatabaseService::open_in_memory().unwrap();
        let sessions = db.get_sessions().unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_create_project() {
        let db = DatabaseService::open_in_memory().unwrap();
        let p = db.create_project("my-app", "/home/user/my-app").unwrap();
        assert_eq!(p.name, "my-app");
        assert_eq!(p.path, "/home/user/my-app");
        assert!(p.id > 0);
    }

    #[test]
    fn test_create_project_idempotent() {
        let db = DatabaseService::open_in_memory().unwrap();
        let p1 = db.create_project("my-app", "/home/user/my-app").unwrap();
        let p2 = db.create_project("my-app", "/home/user/my-app").unwrap();
        assert_eq!(p1.id, p2.id);
    }

    #[test]
    fn test_create_session() {
        let db = DatabaseService::open_in_memory().unwrap();
        let id = db.create_session(None, Some("task 1"), "/tmp/proj", "ignore", None).unwrap();
        assert!(id > 0);
        let sessions = db.get_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].status, "initializing");
        assert_eq!(sessions[0].cwd, Some("/tmp/proj".to_string()));
    }

    #[test]
    fn test_update_session_status() {
        let db = DatabaseService::open_in_memory().unwrap();
        let id = db.create_session(None, None, "/tmp/proj", "ignore", None).unwrap();
        db.update_session_status(id, "running").unwrap();
        let sessions = db.get_sessions().unwrap();
        assert_eq!(sessions[0].status, "running");
    }

    #[test]
    fn test_insert_and_get_outputs() {
        let db = DatabaseService::open_in_memory().unwrap();
        let id = db.create_session(None, None, "/tmp/proj", "ignore", None).unwrap();
        db.insert_output(id, r#"{"type":"assistant"}"#).unwrap();
        db.insert_output(id, r#"{"type":"user"}"#).unwrap();
        let rows = db.get_outputs(id).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], r#"{"type":"assistant"}"#);
    }
}
