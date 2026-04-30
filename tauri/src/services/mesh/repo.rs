use rusqlite::{params, Result as SqlResult};

use crate::models::{
    AgentTemplate, CanvasAnnotation, Floor, Graph, GraphEdge, GraphNode, MeshNote, Run, RunSession,
};
use crate::services::database::DatabaseService;

// ── Floors ────────────────────────────────────────────────────

pub fn create_floor(db: &DatabaseService, name: &str) -> SqlResult<Floor> {
    let conn = db.conn();
    conn.execute("INSERT INTO floors (name) VALUES (?1)", params![name])?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, position, created_at FROM floors WHERE id = ?1",
        params![id],
        |r| {
            Ok(Floor {
                id: r.get(0)?,
                name: r.get(1)?,
                position: r.get(2)?,
                created_at: r.get(3)?,
            })
        },
    )
}

pub fn list_floors(db: &DatabaseService) -> SqlResult<Vec<Floor>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, name, position, created_at FROM floors ORDER BY position ASC, id ASC",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Floor {
                id: r.get(0)?,
                name: r.get(1)?,
                position: r.get(2)?,
                created_at: r.get(3)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;
    Ok(rows)
}

pub fn rename_floor(db: &DatabaseService, id: i64, name: &str) -> SqlResult<()> {
    db.conn().execute(
        "UPDATE floors SET name = ?1 WHERE id = ?2",
        params![name, id],
    )?;
    Ok(())
}

pub fn delete_floor(db: &DatabaseService, id: i64) -> SqlResult<()> {
    db.conn()
        .execute("DELETE FROM floors WHERE id = ?1", params![id])?;
    Ok(())
}

// ── Agent templates ───────────────────────────────────────────

pub fn create_template(
    db: &DatabaseService,
    floor_id: i64,
    name: &str,
    pre_prompt: &str,
    model: Option<&str>,
    use_worktree: bool,
    provider: &str,
) -> SqlResult<AgentTemplate> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO agent_templates (floor_id, name, pre_prompt, model, use_worktree, provider)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            floor_id,
            name,
            pre_prompt,
            model,
            use_worktree as i64,
            provider
        ],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, floor_id, name, pre_prompt, model, provider, use_worktree, created_at, updated_at
         FROM agent_templates WHERE id = ?1",
        params![id],
        |r| {
            Ok(AgentTemplate {
                id: r.get(0)?,
                floor_id: r.get(1)?,
                name: r.get(2)?,
                pre_prompt: r.get(3)?,
                model: r.get(4)?,
                provider: r.get(5)?,
                use_worktree: r.get::<_, i64>(6)? != 0,
                created_at: r.get(7)?,
                updated_at: r.get(8)?,
            })
        },
    )
}

pub fn list_templates(db: &DatabaseService, floor_id: i64) -> SqlResult<Vec<AgentTemplate>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, floor_id, name, pre_prompt, model, provider, use_worktree, created_at, updated_at
         FROM agent_templates WHERE floor_id = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt
        .query_map(params![floor_id], |r| {
            Ok(AgentTemplate {
                id: r.get(0)?,
                floor_id: r.get(1)?,
                name: r.get(2)?,
                pre_prompt: r.get(3)?,
                model: r.get(4)?,
                provider: r.get(5)?,
                use_worktree: r.get::<_, i64>(6)? != 0,
                created_at: r.get(7)?,
                updated_at: r.get(8)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;
    Ok(rows)
}

pub fn update_template(
    db: &DatabaseService,
    id: i64,
    name: &str,
    pre_prompt: &str,
    model: Option<&str>,
    use_worktree: bool,
) -> SqlResult<()> {
    db.conn().execute(
        "UPDATE agent_templates
           SET name=?1, pre_prompt=?2, model=?3, use_worktree=?4, updated_at=datetime('now')
         WHERE id=?5",
        params![name, pre_prompt, model, use_worktree as i64, id],
    )?;
    Ok(())
}

pub fn delete_template(db: &DatabaseService, id: i64) -> SqlResult<()> {
    db.conn()
        .execute("DELETE FROM agent_templates WHERE id = ?1", params![id])?;
    Ok(())
}

// ── Graphs ────────────────────────────────────────────────────

pub fn create_graph(
    db: &DatabaseService,
    floor_id: i64,
    name: &str,
    provider: &str,
) -> SqlResult<Graph> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO graphs (floor_id, name, provider) VALUES (?1, ?2, ?3)",
        params![floor_id, name, provider],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, floor_id, name, entry_node_id, provider, created_at, updated_at
           FROM graphs WHERE id = ?1",
        params![id],
        |r| {
            Ok(Graph {
                id: r.get(0)?,
                floor_id: r.get(1)?,
                name: r.get(2)?,
                entry_node_id: r.get(3)?,
                provider: r.get(4)?,
                created_at: r.get(5)?,
                updated_at: r.get(6)?,
            })
        },
    )
}

pub fn list_graphs(db: &DatabaseService, floor_id: i64) -> SqlResult<Vec<Graph>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, floor_id, name, entry_node_id, provider, created_at, updated_at
           FROM graphs WHERE floor_id = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt
        .query_map(params![floor_id], |r| {
            Ok(Graph {
                id: r.get(0)?,
                floor_id: r.get(1)?,
                name: r.get(2)?,
                entry_node_id: r.get(3)?,
                provider: r.get(4)?,
                created_at: r.get(5)?,
                updated_at: r.get(6)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;
    Ok(rows)
}

pub fn update_graph_entry(
    db: &DatabaseService,
    id: i64,
    entry_node_id: Option<i64>,
) -> SqlResult<()> {
    db.conn().execute(
        "UPDATE graphs SET entry_node_id=?1, updated_at=datetime('now') WHERE id=?2",
        params![entry_node_id, id],
    )?;
    Ok(())
}

pub fn update_graph_provider(db: &DatabaseService, id: i64, provider: &str) -> SqlResult<()> {
    db.conn().execute(
        "UPDATE graphs SET provider=?1, updated_at=datetime('now') WHERE id=?2",
        params![provider, id],
    )?;
    Ok(())
}

pub fn delete_graph(db: &DatabaseService, id: i64) -> SqlResult<()> {
    db.conn()
        .execute("DELETE FROM graphs WHERE id = ?1", params![id])?;
    Ok(())
}

// ── Nodes ────────────────────────────────────────────────────
// Verbs follow graph adjacency convention (`add`/`remove`) instead of
// Orbit's entity convention (`create`/`delete`) used in `database.rs`.

pub fn add_node(
    db: &DatabaseService,
    graph_id: i64,
    template_id: i64,
    display_name: &str,
    x: f64,
    y: f64,
) -> SqlResult<GraphNode> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO graph_nodes (graph_id, template_id, display_name, x, y)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![graph_id, template_id, display_name, x, y],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, graph_id, template_id, display_name, x, y, width, height
           FROM graph_nodes WHERE id = ?1",
        params![id],
        |r| {
            Ok(GraphNode {
                id: r.get(0)?,
                graph_id: r.get(1)?,
                template_id: r.get(2)?,
                display_name: r.get(3)?,
                x: r.get(4)?,
                y: r.get(5)?,
                width: r.get(6)?,
                height: r.get(7)?,
            })
        },
    )
}

pub fn move_node(db: &DatabaseService, id: i64, x: f64, y: f64) -> SqlResult<()> {
    db.conn().execute(
        "UPDATE graph_nodes SET x=?1, y=?2 WHERE id=?3",
        params![x, y, id],
    )?;
    Ok(())
}

pub fn resize_node(db: &DatabaseService, id: i64, width: f64, height: f64) -> SqlResult<()> {
    db.conn().execute(
        "UPDATE graph_nodes SET width=?1, height=?2 WHERE id=?3",
        params![width, height, id],
    )?;
    Ok(())
}

pub fn remove_node(db: &DatabaseService, id: i64) -> SqlResult<()> {
    db.conn()
        .execute("DELETE FROM graph_nodes WHERE id=?1", params![id])?;
    Ok(())
}

pub fn list_nodes(db: &DatabaseService, graph_id: i64) -> SqlResult<Vec<GraphNode>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, graph_id, template_id, display_name, x, y, width, height
           FROM graph_nodes WHERE graph_id = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt
        .query_map(params![graph_id], |r| {
            Ok(GraphNode {
                id: r.get(0)?,
                graph_id: r.get(1)?,
                template_id: r.get(2)?,
                display_name: r.get(3)?,
                x: r.get(4)?,
                y: r.get(5)?,
                width: r.get(6)?,
                height: r.get(7)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;
    Ok(rows)
}

// ── Edges ────────────────────────────────────────────────────

pub fn add_edge(
    db: &DatabaseService,
    graph_id: i64,
    from_node_id: i64,
    to_node_id: i64,
    from_handle: Option<&str>,
    to_handle: Option<&str>,
) -> SqlResult<GraphEdge> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO graph_edges (graph_id, from_node_id, to_node_id, from_handle, to_handle)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![graph_id, from_node_id, to_node_id, from_handle, to_handle],
    )?;
    let id = conn.last_insert_rowid();
    Ok(GraphEdge {
        id,
        graph_id,
        from_node_id,
        to_node_id,
        from_handle: from_handle.map(String::from),
        to_handle: to_handle.map(String::from),
    })
}

pub fn remove_edge(db: &DatabaseService, id: i64) -> SqlResult<()> {
    db.conn()
        .execute("DELETE FROM graph_edges WHERE id=?1", params![id])?;
    Ok(())
}

pub fn list_edges(db: &DatabaseService, graph_id: i64) -> SqlResult<Vec<GraphEdge>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, graph_id, from_node_id, to_node_id, from_handle, to_handle
           FROM graph_edges WHERE graph_id = ?1",
    )?;
    let rows = stmt
        .query_map(params![graph_id], |r| {
            Ok(GraphEdge {
                id: r.get(0)?,
                graph_id: r.get(1)?,
                from_node_id: r.get(2)?,
                to_node_id: r.get(3)?,
                from_handle: r.get(4)?,
                to_handle: r.get(5)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;
    Ok(rows)
}

// ── Canvas annotations ───────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize)]
pub struct NewAnnotation {
    pub kind: String,
    pub payload: String,
    pub z_index: i64,
}

pub fn save_annotations(
    db: &DatabaseService,
    graph_id: i64,
    items: &[NewAnnotation],
) -> SqlResult<()> {
    let mut conn = db.conn();
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM canvas_annotations WHERE graph_id=?1",
        params![graph_id],
    )?;
    for a in items {
        tx.execute(
            "INSERT INTO canvas_annotations (graph_id, kind, payload, z_index)
             VALUES (?1, ?2, ?3, ?4)",
            params![graph_id, a.kind, a.payload, a.z_index],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn list_annotations(db: &DatabaseService, graph_id: i64) -> SqlResult<Vec<CanvasAnnotation>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, graph_id, kind, payload, z_index
           FROM canvas_annotations WHERE graph_id = ?1 ORDER BY z_index ASC, id ASC",
    )?;
    let rows = stmt
        .query_map(params![graph_id], |r| {
            Ok(CanvasAnnotation {
                id: r.get(0)?,
                graph_id: r.get(1)?,
                kind: r.get(2)?,
                payload: r.get(3)?,
                z_index: r.get(4)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;
    Ok(rows)
}

// ── Notes ────────────────────────────────────────────────────
// Notes are graph_nodes pointing to a per-floor system template (provider='note',
// name='__note__'), with their markdown content stored 1:1 in mesh_note_contents.
// The system template is auto-created on first note add for a floor.

pub const NOTE_TEMPLATE_NAME: &str = "__note__";
pub const NOTE_PROVIDER: &str = "note";

fn ensure_note_template(db: &DatabaseService, floor_id: i64) -> SqlResult<i64> {
    let conn = db.conn();
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM agent_templates
                 WHERE floor_id = ?1 AND provider = ?2 AND name = ?3
                 LIMIT 1",
            params![floor_id, NOTE_PROVIDER, NOTE_TEMPLATE_NAME],
            |r| r.get(0),
        )
        .ok();
    if let Some(id) = existing {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO agent_templates (floor_id, name, pre_prompt, model, provider, use_worktree)
         VALUES (?1, ?2, '', NULL, ?3, 0)",
        params![floor_id, NOTE_TEMPLATE_NAME, NOTE_PROVIDER],
    )?;
    Ok(conn.last_insert_rowid())
}

fn floor_id_for_graph(db: &DatabaseService, graph_id: i64) -> SqlResult<i64> {
    db.conn().query_row(
        "SELECT floor_id FROM graphs WHERE id = ?1",
        params![graph_id],
        |r| r.get(0),
    )
}

fn map_note(r: &rusqlite::Row<'_>) -> SqlResult<MeshNote> {
    Ok(MeshNote {
        node_id: r.get(0)?,
        graph_id: r.get(1)?,
        name: r.get(2)?,
        content: r.get(3)?,
        x: r.get(4)?,
        y: r.get(5)?,
        width: r.get(6)?,
        height: r.get(7)?,
        updated_at: r.get(8)?,
    })
}

const NOTE_SELECT: &str = "
    SELECT n.id, n.graph_id, c.name, c.content, n.x, n.y, n.width, n.height, c.updated_at
      FROM graph_nodes n
      JOIN mesh_note_contents c ON c.node_id = n.id
";

pub fn add_note(
    db: &DatabaseService,
    graph_id: i64,
    name: &str,
    x: f64,
    y: f64,
) -> SqlResult<MeshNote> {
    let floor_id = floor_id_for_graph(db, graph_id)?;
    let template_id = ensure_note_template(db, floor_id)?;

    // Borrow the connection once for the whole insert sequence — db.conn() returns
    // a MutexGuard, so re-entering db.conn() while one is live deadlocks.
    let node_id = {
        let conn = db.conn();
        let mut display_name = name.to_string();
        let mut suffix = 2;
        while conn
            .query_row::<i64, _, _>(
                "SELECT id FROM graph_nodes WHERE graph_id = ?1 AND display_name = ?2",
                params![graph_id, display_name],
                |r| r.get(0),
            )
            .is_ok()
        {
            display_name = format!("{name} {suffix}");
            suffix += 1;
        }
        conn.execute(
            "INSERT INTO graph_nodes (graph_id, template_id, display_name, x, y)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![graph_id, template_id, display_name, x, y],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO mesh_note_contents (node_id, name, content) VALUES (?1, ?2, '')",
            params![id, display_name],
        )?;
        id
    };
    get_note(db, node_id)
}

pub fn get_note(db: &DatabaseService, node_id: i64) -> SqlResult<MeshNote> {
    let sql = format!("{NOTE_SELECT} WHERE n.id = ?1");
    db.conn().query_row(&sql, params![node_id], map_note)
}

pub fn list_notes(db: &DatabaseService, graph_id: i64) -> SqlResult<Vec<MeshNote>> {
    let conn = db.conn();
    let sql = format!("{NOTE_SELECT} WHERE n.graph_id = ?1 ORDER BY n.id ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params![graph_id], map_note)?
        .collect::<SqlResult<Vec<_>>>()?;
    Ok(rows)
}

pub fn set_note_content(db: &DatabaseService, node_id: i64, content: &str) -> SqlResult<()> {
    db.conn().execute(
        "UPDATE mesh_note_contents
            SET content=?1, updated_at=datetime('now')
          WHERE node_id=?2",
        params![content, node_id],
    )?;
    Ok(())
}

pub fn rename_note(db: &DatabaseService, node_id: i64, name: &str) -> SqlResult<()> {
    let conn = db.conn();
    conn.execute(
        "UPDATE mesh_note_contents
            SET name=?1, updated_at=datetime('now')
          WHERE node_id=?2",
        params![name, node_id],
    )?;
    // Keep graph_nodes.display_name aligned so connection labels stay consistent.
    // Skip if the new name clashes with another node in the same graph (UNIQUE).
    let _ = conn.execute(
        "UPDATE graph_nodes SET display_name=?1 WHERE id=?2",
        params![name, node_id],
    );
    Ok(())
}

// ── Runs ─────────────────────────────────────────────────────

fn map_run(r: &rusqlite::Row<'_>) -> SqlResult<Run> {
    Ok(Run {
        id: r.get(0)?,
        graph_id: r.get(1)?,
        entry_node_id: r.get(2)?,
        initial_prompt: r.get(3)?,
        status: r.get(4)?,
        max_depth: r.get(5)?,
        timeout_secs: r.get(6)?,
        max_loop_count: r.get(7)?,
        ombro_enabled: r.get::<_, i64>(8)? != 0,
        started_at: r.get(9)?,
        finished_at: r.get(10)?,
        created_at: r.get(11)?,
    })
}

const RUN_COLS: &str = "id, graph_id, entry_node_id, initial_prompt, status, \
                         max_depth, timeout_secs, max_loop_count, ombro_enabled, \
                         started_at, finished_at, created_at";

pub fn create_run(
    db: &DatabaseService,
    graph_id: i64,
    entry_node_id: i64,
    initial_prompt: Option<&str>,
) -> SqlResult<Run> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO runs (graph_id, entry_node_id, initial_prompt, status, started_at)
         VALUES (?1, ?2, ?3, 'running', datetime('now'))",
        params![graph_id, entry_node_id, initial_prompt],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        &format!("SELECT {RUN_COLS} FROM runs WHERE id = ?1"),
        params![id],
        map_run,
    )
}

pub fn get_run(db: &DatabaseService, run_id: i64) -> SqlResult<Run> {
    db.conn().query_row(
        &format!("SELECT {RUN_COLS} FROM runs WHERE id = ?1"),
        params![run_id],
        map_run,
    )
}

pub fn finish_run(db: &DatabaseService, run_id: i64, status: &str) -> SqlResult<()> {
    db.conn().execute(
        "UPDATE runs SET status = ?1, finished_at = datetime('now') WHERE id = ?2",
        params![status, run_id],
    )?;
    Ok(())
}

// ── Run sessions ─────────────────────────────────────────────

pub fn record_run_session(
    db: &DatabaseService,
    run_id: i64,
    node_id: i64,
    session_id: i64,
) -> SqlResult<RunSession> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO run_sessions (run_id, node_id, session_id) VALUES (?1, ?2, ?3)",
        params![run_id, node_id, session_id],
    )?;
    Ok(RunSession {
        id: conn.last_insert_rowid(),
        run_id,
        node_id,
        session_id,
    })
}

pub fn list_run_sessions(db: &DatabaseService, run_id: i64) -> SqlResult<Vec<RunSession>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, run_id, node_id, session_id
           FROM run_sessions WHERE run_id = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt
        .query_map(params![run_id], |r| {
            Ok(RunSession {
                id: r.get(0)?,
                run_id: r.get(1)?,
                node_id: r.get(2)?,
                session_id: r.get(3)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;
    Ok(rows)
}

pub fn find_active_run(db: &DatabaseService, graph_id: i64) -> SqlResult<Option<Run>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(&format!(
        "SELECT {RUN_COLS} FROM runs
           WHERE graph_id = ?1 AND status = 'running'
           ORDER BY id DESC LIMIT 1"
    ))?;
    let mut rows = stmt.query(params![graph_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(map_run(row)?)),
        None => Ok(None),
    }
}
