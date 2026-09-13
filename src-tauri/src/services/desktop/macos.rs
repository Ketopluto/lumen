//! macOS. Live wallpapers land here in a later stage; static wallpapers already work.

use super::{Capabilities, SurfaceSpec};
use crate::models::FitMode;
use tauri::{AppHandle, WebviewWindow};

pub fn capabilities() -> Capabilities {
    let mut caps = Capabilities::full("macos")
        .without_live("Live wallpapers on macOS are still being built. Still images work now.");
    // NSWorkspace has no tiled or spanning desktop image.
    caps.fit_modes = vec![FitMode::Fill, FitMode::Fit, FitMode::Stretch, FitMode::Center];
    // A window cannot span displays while "Displays have separate Spaces" is on (the default).
    caps.live_all_monitors = false;
    caps.pause_on_fullscreen = false;
    caps.custom_titlebar = false;
    caps.beta = true;
    caps
}

pub fn plan_surfaces(_app: &AppHandle) -> Vec<SurfaceSpec> {
    SurfaceSpec::spanning()
}

pub fn attach(_window: &WebviewWindow, _spec: &SurfaceSpec) -> Result<(), String> {
    Err("Live wallpapers on macOS are still being built".into())
}

pub fn refit(_window: &WebviewWindow, _spec: &SurfaceSpec) {}

pub fn fullscreen_app_active() -> bool {
    false
}

pub fn on_battery() -> bool {
    false
}

pub fn set_static_wallpaper(path: &str, _fit: &FitMode) -> Result<(), String> {
    // `wallpaper`'s macOS path drives Finder over AppleScript; NSWorkspace replaces it later,
    // which also removes the Automation permission prompt and brings real fit modes.
    ::wallpaper::set_from_path(path).map_err(|e| format!("Failed to set wallpaper: {}", e))
}

pub fn current_static_wallpaper() -> Option<String> {
    // Asking costs an AppleScript round trip (and a permission prompt), so don't.
    None
}
