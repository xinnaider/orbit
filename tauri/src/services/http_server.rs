use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::agent_tree;
use crate::commands::providers::{build_cli_backends, normalize_session_provider_model};
use crate::models::{JournalEntryType, SessionId};
use crate::providers::ProviderRegistry;
use crate::services::database::DatabaseService;
use crate::services::session_manager::SessionManager;

// ── Shared state ────────────────────────────────────────────────

#[derive(Clone)]
pub struct HttpState {
    pub session_manager: Arc<RwLock<SessionManager>>,
    pub registry: Arc<ProviderRegistry>,
    pub app: AppHandle,
    pub db: Arc<DatabaseService>,
    pub stream_tx: broadcast::Sender<StreamEvent>,
    pub frontend_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StreamEvent {
    pub session_id: SessionId,
    pub event_type: String,
    pub data: Value,
}

// ── Auth middleware ──────────────────────────────────────────────

fn validate_bearer(db: &DatabaseService, headers: &HeaderMap) -> Result<(), StatusCode> {
    let header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let hash = hash_api_key(token);
    db.validate_api_key_hash(&hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .then_some(())
        .ok_or(StatusCode::UNAUTHORIZED)
}

pub fn hash_api_key(key: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    format!("{:016x}", h.finish())
}

// ── Router ──────────────────────────────────────────────────────

pub fn build_router(state: HttpState) -> Router {
    let frontend_dir = state.frontend_dir.clone();

    let api = Router::new()
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions", post(create_session))
        .route("/api/sessions/{id}", get(get_session_status))
        .route("/api/sessions/{id}/stop", post(stop_session))
        .route("/api/sessions/{id}/message", post(send_message))
        .route("/api/sessions/{id}/cancel", post(cancel_session))
        .route("/api/sessions/{id}/subagents", get(get_subagents))
        .route("/api/sessions/{id}/journal", get(get_journal))
        .route("/api/sessions/{id}/rename", post(rename_session))
        .route("/api/sessions/{id}", axum::routing::delete(delete_session))
        .route("/api/providers", get(list_providers))
        .route("/api/ws", get(ws_handler))
        .route("/api/health", get(health))
        .layer(CorsLayer::permissive())
        .with_state(state);

    if let Some(dir) = frontend_dir {
        let serve = ServeDir::new(&dir)
            .fallback(tower_http::services::ServeFile::new(dir.join("index.html")));
        api.fallback_service(serve)
    } else {
        api
    }
}

// ── Start server ────────────────────────────────────────────────

pub async fn start(state: HttpState, host: &str, port: u16) -> Result<(), String> {
    let router = build_router(state);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| format!("Invalid bind address: {e}"))?;

    eprintln!("[orbit:http] listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind {addr}: {e}"))?;

    axum::serve(listener, router)
        .await
        .map_err(|e| format!("HTTP server error: {e}"))
}

// ── Handlers ────────────────────────────────────────────────────

async fn health(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            let hash = hash_api_key(token);
            let valid = state.db.validate_api_key_hash(&hash).unwrap_or(false);
            return Ok(Json(
                json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION"), "authenticated": valid }),
            ));
        }
    }
    Ok(Json(
        json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION"), "authenticated": false }),
    ))
}

async fn list_sessions(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListSessionsParams>,
) -> Result<Json<Value>, StatusCode> {
    validate_bearer(&state.db, &headers)?;

    let mut m = state
        .session_manager
        .write()
        .unwrap_or_else(|e| e.into_inner());

    let mut sessions = m.get_sessions();
    if let Some(filter) = params.status {
        sessions.retain(|s| s.status.as_str() == filter);
    }

    serde_json::to_value(sessions)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
struct ListSessionsParams {
    status: Option<String>,
}

async fn create_session(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(body): Json<CreateSessionBody>,
) -> Result<Json<Value>, StatusCode> {
    validate_bearer(&state.db, &headers)?;

    let (provider_id, model) = match normalize_session_provider_model(
        state.registry.as_ref(),
        body.provider.as_deref(),
        body.model.as_deref(),
    ) {
        Ok(resolved) => resolved,
        Err(error) => return Ok(Json(json!({ "error": error }))),
    };

    let mut session = {
        let mut m = state
            .session_manager
            .write()
            .unwrap_or_else(|e| e.into_inner());
        m.init_session(
            &body.cwd,
            body.name.as_deref(),
            "ignore",
            model.as_deref(),
            false,
            provider_id.as_deref(),
            None,
            None,
            None,
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    let session_id = session.id;

    if let Some(parent_id) = body.parent_session_id {
        let desc = if body.prompt.len() > 80 {
            format!("{}…", &body.prompt[..80])
        } else {
            body.prompt.clone()
        };
        let prov = provider_id.as_deref().unwrap_or("claude-code");
        let mut m = state
            .session_manager
            .write()
            .unwrap_or_else(|e| e.into_inner());
        let _ = m.db.set_session_parent(session_id, parent_id, 1);
        m.register_mcp_subagent(parent_id, session_id, &desc, prov);
        session.parent_session_id = Some(parent_id);
        session.depth = 1;
    }

    let _ = state.app.emit("session:created", &session);

    let manager = Arc::clone(&state.session_manager);
    let reg = Arc::clone(&state.registry);
    let app = state.app.clone();
    let prompt = body.prompt.clone();
    std::thread::spawn(move || {
        SessionManager::do_spawn(manager, app, session_id, prompt, &reg);
    });

    Ok(Json(json!({
        "sessionId": session_id,
        "status": "running"
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionBody {
    cwd: String,
    prompt: String,
    provider: Option<String>,
    model: Option<String>,
    name: Option<String>,
    parent_session_id: Option<i64>,
}

async fn get_session_status(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<SessionId>,
) -> Result<Json<Value>, StatusCode> {
    validate_bearer(&state.db, &headers)?;

    let m = state
        .session_manager
        .read()
        .unwrap_or_else(|e| e.into_inner());

    let session =
        m.db.get_session(id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

    let (tokens, context_percent, mini_log) = if let Some(js) = m.journal_states.get(&id) {
        let resolved_model = js.model.as_deref().or(session.model.as_deref());
        let (_window, pct) = crate::services::session_manager::resolve_context_metrics(
            &session.provider,
            resolved_model,
            js,
        );
        (
            json!({
                "input": js.input_tokens,
                "output": js.output_tokens,
                "cacheRead": js.cache_read,
                "cacheWrite": js.cache_write,
            }),
            pct,
            js.mini_log.clone(),
        )
    } else {
        (json!(null), 0.0, vec![])
    };

    let claude_sid = m.db.get_claude_session_id(id).ok().flatten();
    drop(m);

    let output = extract_output(&state.session_manager, id);
    let subagents = claude_sid
        .as_deref()
        .map(agent_tree::read_subagents)
        .unwrap_or_default();

    Ok(Json(json!({
        "sessionId": id,
        "status": session.status.as_str(),
        "model": session.model,
        "provider": session.provider,
        "output": output,
        "tokens": tokens,
        "contextPercent": context_percent,
        "miniLog": mini_log,
        "subagents": subagents,
    })))
}

async fn send_message(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<SessionId>,
    Json(body): Json<SendMessageBody>,
) -> Result<Json<Value>, StatusCode> {
    validate_bearer(&state.db, &headers)?;

    SessionManager::send_message(
        Arc::clone(&state.session_manager),
        state.app.clone(),
        id,
        body.message,
        Arc::clone(&state.registry),
    )
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(json!({ "sessionId": id, "status": "message_sent" })))
}

#[derive(Deserialize)]
struct SendMessageBody {
    message: String,
}

async fn cancel_session(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<SessionId>,
) -> Result<Json<Value>, StatusCode> {
    validate_bearer(&state.db, &headers)?;

    state
        .session_manager
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .stop_session(id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let _ = state
        .app
        .emit("session:stopped", json!({ "sessionId": id }));

    Ok(Json(json!({ "sessionId": id, "status": "stopped" })))
}

async fn stop_session(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<SessionId>,
) -> Result<Json<Value>, StatusCode> {
    validate_bearer(&state.db, &headers)?;

    state
        .session_manager
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .stop_session(id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let _ = state
        .app
        .emit("session:stopped", json!({ "sessionId": id }));

    Ok(Json(json!({ "sessionId": id, "status": "stopped" })))
}

async fn rename_session(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<SessionId>,
    Json(body): Json<RenameBody>,
) -> Result<Json<Value>, StatusCode> {
    validate_bearer(&state.db, &headers)?;

    state
        .db
        .rename_session(id, &body.name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "sessionId": id, "name": body.name })))
}

#[derive(Deserialize)]
struct RenameBody {
    name: String,
}

async fn delete_session(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<SessionId>,
) -> Result<Json<Value>, StatusCode> {
    validate_bearer(&state.db, &headers)?;

    state
        .db
        .delete_session(id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "sessionId": id, "deleted": true })))
}

async fn get_journal(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<SessionId>,
) -> Result<Json<Value>, StatusCode> {
    validate_bearer(&state.db, &headers)?;

    let m = state
        .session_manager
        .read()
        .unwrap_or_else(|e| e.into_inner());

    let entries = m
        .journal_states
        .get(&id)
        .map(|js| &js.entries)
        .cloned()
        .unwrap_or_default();

    drop(m);

    serde_json::to_value(entries)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_subagents(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<SessionId>,
) -> Result<Json<Value>, StatusCode> {
    validate_bearer(&state.db, &headers)?;

    let m = state
        .session_manager
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let claude_sid =
        m.db.get_claude_session_id(id)
            .ok()
            .flatten()
            .ok_or(StatusCode::NOT_FOUND)?;
    drop(m);

    let subagents = agent_tree::read_subagents(&claude_sid);
    serde_json::to_value(subagents)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn list_providers(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    validate_bearer(&state.db, &headers)?;
    let backends = build_cli_backends(&state.registry);
    serde_json::to_value(backends)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── WebSocket for streaming session output ──────────────────────

async fn ws_handler(
    State(state): State<HttpState>,
    Query(params): Query<WsParams>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, StatusCode> {
    let hash = hash_api_key(&params.token);
    state
        .db
        .validate_api_key_hash(&hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .then_some(())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let rx = state.stream_tx.subscribe();
    let session_filter = params.session_id;

    Ok(ws.on_upgrade(move |socket| handle_ws(socket, rx, session_filter)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WsParams {
    token: String,
    session_id: Option<SessionId>,
}

async fn handle_ws(
    mut socket: WebSocket,
    mut rx: broadcast::Receiver<StreamEvent>,
    session_filter: Option<SessionId>,
) {
    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(evt) => {
                        if let Some(filter) = session_filter {
                            if evt.session_id != filter {
                                continue;
                            }
                        }
                        let payload = serde_json::to_string(&evt).unwrap_or_default();
                        if socket.send(Message::Text(payload.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────

fn extract_output(manager: &Arc<RwLock<SessionManager>>, session_id: SessionId) -> String {
    let m = manager.read().unwrap_or_else(|e| e.into_inner());
    let entries = match m.journal_states.get(&session_id) {
        Some(js) => &js.entries,
        None => return String::new(),
    };
    entries
        .iter()
        .filter(|e| e.entry_type == JournalEntryType::Assistant)
        .filter_map(|e| e.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n")
}
