//! Anything that isn't Windows, macOS or Linux (BSD, …): browsing works, the desktop doesn't.

use super::{Capabilities, SurfaceSpec};
use crate::models::FitMode;
use tauri::{AppHandle, WebviewWindow};

pub fn capabilities() -> Capabilities {
    let mut caps = Capabilities::full("unknown").without_live("Lumen doesn't know how to draw on this desktop.");
    caps.autostart = false;
    caps.pause_on_fullscreen = false;
    caps.pause_on_battery = false;
    caps.static_wallpaper = false;
    caps.fit_modes = Vec::new();
    caps
}

pub fn plan_surfaces(_app: &AppHandle) -> Vec<SurfaceSpec> {
    Vec::new()
}

pub fn attach(_window: &WebviewWindow, _spec: &SurfaceSpec) -> Result<(), String> {
    Err("Live wallpapers aren't supported on this system".into())
}

pub fn refit(_window: &WebviewWindow, _spec: &SurfaceSpec) {}

pub fn fullscreen_app_active() -> bool {
    false
}

pub fn on_battery() -> bool {
    false
}

pub fn set_autostart(_enable: bool) -> Result<(), String> {
    Err("Starting at login isn't supported on this system".into())
}

pub fn set_static_wallpaper(_path: &str, _fit: &FitMode) -> Result<(), String> {
    Err("Setting the wallpaper isn't supported on this system".into())
}

pub fn current_static_wallpaper() -> Option<String> {
    None
}
