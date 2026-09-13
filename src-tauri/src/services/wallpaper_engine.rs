use crate::models::*;
use crate::services::api_client::ApiClient;
use crate::services::database::Database;
use crate::services::desktop;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub use crate::services::desktop::LIVE_LABEL;

const LIVE_SETTING_KEY: &str = "live_wallpaper";

/// State of the live (video/GIF) wallpaper window.
#[derive(Default)]
pub struct LiveState {
    current: Mutex<Option<String>>,
    /// Paused because a fullscreen app is running or the laptop is on battery.
    auto_paused: AtomicBool,
    /// Paused by the user from the app or the tray.
    manual_paused: AtomicBool,
    /// What the player was last told (auto || manual).
    paused: AtomicBool,
}

#[derive(serde::Serialize, Clone)]
pub struct LiveStatus {
    pub path: Option<String>,
    pub paused: bool,
    pub manual_paused: bool,
    pub volume: u32,
}

impl LiveState {
    pub fn current(&self) -> Option<String> {
        self.current.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    fn set_current(&self, value: Option<String>) {
        *self.current.lock().unwrap_or_else(|p| p.into_inner()) = value;
    }

    pub fn status(&self, db: &Database) -> LiveStatus {
        LiveStatus {
            path: self.current(),
            paused: self.paused.load(Ordering::Relaxed),
            manual_paused: self.manual_paused.load(Ordering::Relaxed),
            volume: db.settings().live_wallpaper_volume.min(100),
        }
    }
}

/// Tells every window (UI dock and player) the current live wallpaper state.
fn broadcast_live(app: &AppHandle) {
    let status = app.state::<LiveState>().status(&app.state::<Arc<Database>>());
    let _ = app.emit("live-state", status);
}

/// Recomputes the effective pause state; returns true if it changed.
fn sync_pause(app: &AppHandle) -> bool {
    let state = app.state::<LiveState>();
    let pause = state.auto_paused.load(Ordering::Relaxed) || state.manual_paused.load(Ordering::Relaxed);
    let changed = state.paused.swap(pause, Ordering::Relaxed) != pause;
    if changed {
        // Broadcast: there is one player window per display on some platforms.
        let _ = app.emit("live-pause", pause);
    }
    changed
}

pub fn set_manual_pause(app: &AppHandle, paused: bool) {
    app.state::<LiveState>().manual_paused.store(paused, Ordering::Relaxed);
    sync_pause(app);
    broadcast_live(app);
}

pub fn toggle_manual_pause(app: &AppHandle) {
    let paused = app.state::<LiveState>().manual_paused.load(Ordering::Relaxed);
    set_manual_pause(app, !paused);
}

/// Download (if needed), apply, and record a wallpaper. Returns the local file path.
pub async fn apply(app: &AppHandle, db: &Arc<Database>, info: &WallpaperInfo, fit: &FitMode) -> Result<String, String> {
    let path = resolve_local(app, db, info).await?;
    if MediaType::from_path(&path).is_live() || info.media_type == MediaType::Video {
        set_live(app, &path)?;
    } else {
        set_static(app, &path, fit)?;
    }

    let mut record = info.clone();
    record.local_path = Some(path.clone());
    let settings = db.settings();
    let _ = db.add_history(&record, settings.max_history);
    if info.source != "local" {
        let _ = db.set_favorite_local_path(&info.id, &path);
    }
    let _ = app.emit("wallpaper-changed", &record);
    Ok(path)
}

/// Returns a local path for the wallpaper, downloading remote files into the download folder.
pub async fn resolve_local(app: &AppHandle, db: &Database, info: &WallpaperInfo) -> Result<String, String> {
    if let Some(p) = info.local_path.as_deref() {
        if Path::new(p).is_file() {
            return Ok(p.to_string());
        }
    }
    if !info.url.starts_with("http://") && !info.url.starts_with("https://") {
        return if Path::new(&info.url).is_file() {
            Ok(info.url.clone())
        } else {
            Err("File not found — it may have been moved or deleted".into())
        };
    }

    let dir = PathBuf::from(db.settings().download_dir);
    let dest = dir.join(download_filename(info));
    if !dest.is_file() {
        ApiClient::download_file(app, &info.id, &info.url, &dest).await?;
        let _ = app.asset_protocol_scope().allow_directory(&dir, false);
    }
    Ok(dest.to_string_lossy().to_string())
}

/// A stable, filesystem-safe filename such as `wallhaven_6lyyvx.jpg`.
pub fn download_filename(info: &WallpaperInfo) -> String {
    let stem: String = info
        .id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(80)
        .collect();
    let url_path = info.url.split(['?', '#']).next().unwrap_or("");
    let ext = url_path
        .rsplit('/')
        .next()
        .and_then(|name| name.rsplit_once('.').map(|(_, e)| e.to_lowercase()))
        .filter(|e| crate::utils::is_wallpaper_extension(e))
        .unwrap_or_else(|| {
            if info.media_type == MediaType::Video {
                "mp4".into()
            } else {
                "jpg".into()
            }
        });
    format!("{}.{}", stem, ext)
}

pub fn set_static(app: &AppHandle, path: &str, fit: &FitMode) -> Result<(), String> {
    stop_live(app, false);
    desktop::set_static_wallpaper(path, fit)
}

/// Keep the playing file inside Lumen's own folder. Cloud folders (OneDrive) move files around or
/// swap them for placeholders, which would make the wallpaper vanish later.
fn stage_live_file(path: &str) -> Result<String, String> {
    let dir = crate::utils::app_data_dir().join("live");
    let src = Path::new(path);
    if src.starts_with(&dir) {
        return Ok(path.to_string());
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = src.file_name().ok_or("That file has no name")?;
    let dest = dir.join(name);
    let same_size = std::fs::metadata(&dest).map(|d| d.len()).ok() == std::fs::metadata(src).map(|s| s.len()).ok();
    if !dest.is_file() || !same_size {
        std::fs::copy(src, &dest).map_err(|e| format!("Could not copy the wallpaper into Lumen's folder: {}", e))?;
    }
    // Only one live wallpaper plays at a time, so drop anything staged earlier.
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path() != dest {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    Ok(dest.to_string_lossy().to_string())
}

/// Every live wallpaper window that currently exists.
fn live_windows(app: &AppHandle) -> Vec<WebviewWindow> {
    app.webview_windows()
        .into_iter()
        .filter(|(label, _)| label == LIVE_LABEL || label.starts_with(&format!("{}_", LIVE_LABEL)))
        .map(|(_, window)| window)
        .collect()
}

fn destroy_surfaces(app: &AppHandle) {
    for window in live_windows(app) {
        let _ = window.destroy();
    }
}

pub fn set_live(app: &AppHandle, path: &str) -> Result<(), String> {
    let caps = desktop::capabilities();
    if !caps.live_wallpaper {
        return Err(caps
            .live_unsupported_reason
            .clone()
            .unwrap_or_else(|| "Live wallpapers aren't supported on this desktop".into()));
    }

    let state = app.state::<LiveState>();
    let db = app.state::<Arc<Database>>();
    if !Path::new(path).is_file() {
        return Err("That file is not available right now".into());
    }
    let staged = stage_live_file(path)?;
    let path = staged.as_str();
    app.asset_protocol_scope().allow_file(path).map_err(|e| e.to_string())?;
    state.set_current(Some(path.to_string()));
    state.manual_paused.store(false, Ordering::Relaxed);
    sync_pause(app);
    let _ = db.set_setting(LIVE_SETTING_KEY, path);

    let specs = desktop::plan_surfaces(app);
    if specs.is_empty() {
        return Err("No display to put a wallpaper on".into());
    }

    // Already playing on the right set of surfaces: just point them at the new file.
    if specs.iter().all(|s| app.get_webview_window(&s.label).is_some()) {
        for spec in &specs {
            app.emit_to(spec.label.as_str(), "live-src", path)
                .map_err(|e| e.to_string())?;
        }
        broadcast_live(app);
        return Ok(());
    }
    // The layout changed (a monitor came or went), so rebuild from scratch.
    destroy_surfaces(app);

    for spec in &specs {
        let mut builder = WebviewWindowBuilder::new(app, &spec.label, WebviewUrl::App("index.html".into()))
            .title("Lumen Live")
            .decorations(false)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .visible(false)
            .initialization_script(desktop::bootstrap_script());
        if let Some((x, y, w, h)) = spec.bounds {
            builder = builder.position(x as f64, y as f64).inner_size(w as f64, h as f64);
        }
        let window = builder
            .build()
            .map_err(|e| format!("Could not create live wallpaper window: {}", e))?;

        // `attach` also reveals the window — see the contract in services/desktop/mod.rs.
        let spec = spec.clone();
        let attached = desktop::on_main(app, move || desktop::attach(&window, &spec))?;
        if let Err(e) = attached {
            // Keep `current` set: the watchdog retries (e.g. while the desktop is restarting).
            destroy_surfaces(app);
            return Err(e);
        }
    }
    broadcast_live(app);
    Ok(())
}

/// Stop playing but keep the choice, so the next launch restores it. Used when Lumen exits: only
/// an explicit "stop" should make the desktop forget its wallpaper.
pub fn shutdown_live(app: &AppHandle) {
    let was_live = !live_windows(app).is_empty();
    destroy_surfaces(app);
    if was_live {
        repaint_static(app);
    }
}

/// Ask the desktop to draw its wallpaper again after the player window disappears.
fn repaint_static(app: &AppHandle) {
    if let Some(current) = desktop::current_static_wallpaper() {
        let fit = app.state::<Arc<Database>>().settings().fit_mode;
        let _ = desktop::set_static_wallpaper(&current, &fit);
    }
}

/// Stop the live wallpaper. `refresh` repaints the static wallpaper underneath.
pub fn stop_live(app: &AppHandle, refresh: bool) {
    let state = app.state::<LiveState>();
    let was_live = state.current().is_some() || !live_windows(app).is_empty();
    state.set_current(None);
    state.paused.store(false, Ordering::Relaxed);
    state.manual_paused.store(false, Ordering::Relaxed);
    state.auto_paused.store(false, Ordering::Relaxed);
    let _ = app.state::<Arc<Database>>().delete_setting(LIVE_SETTING_KEY);
    destroy_surfaces(app);
    if was_live {
        broadcast_live(app);
    }
    if refresh && was_live {
        repaint_static(app);
    }
}

/// Re-applies the live wallpaper that was active when Lumen last exited.
pub fn restore_live(app: &AppHandle) {
    let db = app.state::<Arc<Database>>();
    let Ok(Some(path)) = db.get_setting(LIVE_SETTING_KEY) else {
        return;
    };
    // Never drop the saved wallpaper here: at logon the desktop, a cloud folder or an external
    // drive may simply not be ready yet, and the watchdog keeps trying.
    match set_live(app, &path) {
        Ok(()) => log::info!("live wallpaper started: {}", path),
        Err(e) => log::warn!("live wallpaper not started yet ({}): {}", path, e),
    }
}

/// Background thread that pauses the live wallpaper while a fullscreen app runs or on battery.
/// It is also the live wallpaper's watchdog: a restarted desktop shell or a display change can
/// destroy the player windows, so they are re-created, re-attached and re-fitted as needed — a live
/// wallpaper only stops when the user stops it.
pub fn spawn_pause_monitor(app: AppHandle) {
    if !desktop::capabilities().live_wallpaper {
        return; // Nothing to watch over, and nothing worth waking up for every two seconds.
    }
    std::thread::spawn(move || {
        let mut missing_ticks = 0u32;
        let mut idle_ticks = 0u32;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let state = app.state::<LiveState>();
            let Some(path) = state.current() else {
                missing_ticks = 0;
                idle_ticks += 1;
                // A wallpaper saved earlier may not have started yet (at logon the desktop or a
                // cloud folder can lag behind), so keep trying every ~10s.
                if idle_ticks.is_multiple_of(5) {
                    restore_live(&app);
                }
                continue;
            };
            idle_ticks = 0;

            let specs = desktop::plan_surfaces(&app);
            let missing = specs.iter().any(|s| app.get_webview_window(&s.label).is_none());
            if missing {
                missing_ticks += 1;
                // Skip one tick (a window may be mid-creation), then retry every ~10s.
                if missing_ticks == 2 || missing_ticks.is_multiple_of(5) {
                    match set_live(&app, &path) {
                        Ok(()) => log::info!("Live wallpaper restored after the desktop was rebuilt"),
                        Err(e) => log::warn!("Live wallpaper watchdog: {}", e),
                    }
                }
                continue;
            }
            missing_ticks = 0;
            for spec in specs {
                if let Some(window) = app.get_webview_window(&spec.label) {
                    desktop::on_main_async(&app, move || desktop::refit(&window, &spec));
                }
            }
            pause_tick(&app);
        }
    });
}

fn pause_tick(app: &AppHandle) {
    let caps = desktop::capabilities();
    let settings = app.state::<Arc<Database>>().settings();
    let auto = (caps.pause_on_fullscreen && settings.pause_on_fullscreen && desktop::fullscreen_app_active())
        || (caps.pause_on_battery && settings.pause_on_battery && desktop::on_battery());
    app.state::<LiveState>().auto_paused.store(auto, Ordering::Relaxed);
    if sync_pause(app) {
        broadcast_live(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::desktop::SurfaceSpec;

    fn info(id: &str, url: &str, media: MediaType) -> WallpaperInfo {
        let mut w = WallpaperInfo::from_local("x");
        w.id = id.into();
        w.url = url.into();
        w.media_type = media;
        w
    }

    #[test]
    fn download_filenames_are_safe_and_keep_extensions() {
        assert_eq!(
            download_filename(&info(
                "wallhaven_6lyyvx",
                "https://w.wallhaven.cc/full/6l/wallhaven-6lyyvx.png",
                MediaType::Image
            )),
            "wallhaven_6lyyvx.png"
        );
        assert_eq!(
            download_filename(&info(
                "unsplash_ab:c",
                "https://images.unsplash.com/photo-1?ixid=1&fm=jpg",
                MediaType::Image
            )),
            "unsplash_ab_c.jpg"
        );
        assert_eq!(
            download_filename(&info(
                "commons_1",
                "https://upload.wikimedia.org/a/b/X.webm.1080p.vp9.webm",
                MediaType::Video
            )),
            "commons_1.webm"
        );
        assert_eq!(
            download_filename(&info(
                "pexelsvideo_9",
                "https://videos.pexels.com/video-files/9/file?x=1",
                MediaType::Video
            )),
            "pexelsvideo_9.mp4"
        );
        assert_eq!(
            download_filename(&info(
                "nasa_PIA 1/2",
                "https://images-assets.nasa.gov/image/x/x~orig.jpg",
                MediaType::Image
            )),
            "nasa_PIA_1_2.jpg"
        );
    }

    #[test]
    fn extra_surfaces_are_labelled_predictably() {
        assert_eq!(SurfaceSpec::label_for(0), LIVE_LABEL);
        assert_eq!(SurfaceSpec::label_for(2), format!("{}_2", LIVE_LABEL));
    }
}
