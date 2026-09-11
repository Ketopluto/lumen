use crate::models::*;
use crate::services::database::Database;
use crate::services::wallpaper_engine::{self, LiveState, LiveStatus};
use std::sync::Arc;
use tauri::State;

/// Set any wallpaper (remote or local, image or video). Returns the local file path.
#[tauri::command]
pub async fn set_wallpaper(
    app: tauri::AppHandle,
    wallpaper: WallpaperInfo,
    fit_mode: Option<FitMode>,
    db: State<'_, Arc<Database>>,
) -> Result<String, String> {
    let fit = fit_mode.unwrap_or_else(|| db.settings().fit_mode);
    wallpaper_engine::apply(&app, &db, &wallpaper, &fit).await
}

/// Download a wallpaper into the download folder without applying it.
#[tauri::command]
pub async fn download_wallpaper(
    app: tauri::AppHandle,
    wallpaper: WallpaperInfo,
    db: State<'_, Arc<Database>>,
) -> Result<String, String> {
    wallpaper_engine::resolve_local(&app, &db, &wallpaper).await
}

#[tauri::command]
pub async fn stop_live_wallpaper(app: tauri::AppHandle) -> Result<(), String> {
    wallpaper_engine::stop_live(&app, true);
    Ok(())
}

#[tauri::command]
pub fn set_live_paused(app: tauri::AppHandle, paused: bool) {
    wallpaper_engine::set_manual_pause(&app, paused);
}

#[tauri::command]
pub fn get_live_status(live: State<'_, LiveState>, db: State<'_, Arc<Database>>) -> LiveStatus {
    live.status(&db)
}

#[tauri::command]
pub async fn get_current_wallpaper() -> Result<String, String> {
    ::wallpaper::get().map_err(|e| format!("Failed to get wallpaper: {}", e))
}
