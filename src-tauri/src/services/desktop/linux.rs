//! Linux. Session and desktop detection is live; the X11 and Wayland surfaces land in later stages.

use super::{Capabilities, SurfaceSpec};
use crate::models::FitMode;
use tauri::{AppHandle, WebviewWindow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    X11,
    Wayland,
    Unknown,
}

impl SessionType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X11 => "x11",
            Self::Wayland => "wayland",
            Self::Unknown => "unknown",
        }
    }
}

/// Which display server this session uses. `GDK_BACKEND` wins, because it also decides what GTK
/// itself will talk to.
pub fn session() -> SessionType {
    if let Ok(backend) = std::env::var("GDK_BACKEND") {
        let backend = backend.to_lowercase();
        if backend.contains("x11") {
            return SessionType::X11;
        }
        if backend.contains("wayland") {
            return SessionType::Wayland;
        }
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland") {
        SessionType::Wayland
    } else if std::env::var_os("DISPLAY").is_some() {
        SessionType::X11
    } else {
        SessionType::Unknown
    }
}

/// "gnome", "kde", "xfce", "sway", … from whichever variable the desktop sets.
pub fn desktop_environment() -> String {
    let raw = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("XDG_SESSION_DESKTOP"))
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .unwrap_or_default()
        .to_lowercase();
    // "ubuntu:GNOME" → "gnome"
    raw.split(':').next_back().unwrap_or("").to_string()
}

pub fn capabilities() -> Capabilities {
    let session = session();
    let desktop = desktop_environment();
    let mut caps = Capabilities::full("linux").without_live(match session {
        SessionType::Wayland if desktop.contains("gnome") => {
            "GNOME on Wayland doesn't let apps draw on the desktop, so live wallpapers can't work \
             here. Still images work fine. For live wallpapers, log out and choose \
             \"GNOME on Xorg\" at the login screen."
                .to_string()
        }
        _ => "Live wallpapers on Linux are still being built. Still images work now.".to_string(),
    });
    caps.session = Some(session.as_str());
    caps.desktop = Some(desktop);
    caps.autostart = false;
    // No way to inspect other windows on Wayland, by design.
    caps.pause_on_fullscreen = session == SessionType::X11;
    caps.live_all_monitors = session == SessionType::X11;
    caps.fit_modes = vec![FitMode::Fill, FitMode::Fit, FitMode::Stretch, FitMode::Center];
    caps
}

pub fn plan_surfaces(_app: &AppHandle) -> Vec<SurfaceSpec> {
    SurfaceSpec::spanning()
}

pub fn attach(_window: &WebviewWindow, _spec: &SurfaceSpec) -> Result<(), String> {
    Err("Live wallpapers on Linux are still being built".into())
}

pub fn refit(_window: &WebviewWindow, _spec: &SurfaceSpec) {}

pub fn fullscreen_app_active() -> bool {
    false
}

pub fn on_battery() -> bool {
    false
}

pub fn set_autostart(_enable: bool) -> Result<(), String> {
    Err("Starting Lumen at login isn't wired up on Linux yet".into())
}

pub fn set_static_wallpaper(path: &str, _fit: &FitMode) -> Result<(), String> {
    ::wallpaper::set_from_path(path).map_err(|e| format!("Failed to set wallpaper: {}", e))
}

pub fn current_static_wallpaper() -> Option<String> {
    ::wallpaper::get().ok().filter(|p| !p.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_name_drops_the_vendor_prefix() {
        std::env::set_var("XDG_CURRENT_DESKTOP", "ubuntu:GNOME");
        assert_eq!(desktop_environment(), "gnome");
        std::env::remove_var("XDG_CURRENT_DESKTOP");
    }
}
