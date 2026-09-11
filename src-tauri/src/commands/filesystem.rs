use crate::models::*;
use crate::services::database::Database;
use crate::services::local;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, State};

#[tauri::command]
pub fn add_watched_folder(
    app: tauri::AppHandle,
    path: String,
    recursive: Option<bool>,
    db: State<'_, Arc<Database>>,
) -> Result<WatchedFolder, String> {
    if !std::path::Path::new(&path).is_dir() {
        return Err("That isn't a folder".into());
    }
    let recursive = recursive.unwrap_or(true);
    let _ = app.asset_protocol_scope().allow_directory(&path, recursive);
    db.add_watched_folder(&path, recursive)
}

#[tauri::command]
pub fn remove_watched_folder(path: String, db: State<'_, Arc<Database>>) -> Result<(), String> {
    db.remove_watched_folder(&path)
}

#[tauri::command]
pub fn get_watched_folders(db: State<'_, Arc<Database>>) -> Result<Vec<WatchedFolder>, String> {
    db.get_watched_folders()
}

#[tauri::command]
pub async fn get_local_images(folder_path: String, recursive: Option<bool>) -> Result<Vec<LocalImage>, String> {
    let path = PathBuf::from(&folder_path);
    if !path.is_dir() {
        return Err("Folder not found — it may have been moved or deleted".into());
    }
    let recursive = recursive.unwrap_or(true);
    tokio::task::spawn_blocking(move || local::scan_folder(&path, recursive))
        .await
        .map_err(|e| e.to_string())
}

/// Path of a small cached thumbnail for a local image.
#[tauri::command]
pub async fn get_thumbnail(path: String) -> Result<String, String> {
    local::thumbnail(PathBuf::from(path))
        .await
        .map(|p| p.to_string_lossy().to_string())
}
