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

pub use types::{Capabilities, MonitorInfo, MonitorTarget, SurfaceReport, SurfaceSpec};

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
    let _: fn(&str, &FitMode, Option<MonitorTarget>) -> Result<(), String> = imp::set_static_wallpaper;
    let _: fn() -> Option<String> = imp::current_static_wallpaper;
    let _: fn(&FitMode) = imp::redraw_wallpaper;
    let _: fn(&WebviewWindow) -> SurfaceReport = imp::describe;
    let _: fn(&AppHandle) -> Vec<MonitorInfo> = imp::monitor_layout;
    #[cfg(target_os = "windows")]
    let _: fn(&[MonitorTarget]) -> std::collections::BTreeMap<usize, String> = imp::current_monitor_wallpapers;
    #[cfg(target_os = "windows")]
    let _: fn(&[isize]) -> Vec<isize> = imp::sweep_orphan_surfaces;
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

/// What `--selftest` sees: where the surface really ended up. Main thread only.
pub fn describe(window: &WebviewWindow) -> SurfaceReport {
    imp::describe(window)
}

pub fn fullscreen_app_active() -> bool {
    imp::fullscreen_app_active()
}

pub fn on_battery() -> bool {
    imp::on_battery()
}

/// `target` of `None` means the whole desktop. A backend without per-monitor support ignores it,
/// which is why `Capabilities::per_monitor_static` exists for the UI to check first.
pub fn set_static_wallpaper(path: &str, fit: &FitMode, target: Option<MonitorTarget>) -> Result<(), String> {
    imp::set_static_wallpaper(path, fit, target)
}

/// Every monitor, in the one order Lumen numbers them — the UI, the surfaces and the stored
/// layout all index into this.
pub fn monitor_layout(app: &AppHandle) -> Vec<MonitorInfo> {
    imp::monitor_layout(app)
}

/// Every monitor Lumen can see, in the order the UI numbers them.
pub fn monitors(app: &AppHandle) -> Vec<MonitorTarget> {
    monitor_layout(app)
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| MonitorTarget {
            index,
            bounds: monitor.bounds,
        })
        .collect()
}

/// One surface per named monitor, for when the monitors are showing different wallpapers. Labels
/// follow the monitor index so they stay stable while only one monitor's choice changes.
pub fn surfaces_for(app: &AppHandle, monitors: &[usize]) -> Vec<SurfaceSpec> {
    per_monitor(app)
        .into_iter()
        .filter(|spec| spec.monitor.is_some_and(|m| monitors.contains(&m)))
        .map(|mut spec| {
            spec.index = spec.monitor.unwrap_or(spec.index);
            spec.label = SurfaceSpec::label_for(spec.index);
            spec
        })
        .collect()
}

/// The still image each monitor is showing.
///
/// Only Windows keeps one image per monitor; everywhere else the desktop has a single wallpaper,
/// so every monitor truthfully reports the same file. The UI uses this to show what is actually
/// on each screen, so it must never guess — a monitor nothing is known about is left out.
pub fn current_monitor_wallpapers(app: &AppHandle) -> std::collections::BTreeMap<usize, String> {
    #[cfg(target_os = "windows")]
    {
        imp::current_monitor_wallpapers(&monitors(app))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let Some(path) = imp::current_static_wallpaper() else {
            return Default::default();
        };
        monitors(app)
            .into_iter()
            .map(|monitor| (monitor.index, path.clone()))
            .collect()
    }
}

/// Windows only: closes surfaces the desktop is still showing but Tauri has lost track of, and
/// reports what it closed. See the implementation for why they can exist at all. Main thread only.
#[cfg(target_os = "windows")]
pub fn sweep_orphan_surfaces(keep: &[isize]) -> Vec<isize> {
    imp::sweep_orphan_surfaces(keep)
}

/// Windows only, read-only: the shell's own monitor list, for diagnosing a still image that
/// landed on the wrong screen.
#[cfg(target_os = "windows")]
pub fn describe_monitors() -> Vec<String> {
    imp::describe_monitors()
}

/// Makes the desktop paint its wallpaper again after a live surface disappears, without changing
/// which image is set — re-applying one image would flatten per-monitor choices into it.
pub fn redraw_wallpaper(fit: &FitMode) {
    imp::redraw_wallpaper(fit)
}

/// Runs `f` on the UI thread and waits for the result.
pub fn on_main<T: Send + 'static>(app: &AppHandle, f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
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
    surface_script(None, &FitMode::Fill)
}

/// The same, plus the file this particular surface must play. Each monitor can be showing a
/// different wallpaper, so a surface cannot ask "what is the live wallpaper?" and get a useful
/// answer — it is told, before its first paint, which avoids a wrong video flashing up first.
pub fn surface_script(src: Option<&str>, fit: &FitMode) -> String {
    let caps = serde_json::to_string(capabilities()).unwrap_or_else(|_| "null".to_string());
    let src = serde_json::to_string(&src).unwrap_or_else(|_| "null".to_string());
    let fit = serde_json::to_string(fit).unwrap_or_else(|_| "\"fill\"".to_string());
    format!(
        "window.__LUMEN__ = {{ capabilities: {}, live_src: {}, live_fit: {} }};",
        caps, src, fit
    )
}

/// One surface per display, for platforms where a window cannot span displays (macOS, Wayland).
/// Uses Tauri's monitor list rather than the native one: it is safe to call from any thread.
pub(crate) fn per_monitor(app: &AppHandle) -> Vec<SurfaceSpec> {
    let monitors = monitor_layout(app);
    if monitors.is_empty() {
        log::warn!("no monitor could be enumerated; covering the whole desktop with one surface");
        return SurfaceSpec::spanning();
    }
    monitors
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| SurfaceSpec {
            index,
            label: SurfaceSpec::label_for(index),
            monitor: Some(index),
            bounds: Some(monitor.bounds),
        })
        .collect()
}

/// Tauri's monitor list, for the platforms whose own list is already in physical pixels.
#[allow(dead_code)] // Windows enumerates natively instead
pub(crate) fn tauri_monitor_layout(app: &AppHandle) -> Vec<MonitorInfo> {
    let primary = app.primary_monitor().ok().flatten().map(|monitor| *monitor.position());
    app.available_monitors()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| {
            let position = monitor.position();
            let size = monitor.size();
            MonitorInfo {
                name: monitor
                    .name()
                    .cloned()
                    .unwrap_or_else(|| format!("Display {}", index + 1)),
                bounds: (position.x, position.y, size.width, size.height),
                primary: primary.is_some_and(|p| p == *position),
            }
        })
        .collect()
}
