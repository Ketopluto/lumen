use crate::models::*;
use crate::services::api_client::ApiClient;
use crate::services::database::Database;
use crate::services::desktop;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub use crate::services::desktop::LIVE_LABEL;

/// Legacy: one path for the whole desktop. Still read once, so an upgrade keeps playing.
const LIVE_SETTING_KEY: &str = "live_wallpaper";
/// The per-monitor layout, as JSON.
const LIVE_LAYOUT_KEY: &str = "live_wallpaper_layout";

/// Which file plays on which monitor.
///
/// Either `all` holds one file and every monitor shows it, or `all` is empty and `per_monitor`
/// names each monitor explicitly. Assigning a single monitor expands the first form into the
/// second, so no monitor is ever left ambiguous. The distinction is what keeps the common case
/// cheap: one file everywhere means one spanning surface and one video decoding, however many
/// monitors are attached.
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Assignment {
    #[serde(default)]
    all: Option<String>,
    #[serde(default)]
    per_monitor: BTreeMap<usize, String>,
}

impl Assignment {
    fn is_empty(&self) -> bool {
        self.all.is_none() && self.per_monitor.is_empty()
    }

    /// Something to show in the UI when it only has room for one wallpaper.
    fn any_path(&self) -> Option<String> {
        self.all.clone().or_else(|| self.per_monitor.values().next().cloned())
    }

    fn files(&self) -> BTreeSet<String> {
        self.all.iter().chain(self.per_monitor.values()).cloned().collect()
    }

    fn set_all(&mut self, path: String) {
        self.all = Some(path);
        self.per_monitor.clear();
    }

    /// Turns "the same everywhere" into an explicit entry per monitor, so one of them can differ.
    fn expand(&mut self, monitors: &[usize]) {
        if let Some(path) = self.all.take() {
            for monitor in monitors {
                self.per_monitor.insert(*monitor, path.clone());
            }
        }
    }

    fn set_monitor(&mut self, monitor: usize, path: String, monitors: &[usize]) {
        self.expand(monitors);
        self.per_monitor.insert(monitor, path);
    }

    fn clear_monitor(&mut self, monitor: usize, monitors: &[usize]) {
        self.expand(monitors);
        self.per_monitor.remove(&monitor);
    }
}

/// State of the live (video/GIF) wallpaper window.
#[derive(Default)]
pub struct LiveState {
    assignment: Mutex<Assignment>,
    /// Paused because a fullscreen app is running or the laptop is on battery.
    auto_paused: AtomicBool,
    /// Paused by the user from the app or the tray.
    manual_paused: AtomicBool,
    /// What the player was last told (auto || manual).
    paused: AtomicBool,
}

#[derive(serde::Serialize, Clone)]
pub struct LiveStatus {
    /// Whatever is playing, for the places that show a single wallpaper.
    pub path: Option<String>,
    /// Monitor index to file, when the monitors are showing different wallpapers.
    pub per_monitor: BTreeMap<usize, String>,
    pub paused: bool,
    pub manual_paused: bool,
    pub volume: u32,
}

impl LiveState {
    fn assignment(&self) -> Assignment {
        self.assignment.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    fn set_assignment(&self, value: Assignment) {
        *self.assignment.lock().unwrap_or_else(|p| p.into_inner()) = value;
    }

    pub fn status(&self, db: &Database) -> LiveStatus {
        let assignment = self.assignment();
        LiveStatus {
            path: assignment.any_path(),
            per_monitor: assignment.per_monitor.clone(),
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
pub async fn apply(
    app: &AppHandle,
    db: &Arc<Database>,
    info: &WallpaperInfo,
    fit: &FitMode,
    monitor: Option<usize>,
) -> Result<String, String> {
    let path = resolve_local(app, db, info).await?;
    if MediaType::from_path(&path).is_live() || info.media_type == MediaType::Video {
        set_live(app, &path, monitor)?;
    } else {
        set_static(app, &path, fit, monitor)?;
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

/// `monitor` of `None` dresses the whole desktop and stops any live wallpaper. A single monitor
/// replaces only that one and leaves the others playing.
pub fn set_static(app: &AppHandle, path: &str, fit: &FitMode, monitor: Option<usize>) -> Result<(), String> {
    let Some(index) = monitor else {
        stop_live(app, false);
        return desktop::set_static_wallpaper(path, fit, None);
    };
    if !desktop::capabilities().per_monitor_static {
        return Err("This desktop keeps one wallpaper for every monitor".into());
    }
    let target = desktop::monitors(app)
        .into_iter()
        .find(|monitor| monitor.index == index)
        .ok_or("That monitor is no longer connected")?;
    // Only this monitor stops playing; anything live elsewhere carries on.
    let _ = clear_monitor(app, index);
    desktop::set_static_wallpaper(path, fit, Some(target))
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
    Ok(dest.to_string_lossy().to_string())
}

/// Drops staged files no monitor is playing any more. Monitors can hold different wallpapers, so
/// "keep only the newest" would delete a file another monitor is still showing.
fn prune_staged(keep: &BTreeSet<String>) {
    let dir = crate::utils::app_data_dir().join("live");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !keep.contains(path.to_string_lossy().as_ref()) {
            let _ = std::fs::remove_file(path);
        }
    }
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
    sweep_orphan_surfaces(app);
}

/// Closes any surface the desktop is still showing that Tauri no longer lists.
///
/// This is how a black desktop happens: the shell owns a live surface once it is re-parented, so
/// when the webview inside dies — or the monitor it was pinned to is unplugged — Tauri forgets the
/// window while the shell keeps drawing the empty frame over the wallpaper. `destroy_surfaces`
/// only ever reached the windows Tauri still knew about, so that frame stayed until the next
/// logon. Whatever is swept here was showing nothing, so the wallpaper underneath is repainted.
#[cfg(target_os = "windows")]
fn sweep_orphan_surfaces(app: &AppHandle) {
    let keep: Vec<isize> = live_windows(app)
        .iter()
        .filter_map(|window| window.hwnd().ok())
        .map(|hwnd| hwnd.0 as isize)
        .collect();
    let app = app.clone();
    desktop::on_main_async(&app.clone(), move || {
        let closed = desktop::sweep_orphan_surfaces(&keep);
        if closed.is_empty() {
            return;
        }
        log::warn!(
            "closed {} live surface(s) the desktop was still showing after Tauri lost them",
            closed.len()
        );
        redraw(&app);
    });
}

#[cfg(not(target_os = "windows"))]
fn sweep_orphan_surfaces(_app: &AppHandle) {}

/// Destroys the surfaces and waits for their labels to be free again.
///
/// `destroy` is asynchronous: the window is gone but Tauri still holds the label for a moment, and
/// building a surface with a label that is still registered fails outright. Rebuilding immediately
/// is exactly what happens when a new wallpaper replaces a playing one, which is why that used to
/// need stopping the old one first.
fn destroy_surfaces_and_wait(app: &AppHandle) {
    destroy_surfaces(app);
    for _ in 0..100 {
        if live_windows(app).is_empty() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    log::warn!("live wallpaper windows did not close in time; the next surface may fail to build");
}

/// Plays `path` on one monitor, or on every monitor when `monitor` is `None`.
pub fn set_live(app: &AppHandle, path: &str, monitor: Option<usize>) -> Result<(), String> {
    let caps = desktop::capabilities();
    if !caps.live_wallpaper {
        return Err(caps
            .live_unsupported_reason
            .clone()
            .unwrap_or_else(|| "Live wallpapers aren't supported on this desktop".into()));
    }
    if monitor.is_some() && !caps.per_monitor_live {
        return Err("This desktop cannot play a separate wallpaper on each monitor".into());
    }

    let db = app.state::<Arc<Database>>();
    if !Path::new(path).is_file() {
        return Err("That file is not available right now".into());
    }
    crate::services::media::check_playable(&db, path)?;
    let staged = stage_live_file(path)?;
    app.asset_protocol_scope()
        .allow_file(&staged)
        .map_err(|e| e.to_string())?;

    let state = app.state::<LiveState>();
    let mut assignment = state.assignment();
    match monitor {
        Some(index) => assignment.set_monitor(index, staged, &monitor_indices(app)),
        None => assignment.set_all(staged),
    }
    state.set_assignment(assignment);
    state.manual_paused.store(false, Ordering::Relaxed);
    sync_pause(app);
    save_assignment(app);
    apply_live(app)
}

/// Re-applies the live wallpaper after the fit mode changed. Does nothing when none is playing.
pub fn refresh_fit(app: &AppHandle) {
    if app.state::<LiveState>().assignment().is_empty() {
        return;
    }
    if let Err(e) = apply_live(app) {
        log::warn!("could not re-apply the live wallpaper after a fit change: {}", e);
    }
}

/// Stops the wallpaper on one monitor and leaves the others playing.
pub fn clear_monitor(app: &AppHandle, monitor: usize) -> Result<(), String> {
    let state = app.state::<LiveState>();
    let mut assignment = state.assignment();
    assignment.clear_monitor(monitor, &monitor_indices(app));
    let emptied = assignment.is_empty();
    state.set_assignment(assignment);
    save_assignment(app);
    if emptied {
        stop_live(app, true);
        return Ok(());
    }
    let result = apply_live(app);
    // The monitor that went quiet has to show its still wallpaper again.
    redraw(app);
    result
}

fn fit_mode(app: &AppHandle) -> FitMode {
    app.state::<Arc<Database>>().settings().fit_mode
}

fn monitor_indices(app: &AppHandle) -> Vec<usize> {
    desktop::monitors(app)
        .into_iter()
        .map(|monitor| monitor.index)
        .collect()
}

fn save_assignment(app: &AppHandle) {
    let db = app.state::<Arc<Database>>();
    let assignment = app.state::<LiveState>().assignment();
    if assignment.is_empty() {
        let _ = db.delete_setting(LIVE_LAYOUT_KEY);
        let _ = db.delete_setting(LIVE_SETTING_KEY);
    } else {
        if let Ok(json) = serde_json::to_string(&assignment) {
            let _ = db.set_setting(LIVE_LAYOUT_KEY, &json);
        }
        // Leave the old single-wallpaper key pointing at something playable, so going back to an
        // older Lumen still finds a wallpaper rather than none.
        if let Some(path) = assignment.any_path() {
            let _ = db.set_setting(LIVE_SETTING_KEY, &path);
        }
    }
    prune_staged(&assignment.files());
}

/// The windows this assignment needs, each with the file it plays. One file everywhere keeps the
/// platform default — a single spanning surface on Windows and X11 — and only a genuine mix of
/// wallpapers pays for a webview per monitor.
fn plan_live(app: &AppHandle, assignment: &Assignment) -> Vec<(desktop::SurfaceSpec, String)> {
    if let Some(path) = &assignment.all {
        // One surface over the whole desktop means one video stretched across every screen, which
        // is only right when the fit mode actually asks to span: on two monitors a 16:9 video
        // covering 32:9 shows its middle third at double size. Any other fit gives each monitor
        // its own surface, so the video fits that screen — at the cost of decoding per monitor.
        let specs = if fit_mode(app) == FitMode::Span {
            desktop::plan_surfaces(app)
        } else {
            let monitors: Vec<usize> = desktop::monitors(app).into_iter().map(|m| m.index).collect();
            let per_monitor = desktop::surfaces_for(app, &monitors);
            // A desktop that cannot enumerate its monitors still gets its wallpaper.
            if per_monitor.is_empty() {
                desktop::plan_surfaces(app)
            } else {
                per_monitor
            }
        };
        return specs.into_iter().map(|spec| (spec, path.clone())).collect();
    }
    let monitors: Vec<usize> = assignment.per_monitor.keys().copied().collect();
    desktop::surfaces_for(app, &monitors)
        .into_iter()
        .filter_map(|spec| {
            let path = assignment.per_monitor.get(&spec.monitor?)?.clone();
            Some((spec, path))
        })
        .collect()
}

/// Brings the live windows in line with the assignment. The existing windows are reused when the
/// set of them is already right, so changing one monitor never restarts the others.
fn apply_live(app: &AppHandle) -> Result<(), String> {
    let assignment = app.state::<LiveState>().assignment();
    if assignment.is_empty() {
        destroy_surfaces(app);
        broadcast_live(app);
        return Ok(());
    }
    // Every file any monitor plays has to be readable through the asset protocol. Doing it here
    // rather than only where a wallpaper is chosen covers the paths that do not go through a
    // choice at all: restoring the saved layout at startup, and the watchdog rebuilding surfaces.
    for file in assignment.files() {
        let _ = app.asset_protocol_scope().allow_file(&file);
    }
    let planned = plan_live(app, &assignment);
    if planned.is_empty() {
        // Nothing could be planned, usually because the monitors could not be enumerated for a
        // moment. Whatever is on screen is not what the assignment asks for, so it must not be
        // left playing: a wallpaper spanning both monitors outliving a per-monitor choice is
        // exactly how a stale surface looks like a zoomed one.
        destroy_surfaces(app);
        redraw(app);
        return Err("No display to put a wallpaper on".into());
    }

    let wanted: BTreeSet<&str> = planned.iter().map(|(spec, _)| spec.label.as_str()).collect();
    let existing: BTreeSet<String> = live_windows(app)
        .iter()
        .map(|window| window.label().to_string())
        .collect();
    let reusable = wanted.len() == existing.len() && wanted.iter().all(|label| existing.contains(*label));

    if !reusable {
        destroy_surfaces_and_wait(app);
        for (spec, path) in &planned {
            if let Err(e) = build_surface(app, spec, path) {
                // Keep the assignment: the watchdog retries, e.g. while the desktop restarts.
                destroy_surfaces(app);
                return Err(e);
            }
        }
    }
    let fit = fit_mode(app);
    for (spec, path) in &planned {
        app.emit_to(spec.label.as_str(), "live-src", path.as_str())
            .map_err(|e| e.to_string())?;
        let _ = app.emit_to(spec.label.as_str(), "live-fit", &fit);
    }
    broadcast_live(app);
    Ok(())
}

fn build_surface(app: &AppHandle, spec: &desktop::SurfaceSpec, path: &str) -> Result<(), String> {
    let mut builder = WebviewWindowBuilder::new(app, &spec.label, WebviewUrl::App("index.html".into()))
        .title("Lumen Live")
        .decorations(false)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .visible(false)
        // Told which file to play before its first paint, so a surface never flashes up the
        // wallpaper belonging to another monitor.
        .initialization_script(desktop::surface_script(Some(path), &fit_mode(app)));
    if let Some((x, y, width, height)) = spec.bounds {
        builder = builder
            .position(x as f64, y as f64)
            .inner_size(width as f64, height as f64);
    }
    let window = builder
        .build()
        .map_err(|e| format!("Could not create live wallpaper window: {}", e))?;

    // `attach` also reveals the window — see the contract in services/desktop/mod.rs.
    let spec = spec.clone();
    desktop::on_main(app, move || desktop::attach(&window, &spec))?
}

/// Stop playing but keep the choice, so the next launch restores it. Used when Lumen exits: only
/// an explicit "stop" should make the desktop forget its wallpaper.
pub fn shutdown_live(app: &AppHandle) {
    let was_live = !live_windows(app).is_empty();
    destroy_surfaces(app);
    if was_live {
        redraw(app);
    }
}

/// Ask the desktop to draw its wallpaper again after a player window disappears.
fn redraw(app: &AppHandle) {
    let fit = app.state::<Arc<Database>>().settings().fit_mode;
    desktop::redraw_wallpaper(&fit);
}

/// Stop the live wallpaper. `refresh` repaints the static wallpaper underneath.
pub fn stop_live(app: &AppHandle, refresh: bool) {
    let state = app.state::<LiveState>();
    let was_live = !state.assignment().is_empty() || !live_windows(app).is_empty();
    state.set_assignment(Assignment::default());
    state.paused.store(false, Ordering::Relaxed);
    state.manual_paused.store(false, Ordering::Relaxed);
    state.auto_paused.store(false, Ordering::Relaxed);
    save_assignment(app);
    destroy_surfaces(app);
    if was_live {
        broadcast_live(app);
    }
    if refresh && was_live {
        redraw(app);
    }
}

/// Re-applies the live wallpaper that was active when Lumen last exited.
pub fn restore_live(app: &AppHandle) {
    let db = app.state::<Arc<Database>>();
    let stored = db
        .get_setting(LIVE_LAYOUT_KEY)
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str::<Assignment>(&json).ok())
        .or_else(|| {
            // Upgraded from a Lumen that knew only one wallpaper for the whole desktop.
            let path = db.get_setting(LIVE_SETTING_KEY).ok().flatten()?;
            Some(Assignment {
                all: Some(path),
                per_monitor: BTreeMap::new(),
            })
        });
    let Some(assignment) = stored.filter(|assignment| !assignment.is_empty()) else {
        return;
    };
    app.state::<LiveState>().set_assignment(assignment.clone());
    // Never drop the saved wallpaper here: at logon the desktop, a cloud folder or an external
    // drive may simply not be ready yet, and the watchdog keeps trying.
    match apply_live(app) {
        Ok(()) => log::info!("live wallpaper started: {:?}", assignment.any_path()),
        Err(e) => log::warn!("live wallpaper not started yet: {}", e),
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
            let assignment = state.assignment();
            if assignment.is_empty() {
                missing_ticks = 0;
                idle_ticks += 1;
                // A wallpaper saved earlier may not have started yet (at logon the desktop or a
                // cloud folder can lag behind), so keep trying every ~10s. The same tick sweeps up
                // a surface the desktop is still showing after Tauri lost it, which is the one
                // case nothing else would ever notice: no assignment, no windows, black desktop.
                if idle_ticks.is_multiple_of(5) {
                    sweep_orphan_surfaces(&app);
                    restore_live(&app);
                }
                continue;
            }
            idle_ticks = 0;

            let planned = plan_live(&app, &assignment);
            let missing = planned
                .iter()
                .any(|(spec, _)| app.get_webview_window(&spec.label).is_none());
            if missing {
                missing_ticks += 1;
                // Skip one tick (a window may be mid-creation), then retry every ~10s.
                if missing_ticks == 2 || missing_ticks.is_multiple_of(5) {
                    match apply_live(&app) {
                        Ok(()) => log::info!("Live wallpaper restored after the desktop was rebuilt"),
                        Err(e) => log::warn!("Live wallpaper watchdog: {}", e),
                    }
                }
                continue;
            }
            missing_ticks = 0;
            for (spec, _) in planned {
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
mod assignment_tests {
    use super::*;

    const MONITORS: [usize; 2] = [0, 1];

    #[test]
    fn one_file_everywhere_stays_a_single_span() {
        let mut assignment = Assignment::default();
        assignment.set_all("a.mp4".into());
        // `all` set and nothing per-monitor is what lets the planner use one spanning surface.
        assert_eq!(assignment.all.as_deref(), Some("a.mp4"));
        assert!(assignment.per_monitor.is_empty());
        assert_eq!(assignment.files().len(), 1);
    }

    #[test]
    fn assigning_one_monitor_pins_the_others_to_what_they_already_showed() {
        let mut assignment = Assignment::default();
        assignment.set_all("a.mp4".into());
        assignment.set_monitor(1, "b.mp4".into(), &MONITORS);
        // The second monitor changed; the first keeps playing what it had, explicitly now.
        assert_eq!(assignment.all, None);
        assert_eq!(assignment.per_monitor.get(&0).map(String::as_str), Some("a.mp4"));
        assert_eq!(assignment.per_monitor.get(&1).map(String::as_str), Some("b.mp4"));
        assert_eq!(assignment.files().len(), 2);
    }

    #[test]
    fn clearing_one_monitor_leaves_the_rest_playing() {
        let mut assignment = Assignment::default();
        assignment.set_all("a.mp4".into());
        assignment.clear_monitor(0, &MONITORS);
        assert!(!assignment.is_empty(), "the other monitor still has a wallpaper");
        assert_eq!(assignment.per_monitor.get(&0), None);
        assert_eq!(assignment.per_monitor.get(&1).map(String::as_str), Some("a.mp4"));
    }

    #[test]
    fn clearing_the_last_monitor_empties_the_assignment() {
        let mut assignment = Assignment::default();
        assignment.set_all("a.mp4".into());
        for monitor in MONITORS {
            assignment.clear_monitor(monitor, &MONITORS);
        }
        assert!(assignment.is_empty());
        assert!(assignment.any_path().is_none());
    }

    #[test]
    fn setting_every_monitor_to_the_same_file_stages_it_once() {
        let mut assignment = Assignment::default();
        assignment.set_monitor(0, "a.mp4".into(), &MONITORS);
        assignment.set_monitor(1, "a.mp4".into(), &MONITORS);
        // Both monitors, one staged file to keep on disk.
        assert_eq!(assignment.files().len(), 1);
    }

    #[test]
    fn a_stored_layout_survives_the_round_trip() {
        let mut assignment = Assignment::default();
        assignment.set_monitor(1, "b.mp4".into(), &MONITORS);
        let json = serde_json::to_string(&assignment).unwrap();
        assert_eq!(serde_json::from_str::<Assignment>(&json).unwrap(), assignment);
        // And the shape an older Lumen wrote still reads as "the same everywhere".
        let legacy: Assignment = serde_json::from_str(r#"{"all":"a.mp4"}"#).unwrap();
        assert_eq!(legacy.all.as_deref(), Some("a.mp4"));
        assert!(legacy.per_monitor.is_empty());
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
