//! Everything that touches the desktop shell, one module per platform.
//!
//! # Contract for backends
//!
//! **`attach` owns visibility.** Live wallpaper windows are always built with `.visible(false)`,
//! and `attach` is the only code allowed to reveal one — without activating it, so setting a
//! wallpaper never pulls focus out of a fullscreen game. `wallpaper_engine` must never call
//! `WebviewWindow::show()` on a live surface: on Windows that re-applies Tauri's window styles and
//! drops `WS_CHILD`, detaching the wallpaper from the desktop. Each backend reveals its surface its
//! own way (`ShowWindow(SW_SHOWNOACTIVATE)`, `orderFrontRegardless`, `gtk_window.show_all()`).
//!
//! **`attach` and `refit` run on the main thread.** Both are called through [`on_main`], because
//! GTK and AppKit require it. Backends may assume main-thread context.

mod types;

pub use types::{Capabilities, SurfaceSpec};

use crate::models::FitMode;
use std::sync::OnceLock;
use tauri::{AppHandle, WebviewWindow};

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod imp;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod imp;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod imp;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
#[path = "unsupported.rs"]
mod imp;

/// Label of the first live wallpaper window; further surfaces append `_1`, `_2`, …
pub const LIVE_LABEL: &str = "live_wallpaper";

/// Every backend must expose exactly these signatures — a mismatch is a compile error here rather
/// than a surprise on someone else's operating system.
#[allow(dead_code)]
const _CONTRACT: () = {
    let _: fn() -> Capabilities = imp::capabilities;
    let _: fn(&AppHandle) -> Vec<SurfaceSpec> = imp::plan_surfaces;
    let _: fn(&WebviewWindow, &SurfaceSpec) -> Result<(), String> = imp::attach;
    let _: fn(&WebviewWindow, &SurfaceSpec) = imp::refit;
    let _: fn() -> bool = imp::fullscreen_app_active;
    let _: fn() -> bool = imp::on_battery;
    let _: fn(bool) -> Result<(), String> = imp::set_autostart;
    let _: fn(&str, &FitMode) -> Result<(), String> = imp::set_static_wallpaper;
    let _: fn() -> Option<String> = imp::current_static_wallpaper;
};

/// Computed once: reading the session type and monitor layout is not free.
pub fn capabilities() -> &'static Capabilities {
    static CAPS: OnceLock<Capabilities> = OnceLock::new();
    CAPS.get_or_init(imp::capabilities)
}

/// How many live wallpaper windows this desktop needs, and where they go.
pub fn plan_surfaces(app: &AppHandle) -> Vec<SurfaceSpec> {
    imp::plan_surfaces(app)
}

/// Places a live wallpaper window on the desktop and makes it visible. Main thread only.
pub fn attach(window: &WebviewWindow, spec: &SurfaceSpec) -> Result<(), String> {
    imp::attach(window, spec)
}

/// Re-asserts placement after the desktop, resolution or monitor layout changed. Main thread only.
pub fn refit(window: &WebviewWindow, spec: &SurfaceSpec) {
    imp::refit(window, spec)
}

pub fn fullscreen_app_active() -> bool {
    imp::fullscreen_app_active()
}

pub fn on_battery() -> bool {
    imp::on_battery()
}

pub fn set_autostart(enable: bool) -> Result<(), String> {
    imp::set_autostart(enable)
}

pub fn set_static_wallpaper(path: &str, fit: &FitMode) -> Result<(), String> {
    imp::set_static_wallpaper(path, fit)
}

/// The wallpaper the desktop is showing, where asking is cheap and prompt-free.
pub fn current_static_wallpaper() -> Option<String> {
    imp::current_static_wallpaper()
}

/// Runs `f` on the UI thread and waits for the result.
pub fn on_main<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let _ = tx.send(f());
    })
    .map_err(|e| e.to_string())?;
    // A timeout rather than a hang if the UI thread is wedged; the watchdog will try again.
    rx.recv_timeout(std::time::Duration::from_secs(10))
        .map_err(|_| "the desktop did not respond".to_string())
}

/// Fire-and-forget variant for work that can be retried on the next watchdog tick.
pub fn on_main_async(app: &AppHandle, f: impl FnOnce() + Send + 'static) {
    let _ = app.run_on_main_thread(f);
}

/// Injected into every window before its first paint, so the UI can adapt to the platform without
/// waiting for a round trip to the backend.
pub fn bootstrap_script() -> String {
    let caps = serde_json::to_string(capabilities()).unwrap_or_else(|_| "null".to_string());
    format!("window.__LUMEN__ = {{ capabilities: {} }};", caps)
}
