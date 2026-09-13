use crate::models::*;
use crate::services::database::Database;
use crate::services::desktop;
use crate::services::wallpaper_engine::LIVE_LABEL;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};

#[tauri::command]
pub fn get_settings(db: State<'_, Arc<Database>>) -> AppSettings {
    db.settings()
}

#[tauri::command]
pub fn update_settings(
    app: tauri::AppHandle,
    mut settings: AppSettings,
    db: State<'_, Arc<Database>>,
) -> Result<AppSettings, String> {
    settings.live_wallpaper_volume = settings.live_wallpaper_volume.min(100);
    settings.max_history = settings.max_history.clamp(10, 10_000);
    if settings.download_dir.trim().is_empty() {
        settings.download_dir = crate::utils::default_download_dir().to_string_lossy().to_string();
    }

    let previous = db.settings();
    if previous.start_on_boot != settings.start_on_boot {
        desktop::set_autostart(settings.start_on_boot).map_err(|e| format!("Couldn't change start-on-login: {}", e))?;
    }
    if previous.download_dir != settings.download_dir {
        std::fs::create_dir_all(&settings.download_dir)
            .map_err(|e| format!("Can't use that download folder: {}", e))?;
        let _ = app.asset_protocol_scope().allow_directory(&settings.download_dir, true);
    }

    db.save_settings(&settings)?;
    let _ = app.emit_to(LIVE_LABEL, "live-volume", settings.live_wallpaper_volume);
    Ok(settings)
}
