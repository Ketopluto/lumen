use crate::models::FitMode;
use serde::Serialize;

/// What this platform can actually do.
///
/// Serialized into every window as `window.__LUMEN__.capabilities`, so the UI can hide or explain
/// what is unavailable instead of offering a control that silently does nothing.
#[derive(Debug, Clone, Serialize)]
pub struct Capabilities {
    pub os: &'static str,
    /// Linux only: "x11" or "wayland".
    pub session: Option<&'static str>,
    /// Linux only: "gnome", "kde", "xfce", …
    pub desktop: Option<String>,
    pub live_wallpaper: bool,
    /// Shown verbatim in the UI whenever `live_wallpaper` is false.
    pub live_unsupported_reason: Option<String>,
    /// Whether a live wallpaper covers every monitor (one surface) or only the one it is pinned to.
    pub live_all_monitors: bool,
    pub autostart: bool,
    pub pause_on_fullscreen: bool,
    pub pause_on_battery: bool,
    /// Fit modes this platform can really apply — the rest are hidden in Settings.
    pub fit_modes: Vec<FitMode>,
    pub static_wallpaper: bool,
    /// False where the OS draws its own window controls (macOS traffic lights).
    pub custom_titlebar: bool,
    /// Marks a platform as not fully tested, so the UI can say so.
    pub beta: bool,
}

impl Capabilities {
    /// Everything supported. Backends start here and switch off what they lack.
    #[allow(dead_code)] // each backend uses a different subset
    pub fn full(os: &'static str) -> Self {
        Self {
            os,
            session: None,
            desktop: None,
            live_wallpaper: true,
            live_unsupported_reason: None,
            live_all_monitors: true,
            autostart: true,
            pause_on_fullscreen: true,
            pause_on_battery: true,
            fit_modes: vec![
                FitMode::Fill,
                FitMode::Fit,
                FitMode::Stretch,
                FitMode::Center,
                FitMode::Tile,
                FitMode::Span,
            ],
            static_wallpaper: true,
            custom_titlebar: true,
            beta: false,
        }
    }

    /// Turns the live wallpaper off and records why, in words meant for the user.
    #[allow(dead_code)] // only the non-Windows backends need it
    pub fn without_live(mut self, reason: impl Into<String>) -> Self {
        self.live_wallpaper = false;
        self.live_unsupported_reason = Some(reason.into());
        self
    }
}

/// One live wallpaper window.
///
/// Windows and X11 use a single surface spanning every monitor; macOS and Wayland cannot span
/// displays, so they get one surface per display.
#[derive(Debug, Clone)]
#[allow(dead_code)] // per-display platforms use the monitor and index fields
pub struct SurfaceSpec {
    pub index: usize,
    /// Window label: `live_wallpaper`, `live_wallpaper_1`, …
    pub label: String,
    /// Which monitor this surface belongs to, for per-display platforms.
    pub monitor: Option<usize>,
    /// Position and size hint applied at build time, in physical pixels.
    pub bounds: Option<(i32, i32, u32, u32)>,
}

impl SurfaceSpec {
    pub fn label_for(index: usize) -> String {
        if index == 0 {
            super::LIVE_LABEL.to_string()
        } else {
            format!("{}_{}", super::LIVE_LABEL, index)
        }
    }

    /// A single surface covering the whole desktop (Windows, X11).
    pub fn spanning() -> Vec<Self> {
        vec![Self {
            index: 0,
            label: Self::label_for(0),
            monitor: None,
            bounds: None,
        }]
    }
}

/// What `--selftest` found out about a live wallpaper surface it just created.
///
/// This is the only way the macOS and Linux placement code is checked by anything other than the
/// compiler: CI runs the binary on a real window server and reads this back.
#[derive(Debug, Default, Serialize)]
pub struct SurfaceReport {
    /// Whether the surface really ended up where a wallpaper belongs.
    pub placed: bool,
    pub visible: bool,
    /// Whatever the platform can tell us, for the CI log.
    pub details: std::collections::BTreeMap<String, String>,
}

impl SurfaceReport {
    pub fn detail(&mut self, key: &str, value: impl std::fmt::Display) -> &mut Self {
        self.details.insert(key.to_string(), value.to_string());
        self
    }
}
