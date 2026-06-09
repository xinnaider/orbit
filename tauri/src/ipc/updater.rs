use tauri::{command, AppHandle};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub body: String,
    pub current_version: String,
}

/// Returns None if already on latest version.
#[command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    // Dev builds (`tauri:dev`) run locally with no published release feed — skip
    // the remote check so it never errors with "Could not fetch a valid release JSON".
    if cfg!(debug_assertions) {
        return Ok(None);
    }

    let updater = app.updater_builder().build().map_err(|e| e.to_string())?;

    match updater.check().await {
        Ok(Some(update)) => Ok(Some(UpdateInfo {
            version: update.version.clone(),
            body: update.body.clone().unwrap_or_default(),
            current_version: update.current_version.to_string(),
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Downloads, installs, then restarts the app.
#[command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Err("Updates are disabled in dev builds".to_string());
    }

    let updater = app.updater_builder().build().map_err(|e| e.to_string())?;

    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No update available".to_string())?;

    update
        .download_and_install(|_downloaded, _total| {}, || {})
        .await
        .map_err(|e| e.to_string())?;

    app.restart();
}
