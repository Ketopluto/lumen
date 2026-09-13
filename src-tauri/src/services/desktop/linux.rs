//! Linux. Session and desktop detection, power and fullscreen state; the X11 and Wayland
//! wallpaper surfaces land in later stages.

use super::{Capabilities, SurfaceSpec};

#[path = "linux_x11.rs"]
mod x11;
use crate::models::FitMode;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
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
    let mut caps = Capabilities::full("linux");
    caps = match session {
        // Every X11 desktop understands a desktop-type window.
        SessionType::X11 => caps,
        SessionType::Wayland if desktop.contains("gnome") => caps.without_live(
            "GNOME on Wayland does not let apps draw on the desktop, so live wallpapers cannot work \
             here. Still images work fine. For live wallpapers, log out and choose \"GNOME on Xorg\" \
             at the login screen.",
        ),
        SessionType::Wayland => {
            caps.without_live("Live wallpapers on Wayland compositors are still being built. Still images work now.")
        }
        SessionType::Unknown => caps.without_live("Lumen could not tell which display server this session uses."),
    };
    caps.session = Some(session.as_str());
    caps.desktop = Some(desktop);
    // No way to inspect other windows on Wayland, by design.
    caps.pause_on_fullscreen = session == SessionType::X11;
    caps.live_all_monitors = session == SessionType::X11;
    caps.fit_modes = vec![FitMode::Fill, FitMode::Fit, FitMode::Stretch, FitMode::Center];
    caps
}

pub fn plan_surfaces(_app: &AppHandle) -> Vec<SurfaceSpec> {
    SurfaceSpec::spanning()
}

pub fn attach(window: &WebviewWindow, spec: &SurfaceSpec) -> Result<(), String> {
    match session() {
        SessionType::X11 => x11::attach(window, spec),
        _ => Err(capabilities()
            .live_unsupported_reason
            .clone()
            .unwrap_or_else(|| "Live wallpapers are not supported on this desktop".into())),
    }
}

pub fn refit(window: &WebviewWindow, spec: &SurfaceSpec) {
    if session() == SessionType::X11 {
        x11::refit(window, spec);
    }
}

// ── Fullscreen (X11 only) ──────────────────────────────────────────────────

struct X11Probe {
    conn: x11rb::rust_connection::RustConnection,
    root: u32,
    active_window: u32,
    wm_state: u32,
    fullscreen: u32,
}

/// One connection for the lifetime of the process: this is polled every two seconds.
fn x11_probe() -> Option<&'static X11Probe> {
    static PROBE: OnceLock<Option<X11Probe>> = OnceLock::new();
    PROBE
        .get_or_init(|| {
            use x11rb::connection::Connection;
            use x11rb::protocol::xproto::ConnectionExt as _;

            let (conn, screen) = x11rb::connect(None).ok()?;
            let root = conn.setup().roots.get(screen)?.root;
            let atom = |name: &str| {
                conn.intern_atom(true, name.as_bytes())
                    .ok()?
                    .reply()
                    .ok()
                    .map(|r| r.atom)
            };
            Some(X11Probe {
                root,
                active_window: atom("_NET_ACTIVE_WINDOW")?,
                wm_state: atom("_NET_WM_STATE")?,
                fullscreen: atom("_NET_WM_STATE_FULLSCREEN")?,
                conn,
            })
        })
        .as_ref()
}

/// True when the focused window claims `_NET_WM_STATE_FULLSCREEN` — the same signal a compositor
/// uses to unredirect a game. Wayland has no way to ask about other windows, by design.
pub fn fullscreen_app_active() -> bool {
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};

    if session() != SessionType::X11 {
        return false;
    }
    let Some(probe) = x11_probe() else {
        return false;
    };
    let active = probe
        .conn
        .get_property(false, probe.root, probe.active_window, AtomEnum::WINDOW, 0, 1)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .and_then(|reply| reply.value32()?.next())
        .filter(|window| *window != 0);
    let Some(window) = active else {
        return false;
    };
    probe
        .conn
        .get_property(false, window, probe.wm_state, AtomEnum::ATOM, 0, 32)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .and_then(|reply| Some(reply.value32()?.any(|atom| atom == probe.fullscreen)))
        .unwrap_or(false)
}

// ── Power ──────────────────────────────────────────────────────────────────

fn power_supply_dir() -> PathBuf {
    // Overridable so the logic is testable without a laptop.
    std::env::var_os("LUMEN_SYSFS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/sys"))
        .join("class/power_supply")
}

pub fn on_battery() -> bool {
    on_battery_in(&power_supply_dir())
}

/// A "Mains" supply reporting `online = 0` means the machine is running on its battery.
/// Desktops have no such entry, so they are never "on battery".
fn on_battery_in(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_mains = std::fs::read_to_string(path.join("type"))
            .map(|kind| kind.trim() == "Mains")
            .unwrap_or(false);
        if !is_mains {
            continue;
        }
        if let Ok(online) = std::fs::read_to_string(path.join("online")) {
            return online.trim() == "0";
        }
    }
    false
}

// ── Static wallpaper ───────────────────────────────────────────────────────

pub fn set_static_wallpaper(path: &str, fit: &FitMode) -> Result<(), String> {
    // Honoured by the GNOME, KDE, Cinnamon, MATE and XFCE paths; other desktops ignore it.
    let _ = ::wallpaper::set_mode(fit.to_mode());
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

    #[test]
    fn battery_state_comes_from_the_mains_supply() {
        let root = std::env::temp_dir().join(format!("lumen-sysfs-{}", uuid::Uuid::new_v4()));
        let mains = root.join("AC");
        let battery = root.join("BAT0");
        std::fs::create_dir_all(&mains).unwrap();
        std::fs::create_dir_all(&battery).unwrap();
        std::fs::write(battery.join("type"), "Battery\n").unwrap();
        std::fs::write(mains.join("type"), "Mains\n").unwrap();

        std::fs::write(mains.join("online"), "0\n").unwrap();
        assert!(on_battery_in(&root), "unplugged means on battery");

        std::fs::write(mains.join("online"), "1\n").unwrap();
        assert!(!on_battery_in(&root), "plugged in is not on battery");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_desktop_without_a_mains_supply_is_never_on_battery() {
        let empty = std::env::temp_dir().join(format!("lumen-sysfs-empty-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&empty).unwrap();
        assert!(!on_battery_in(&empty));
        let _ = std::fs::remove_dir_all(&empty);
    }
}
