use crate::services::updater::{self, UpdateStatus};

/// What release is out, and whether Lumen can install it itself.
#[tauri::command]
pub async fn get_update_status(app: tauri::AppHandle) -> UpdateStatus {
    updater::check(&app).await
}

/// Download the latest release and hand over to its installer. Lumen exits to do it.
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    updater::install(&app).await
}
