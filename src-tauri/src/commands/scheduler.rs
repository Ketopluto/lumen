use crate::models::*;
use crate::services::scheduler::SlideshowScheduler;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn start_slideshow(
    app: tauri::AppHandle,
    config: SlideshowConfig,
    scheduler: State<'_, Arc<SlideshowScheduler>>,
) -> Result<(), String> {
    scheduler.start(app, config).await
}

#[tauri::command]
pub async fn stop_slideshow(scheduler: State<'_, Arc<SlideshowScheduler>>) -> Result<(), String> {
    scheduler.stop().await;
    Ok(())
}

/// The running slideshow's config, or null when stopped.
#[tauri::command]
pub async fn get_slideshow_status(
    scheduler: State<'_, Arc<SlideshowScheduler>>,
) -> Result<Option<SlideshowConfig>, String> {
    Ok(scheduler.status().await)
}
