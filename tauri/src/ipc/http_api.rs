use serde::Serialize;
use std::net::UdpSocket;
use tauri::State;
use uuid::Uuid;

use crate::ipc::IpcError;
use crate::services::http_server::hash_api_key;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyCreated {
    pub id: String,
    pub label: String,
    pub key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyInfo {
    pub id: String,
    pub label: String,
    pub created_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpSettingsInfo {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

#[tauri::command]
pub fn generate_api_key(
    label: String,
    session_state: State<'_, crate::ipc::session::SessionState>,
) -> Result<ApiKeyCreated, IpcError> {
    let m = session_state
        .0
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let id = Uuid::new_v4().to_string();
    let key = format!("orbit_{}", Uuid::new_v4().simple());
    let key_hash = hash_api_key(&key);

    m.db.create_api_key(&id, &label, &key_hash)
        .map_err(|e| IpcError::Other(e.to_string()))?;

    Ok(ApiKeyCreated { id, label, key })
}

#[tauri::command]
pub fn list_api_keys(
    session_state: State<'_, crate::ipc::session::SessionState>,
) -> Result<Vec<ApiKeyInfo>, IpcError> {
    let m = session_state
        .0
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let keys = m
        .db
        .list_api_keys()
        .map_err(|e| IpcError::Other(e.to_string()))?;
    Ok(keys
        .into_iter()
        .map(|(id, label, created_at)| ApiKeyInfo {
            id,
            label,
            created_at,
        })
        .collect())
}

#[tauri::command]
pub fn revoke_api_key(
    id: String,
    session_state: State<'_, crate::ipc::session::SessionState>,
) -> Result<bool, IpcError> {
    let m = session_state
        .0
        .read()
        .unwrap_or_else(|e| e.into_inner());
    m.db.delete_api_key(&id)
        .map_err(|e| IpcError::Other(e.to_string()))
}

#[tauri::command]
pub fn get_http_settings(
    session_state: State<'_, crate::ipc::session::SessionState>,
) -> Result<HttpSettingsInfo, IpcError> {
    let m = session_state
        .0
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let enabled = m
        .db
        .get_http_setting("enabled")
        .unwrap_or(None)
        .map(|v| v == "true")
        .unwrap_or(false);
    let host = m
        .db
        .get_http_setting("host")
        .unwrap_or(None)
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = m
        .db
        .get_http_setting("port")
        .unwrap_or(None)
        .and_then(|v| v.parse().ok())
        .unwrap_or(9999);
    Ok(HttpSettingsInfo {
        enabled,
        host,
        port,
    })
}

#[tauri::command]
pub fn set_http_settings(
    enabled: bool,
    host: String,
    port: u16,
    session_state: State<'_, crate::ipc::session::SessionState>,
) -> Result<(), IpcError> {
    let m = session_state
        .0
        .read()
        .unwrap_or_else(|e| e.into_inner());
    m.db
        .set_http_setting("enabled", if enabled { "true" } else { "false" })
        .map_err(|e| IpcError::Other(e.to_string()))?;
    m.db
        .set_http_setting("host", &host)
        .map_err(|e| IpcError::Other(e.to_string()))?;
    m.db
        .set_http_setting("port", &port.to_string())
        .map_err(|e| IpcError::Other(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn get_lan_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}
