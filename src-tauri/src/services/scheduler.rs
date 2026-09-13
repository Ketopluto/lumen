use crate::models::*;
use crate::services::database::Database;
use crate::services::{local, wallpaper_engine};
use std::sync::Arc;
use tokio::sync::Mutex;

const SLIDESHOW_SETTING_KEY: &str = "slideshow";

/// Rotates wallpapers on an interval. Persists its config so it resumes after restart.
pub struct SlideshowScheduler {
    task: Mutex<Option<(SlideshowConfig, tauri::async_runtime::JoinHandle<()>)>>,
    db: Arc<Database>,
}

impl SlideshowScheduler {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            task: Mutex::new(None),
            db,
        }
    }

    pub async fn status(&self) -> Option<SlideshowConfig> {
        self.task.lock().await.as_ref().map(|(cfg, _)| cfg.clone())
    }

    fn collect(&self, source: &SlideshowSource) -> Result<Vec<WallpaperInfo>, String> {
        Ok(match source {
            SlideshowSource::Favorites => self.db.get_favorites(None)?.into_iter().map(|f| f.wallpaper).collect(),
            SlideshowSource::Collection(id) => self
                .db
                .get_favorites(Some(id))?
                .into_iter()
                .map(|f| f.wallpaper)
                .collect(),
            SlideshowSource::Folder(path) => {
                let dir = std::path::Path::new(path);
                if !dir.is_dir() {
                    return Err("Slideshow folder doesn't exist".into());
                }
                local::scan_folder(dir, true)
                    .into_iter()
                    .map(|f| WallpaperInfo::from_local(&f.path))
                    .collect()
            }
        })
    }

    pub async fn start(&self, app: tauri::AppHandle, config: SlideshowConfig) -> Result<(), String> {
        let mut items = self.collect(&config.source)?;
        if items.is_empty() {
            return Err(match config.source {
                SlideshowSource::Folder(_) => "That folder has no images or videos".into(),
                _ => "Nothing to show yet — add some favorites first".into(),
            });
        }

        self.stop_task().await;
        let _ = self.db.set_setting(
            SLIDESHOW_SETTING_KEY,
            &serde_json::to_string(&config).map_err(|e| e.to_string())?,
        );

        let db = self.db.clone();
        let interval = std::time::Duration::from_secs(config.interval_secs.max(10));
        let shuffle = config.shuffle;
        let fit = config.fit_mode.clone();

        let handle = tauri::async_runtime::spawn(async move {
            use rand::seq::SliceRandom;
            loop {
                if shuffle {
                    items.shuffle(&mut rand::thread_rng());
                }
                for item in &items {
                    if let Err(e) = wallpaper_engine::apply(&app, &db, item, &fit).await {
                        log::warn!("Slideshow: skipping {}: {}", item.url, e);
                        continue;
                    }
                    tokio::time::sleep(interval).await;
                }
            }
        });

        *self.task.lock().await = Some((config, handle));
        Ok(())
    }

    async fn stop_task(&self) {
        if let Some((_, handle)) = self.task.lock().await.take() {
            handle.abort();
        }
    }

    pub async fn stop(&self) {
        self.stop_task().await;
        let _ = self.db.delete_setting(SLIDESHOW_SETTING_KEY);
    }

    /// Resume the slideshow that was running when the app last exited.
    pub async fn restore(&self, app: tauri::AppHandle) {
        let saved = self.db.get_setting(SLIDESHOW_SETTING_KEY).ok().flatten();
        if let Some(config) = saved.and_then(|s| serde_json::from_str::<SlideshowConfig>(&s).ok()) {
            if let Err(e) = self.start(app, config).await {
                log::warn!("Could not resume slideshow: {}", e);
                let _ = self.db.delete_setting(SLIDESHOW_SETTING_KEY);
            }
        }
    }
}
