use crate::models::*;
use crate::services::api_client::ApiClient;
use crate::services::database::Database;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

pub const LIVE_LABEL: &str = "live_wallpaper";
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
        let _ = app.emit_to(LIVE_LABEL, "live-pause", pause);
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
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .take(80)
        .collect();
    let url_path = info.url.split(['?', '#']).next().unwrap_or("");
    let ext = url_path
        .rsplit('/')
        .next()
        .and_then(|name| name.rsplit_once('.').map(|(_, e)| e.to_lowercase()))
        .filter(|e| crate::utils::is_wallpaper_extension(e))
        .unwrap_or_else(|| if info.media_type == MediaType::Video { "mp4".into() } else { "jpg".into() });
    format!("{}.{}", stem, ext)
}

pub fn set_static(app: &AppHandle, path: &str, fit: &FitMode) -> Result<(), String> {
    stop_live(app, false);
    let _ = ::wallpaper::set_mode(fit.to_mode());
    ::wallpaper::set_from_path(path).map_err(|e| format!("Failed to set wallpaper: {}", e))
}

pub fn set_live(app: &AppHandle, path: &str) -> Result<(), String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, path);
        return Err("Live wallpapers are currently only supported on Windows".into());
    }

    #[cfg(target_os = "windows")]
    {
        use tauri::{WebviewUrl, WebviewWindowBuilder};

        let state = app.state::<LiveState>();
        let db = app.state::<Arc<Database>>();
        app.asset_protocol_scope().allow_file(path).map_err(|e| e.to_string())?;
        state.set_current(Some(path.to_string()));
        state.manual_paused.store(false, Ordering::Relaxed);
        sync_pause(app);
        let _ = db.set_setting(LIVE_SETTING_KEY, path);

        if app.get_webview_window(LIVE_LABEL).is_some() {
            app.emit_to(LIVE_LABEL, "live-src", path).map_err(|e| e.to_string())?;
            broadcast_live(app);
            return Ok(());
        }

        let window = WebviewWindowBuilder::new(app, LIVE_LABEL, WebviewUrl::App("index.html".into()))
            .title("Lumen Live")
            .decorations(false)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .visible(false)
            .build()
            .map_err(|e| format!("Could not create live wallpaper window: {}", e))?;

        // `attach` also shows the window. Tauri's own `show()` is deliberately not used: it re-applies
        // Tauri's window styles (dropping WS_CHILD / WS_EX_LAYERED) and activates the window, which would
        // pull focus out of a fullscreen game when a slideshow or startup restore sets a live wallpaper.
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;
        if let Err(e) = unsafe { desktop::attach(hwnd.0 as isize) } {
            // Keep `current` set: the watchdog retries (e.g. while Explorer is restarting).
            let _ = window.destroy();
            return Err(e);
        }
        broadcast_live(app);
        Ok(())
    }
}

/// Stop the live wallpaper. `refresh` repaints the static wallpaper underneath.
pub fn stop_live(app: &AppHandle, refresh: bool) {
    let state = app.state::<LiveState>();
    let was_live = state.current().is_some() || app.get_webview_window(LIVE_LABEL).is_some();
    state.set_current(None);
    state.paused.store(false, Ordering::Relaxed);
    state.manual_paused.store(false, Ordering::Relaxed);
    state.auto_paused.store(false, Ordering::Relaxed);
    let _ = app.state::<Arc<Database>>().delete_setting(LIVE_SETTING_KEY);
    if let Some(window) = app.get_webview_window(LIVE_LABEL) {
        let _ = window.destroy();
    }
    if was_live {
        broadcast_live(app);
    }
    if refresh && was_live {
        if let Ok(current) = ::wallpaper::get() {
            if !current.is_empty() {
                let _ = ::wallpaper::set_from_path(&current);
            }
        }
    }
}

/// Re-applies the live wallpaper that was active when the app last exited.
pub fn restore_live(app: &AppHandle) {
    let db = app.state::<Arc<Database>>();
    if let Ok(Some(path)) = db.get_setting(LIVE_SETTING_KEY) {
        if Path::new(&path).is_file() {
            if let Err(e) = set_live(app, &path) {
                log::warn!("Could not restore live wallpaper: {}", e);
            }
        } else {
            let _ = db.delete_setting(LIVE_SETTING_KEY);
        }
    }
}

/// Background thread that pauses the live wallpaper while a fullscreen app runs or on battery.
/// It is also the live wallpaper's watchdog: Explorer restarts and display changes can destroy the
/// desktop windows (taking the player with them), so the player is re-created, re-attached and
/// re-fitted as needed — a live wallpaper only stops when the user stops it.
pub fn spawn_pause_monitor(app: AppHandle) {
    std::thread::spawn(move || {
        let mut missing_ticks = 0u32;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let state = app.state::<LiveState>();
            let Some(path) = state.current() else {
                missing_ticks = 0;
                continue;
            };
            match app.get_webview_window(LIVE_LABEL) {
                None => {
                    missing_ticks += 1;
                    // Skip one tick (the window may be mid-creation), then retry every ~10s.
                    if missing_ticks == 2 || missing_ticks % 5 == 0 {
                        match set_live(&app, &path) {
                            Ok(()) => log::info!("Live wallpaper restored after the desktop was rebuilt"),
                            Err(e) => log::warn!("Live wallpaper watchdog: {}", e),
                        }
                    }
                    continue;
                }
                Some(_window) => {
                    missing_ticks = 0;
                    #[cfg(target_os = "windows")]
                    if let Ok(hwnd) = _window.hwnd() {
                        unsafe { desktop::keep_fitted(hwnd.0 as isize) };
                    }
                }
            }
            pause_tick(&app);
        }
    });
}

fn pause_tick(app: &AppHandle) {
    let state = app.state::<LiveState>();
    {
        let settings = app.state::<Arc<Database>>().settings();
        let auto = (settings.pause_on_fullscreen && desktop::fullscreen_app_active())
            || (settings.pause_on_battery && desktop::on_battery());
        state.auto_paused.store(auto, Ordering::Relaxed);
        if sync_pause(app) {
            broadcast_live(app);
        }
    }
}

#[cfg(target_os = "windows")]
pub fn set_autostart(enable: bool) -> Result<(), String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;
    let run = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_SET_VALUE)
        .map_err(|e| e.to_string())?;
    // Entry left behind by the app's previous name.
    let _ = run.delete_value("Vividwall");
    if enable {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        run.set_value("Lumen", &format!("\"{}\" --minimized", exe.display()))
            .map_err(|e| e.to_string())
    } else {
        match run.delete_value("Lumen") {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_autostart(_enable: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
mod desktop {
    use std::ffi::c_void;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{BOOL, COLORREF, FALSE, HWND, LPARAM, RECT, TRUE, WPARAM};
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    use windows::Win32::UI::WindowsAndMessaging::*;

    /// Parent `raw` behind the desktop icons so it renders as the wallpaper.
    ///
    /// Windows 11 24H2+ keeps the icon view (SHELLDLL_DefView) and the wallpaper WorkerW side by side
    /// inside Progman, so the window becomes a child of that WorkerW. (A layered sibling of the icons
    /// is only a fallback: WebView2 renders black inside layered windows.) Older Windows moves the
    /// icons into a new WorkerW, and the wallpaper goes into the WorkerW behind it.
    pub unsafe fn attach(raw: isize) -> Result<(), String> {
        let hwnd = HWND(raw as *mut c_void);
        let progman = FindWindowW(w!("Progman"), PCWSTR::null())
            .map_err(|_| "Could not find the desktop window (is Explorer running?)".to_string())?;

        // Ask Progman to create the WorkerW that sits behind the icons.
        let mut result = 0usize;
        let _ = SendMessageTimeoutW(progman, 0x052C, WPARAM(0xD), LPARAM(0x1), SMTO_NORMAL, 1000, Some(&mut result));

        let cx = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let cy = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let remove = (WS_POPUP | WS_CAPTION | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX).0 as isize;
        SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !remove) | WS_CHILD.0 as isize);

        let raised_desktop = FindWindowExW(progman, HWND::default(), w!("SHELLDLL_DefView"), PCWSTR::null());
        let raised_worker = FindWindowExW(progman, HWND::default(), w!("WorkerW"), PCWSTR::null());
        if let (Ok(_), Ok(worker)) = (&raised_desktop, &raised_worker) {
            // Windows 11 24H2+: become a child of the WorkerW that paints the wallpaper. It sits
            // below the icons, and needs no WS_EX_LAYERED (WebView2 renders black in layered windows).
            SetParent(hwnd, *worker).map_err(|e| format!("SetParent failed: {}", e))?;
            let _ = SetWindowPos(hwnd, HWND::default(), 0, 0, cx, cy, SWP_NOACTIVATE);
        } else if let Ok(icons) = raised_desktop {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_LAYERED.0 as isize);
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
            SetParent(hwnd, progman).map_err(|e| format!("SetParent failed: {}", e))?;
            // Directly below the icon view...
            let _ = SetWindowPos(hwnd, icons, 0, 0, cx, cy, SWP_NOACTIVATE);
            // ...and above the WorkerW that paints the static wallpaper.
            if let Ok(worker) = FindWindowExW(progman, HWND::default(), w!("WorkerW"), PCWSTR::null()) {
                let _ = SetWindowPos(worker, hwnd, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        } else {
            let mut worker = HWND::default();
            let _ = EnumWindows(Some(find_worker), LPARAM(&mut worker as *mut HWND as isize));
            if worker.0.is_null() {
                return Err("Could not find the desktop WorkerW window".into());
            }
            SetParent(hwnd, worker).map_err(|e| format!("SetParent failed: {}", e))?;
            let _ = SetWindowPos(hwnd, HWND::default(), 0, 0, cx, cy, SWP_NOACTIVATE);
        }
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        Ok(())
    }

    /// Re-attaches the player if it lost its desktop parent, and keeps it covering every monitor
    /// (the virtual screen changes with resolution or monitor changes).
    pub unsafe fn keep_fitted(raw: isize) {
        let hwnd = HWND(raw as *mut c_void);
        let attached = matches!(GetParent(hwnd), Ok(p) if !p.0.is_null() && IsWindow(p).as_bool());
        if !attached {
            let _ = attach(raw);
            return;
        }
        let cx = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let cy = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        let mut rect = RECT::default();
        if GetClientRect(hwnd, &mut rect).is_ok() && (rect.right != cx || rect.bottom != cy) {
            let _ = SetWindowPos(hwnd, HWND::default(), 0, 0, cx, cy, SWP_NOACTIVATE | SWP_NOZORDER);
        }
    }

    /// Finds the WorkerW that follows the top-level window hosting SHELLDLL_DefView.
    unsafe extern "system" fn find_worker(top: HWND, out: LPARAM) -> BOOL {
        if FindWindowExW(top, HWND::default(), w!("SHELLDLL_DefView"), PCWSTR::null()).is_ok() {
            if let Ok(worker) = FindWindowExW(HWND::default(), top, w!("WorkerW"), PCWSTR::null()) {
                *(out.0 as *mut HWND) = worker;
                return FALSE;
            }
        }
        TRUE
    }

    pub fn fullscreen_app_active() -> bool {
        unsafe {
            let fg = GetForegroundWindow();
            if fg.0.is_null() {
                return false;
            }
            let mut class = [0u16; 64];
            let len = GetClassNameW(fg, &mut class).max(0) as usize;
            let class = String::from_utf16_lossy(&class[..len]);
            if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
                return false;
            }
            let mut rect = RECT::default();
            if GetWindowRect(fg, &mut rect).is_err() {
                return false;
            }
            let monitor = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            if !GetMonitorInfoW(monitor, &mut info).as_bool() {
                return false;
            }
            let m = info.rcMonitor;
            rect.left <= m.left && rect.top <= m.top && rect.right >= m.right && rect.bottom >= m.bottom
        }
    }

    pub fn on_battery() -> bool {
        let mut status = SYSTEM_POWER_STATUS::default();
        unsafe { GetSystemPowerStatus(&mut status).is_ok() && status.ACLineStatus == 0 }
    }
}

#[cfg(not(target_os = "windows"))]
mod desktop {
    pub fn fullscreen_app_active() -> bool {
        false
    }
    pub fn on_battery() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            download_filename(&info("wallhaven_6lyyvx", "https://w.wallhaven.cc/full/6l/wallhaven-6lyyvx.png", MediaType::Image)),
            "wallhaven_6lyyvx.png"
        );
        assert_eq!(
            download_filename(&info("unsplash_ab:c", "https://images.unsplash.com/photo-1?ixid=1&fm=jpg", MediaType::Image)),
            "unsplash_ab_c.jpg"
        );
        assert_eq!(
            download_filename(&info("commons_1", "https://upload.wikimedia.org/a/b/X.webm.1080p.vp9.webm", MediaType::Video)),
            "commons_1.webm"
        );
        assert_eq!(
            download_filename(&info("pexelsvideo_9", "https://videos.pexels.com/video-files/9/file?x=1", MediaType::Video)),
            "pexelsvideo_9.mp4"
        );
        assert_eq!(
            download_filename(&info("nasa_PIA 1/2", "https://images-assets.nasa.gov/image/x/x~orig.jpg", MediaType::Image)),
            "nasa_PIA_1_2.jpg"
        );
    }
}
