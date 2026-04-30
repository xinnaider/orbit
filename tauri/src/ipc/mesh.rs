use tauri::State;

use crate::ipc::session::SessionState;
use crate::ipc::IpcError;
use crate::models::{
    AgentTemplate, CanvasAnnotation, Floor, Graph, GraphEdge, GraphNode, MeshNote, Run, RunSession,
    Skill,
};
use crate::services::mesh::repo::{self, NewAnnotation};
use crate::services::mesh::skills;
use crate::services::mesh::MESH_DEFAULT_PROVIDER;

// ── Floors ─────────────────────────────────────────────────

#[tauri::command]
pub fn mesh_create_floor(state: State<SessionState>, name: String) -> Result<Floor, IpcError> {
    let sm = state.read();
    repo::create_floor(&sm.db, &name).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_list_floors(state: State<SessionState>) -> Result<Vec<Floor>, IpcError> {
    let sm = state.read();
    repo::list_floors(&sm.db).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_rename_floor(
    state: State<SessionState>,
    id: i64,
    name: String,
) -> Result<(), IpcError> {
    let sm = state.read();
    repo::rename_floor(&sm.db, id, &name).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_delete_floor(state: State<SessionState>, id: i64) -> Result<(), IpcError> {
    let sm = state.read();
    repo::delete_floor(&sm.db, id).map_err(Into::into)
}

// ── Templates ──────────────────────────────────────────────

#[tauri::command]
pub fn mesh_create_template(
    state: State<SessionState>,
    floor_id: i64,
    name: String,
    pre_prompt: String,
    model: Option<String>,
    use_worktree: bool,
    provider: Option<String>,
) -> Result<AgentTemplate, IpcError> {
    let sm = state.read();
    let prov = provider.as_deref().unwrap_or(MESH_DEFAULT_PROVIDER);
    repo::create_template(
        &sm.db,
        floor_id,
        &name,
        &pre_prompt,
        model.as_deref(),
        use_worktree,
        prov,
    )
    .map_err(Into::into)
}

#[tauri::command]
pub fn mesh_list_templates(
    state: State<SessionState>,
    floor_id: i64,
) -> Result<Vec<AgentTemplate>, IpcError> {
    let sm = state.read();
    repo::list_templates(&sm.db, floor_id).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_update_template(
    state: State<SessionState>,
    id: i64,
    name: String,
    pre_prompt: String,
    model: Option<String>,
    use_worktree: bool,
) -> Result<(), IpcError> {
    let sm = state.read();
    repo::update_template(
        &sm.db,
        id,
        &name,
        &pre_prompt,
        model.as_deref(),
        use_worktree,
    )
    .map_err(Into::into)
}

#[tauri::command]
pub fn mesh_delete_template(state: State<SessionState>, id: i64) -> Result<(), IpcError> {
    let sm = state.read();
    repo::delete_template(&sm.db, id).map_err(Into::into)
}

// ── Graphs ─────────────────────────────────────────────────

#[tauri::command]
pub fn mesh_create_graph(
    state: State<SessionState>,
    floor_id: i64,
    name: String,
    provider: Option<String>,
) -> Result<Graph, IpcError> {
    let sm = state.read();
    let prov = provider.as_deref().unwrap_or(MESH_DEFAULT_PROVIDER);
    repo::create_graph(&sm.db, floor_id, &name, prov).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_list_graphs(state: State<SessionState>, floor_id: i64) -> Result<Vec<Graph>, IpcError> {
    let sm = state.read();
    repo::list_graphs(&sm.db, floor_id).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_set_graph_entry(
    state: State<SessionState>,
    id: i64,
    entry_node_id: Option<i64>,
) -> Result<(), IpcError> {
    let sm = state.read();
    repo::update_graph_entry(&sm.db, id, entry_node_id).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_set_graph_provider(
    state: State<SessionState>,
    id: i64,
    provider: String,
) -> Result<(), IpcError> {
    let sm = state.read();
    repo::update_graph_provider(&sm.db, id, &provider).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_delete_graph(state: State<SessionState>, id: i64) -> Result<(), IpcError> {
    let sm = state.read();
    repo::delete_graph(&sm.db, id).map_err(Into::into)
}

// ── Nodes ──────────────────────────────────────────────────

#[tauri::command]
pub fn mesh_add_node(
    state: State<SessionState>,
    graph_id: i64,
    template_id: i64,
    display_name: String,
    x: f64,
    y: f64,
) -> Result<GraphNode, IpcError> {
    let sm = state.read();
    repo::add_node(&sm.db, graph_id, template_id, &display_name, x, y).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_move_node(state: State<SessionState>, id: i64, x: f64, y: f64) -> Result<(), IpcError> {
    let sm = state.read();
    repo::move_node(&sm.db, id, x, y).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_resize_node(
    state: State<SessionState>,
    id: i64,
    width: f64,
    height: f64,
) -> Result<(), IpcError> {
    let sm = state.read();
    repo::resize_node(&sm.db, id, width, height).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_remove_node(state: State<SessionState>, id: i64) -> Result<(), IpcError> {
    let sm = state.read();
    repo::remove_node(&sm.db, id).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_list_nodes(
    state: State<SessionState>,
    graph_id: i64,
) -> Result<Vec<GraphNode>, IpcError> {
    let sm = state.read();
    repo::list_nodes(&sm.db, graph_id).map_err(Into::into)
}

// ── Edges ──────────────────────────────────────────────────

#[tauri::command]
pub fn mesh_add_edge(
    state: State<SessionState>,
    graph_id: i64,
    from_node_id: i64,
    to_node_id: i64,
    from_handle: Option<String>,
    to_handle: Option<String>,
) -> Result<GraphEdge, IpcError> {
    let sm = state.read();
    repo::add_edge(
        &sm.db,
        graph_id,
        from_node_id,
        to_node_id,
        from_handle.as_deref(),
        to_handle.as_deref(),
    )
    .map_err(Into::into)
}

#[tauri::command]
pub fn mesh_remove_edge(state: State<SessionState>, id: i64) -> Result<(), IpcError> {
    let sm = state.read();
    repo::remove_edge(&sm.db, id).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_list_edges(
    state: State<SessionState>,
    graph_id: i64,
) -> Result<Vec<GraphEdge>, IpcError> {
    let sm = state.read();
    repo::list_edges(&sm.db, graph_id).map_err(Into::into)
}

// ── Notes ──────────────────────────────────────────────────

#[tauri::command]
pub fn mesh_note_create(
    state: State<SessionState>,
    graph_id: i64,
    name: String,
    x: f64,
    y: f64,
) -> Result<MeshNote, IpcError> {
    let sm = state.read();
    repo::add_note(&sm.db, graph_id, &name, x, y).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_note_get(state: State<SessionState>, node_id: i64) -> Result<MeshNote, IpcError> {
    let sm = state.read();
    repo::get_note(&sm.db, node_id).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_note_list(
    state: State<SessionState>,
    graph_id: i64,
) -> Result<Vec<MeshNote>, IpcError> {
    let sm = state.read();
    repo::list_notes(&sm.db, graph_id).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_note_set_content(
    state: State<SessionState>,
    node_id: i64,
    content: String,
) -> Result<(), IpcError> {
    let sm = state.read();
    repo::set_note_content(&sm.db, node_id, &content).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_note_rename(
    state: State<SessionState>,
    node_id: i64,
    name: String,
) -> Result<(), IpcError> {
    let sm = state.read();
    repo::rename_note(&sm.db, node_id, &name).map_err(Into::into)
}

// ── Annotations ────────────────────────────────────────────

#[tauri::command]
pub fn mesh_save_annotations(
    state: State<SessionState>,
    graph_id: i64,
    items: Vec<NewAnnotation>,
) -> Result<(), IpcError> {
    let sm = state.read();
    repo::save_annotations(&sm.db, graph_id, &items).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_list_annotations(
    state: State<SessionState>,
    graph_id: i64,
) -> Result<Vec<CanvasAnnotation>, IpcError> {
    let sm = state.read();
    repo::list_annotations(&sm.db, graph_id).map_err(Into::into)
}

// ── Skills (Claude Code skills from ~/.claude/skills/) ─────

#[tauri::command]
pub fn mesh_list_skills() -> Result<Vec<Skill>, IpcError> {
    skills::list_skills().map_err(Into::into)
}

#[tauri::command]
pub fn mesh_read_skill(slug: String) -> Result<Skill, IpcError> {
    skills::read_skill(&slug).map_err(Into::into)
}

// ── Runs ───────────────────────────────────────────────────

#[tauri::command]
pub fn mesh_create_run(
    state: State<SessionState>,
    graph_id: i64,
    entry_node_id: i64,
    initial_prompt: Option<String>,
) -> Result<Run, IpcError> {
    let sm = state.read();
    repo::create_run(&sm.db, graph_id, entry_node_id, initial_prompt.as_deref()).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_finish_run(
    state: State<SessionState>,
    run_id: i64,
    status: String,
) -> Result<(), IpcError> {
    let sm = state.read();
    repo::finish_run(&sm.db, run_id, &status).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_record_run_session(
    state: State<SessionState>,
    run_id: i64,
    node_id: i64,
    session_id: i64,
) -> Result<RunSession, IpcError> {
    let sm = state.read();
    repo::record_run_session(&sm.db, run_id, node_id, session_id).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_list_run_sessions(
    state: State<SessionState>,
    run_id: i64,
) -> Result<Vec<RunSession>, IpcError> {
    let sm = state.read();
    repo::list_run_sessions(&sm.db, run_id).map_err(Into::into)
}

#[tauri::command]
pub fn mesh_find_active_run(
    state: State<SessionState>,
    graph_id: i64,
) -> Result<Option<Run>, IpcError> {
    let sm = state.read();
    repo::find_active_run(&sm.db, graph_id).map_err(Into::into)
}
