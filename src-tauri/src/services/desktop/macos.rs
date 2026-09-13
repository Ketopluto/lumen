//! macOS: the player sits at the desktop window level, below the desktop icons.
//!
//! Beta. The window placement is exercised by CI, but nobody has confirmed visually that the video
//! appears behind the icons, so `Capabilities::beta` is set and the UI says so.

use super::{Capabilities, SurfaceReport, SurfaceSpec};
use crate::models::FitMode;
use objc2::msg_send;
use objc2::runtime::{AnyObject, Bool};
use tauri::{AppHandle, WebviewWindow};

/// `kCGDesktopWindowLevel`. The icons sit 20 levels above it, so the wallpaper stays behind them.
const DESKTOP_WINDOW_LEVEL: isize = -2_147_483_623;

/// canJoinAllSpaces | stationary | ignoresCycle | fullScreenNone — present on every Space, never
/// moved by Mission Control, never offered by Cmd+Tab.
const COLLECTION_BEHAVIOR: usize = (1 << 0) | (1 << 4) | (1 << 6) | (1 << 9);

pub fn capabilities() -> Capabilities {
    let mut caps = Capabilities::full("macos");
    // NSWorkspace has no tiled or spanning desktop image.
    caps.fit_modes = vec![FitMode::Fill, FitMode::Fit, FitMode::Stretch, FitMode::Center];
    // Asking which app is fullscreen needs AppKit on the main thread; not wired up yet.
    caps.pause_on_fullscreen = false;
    // The system draws the traffic lights.
    caps.custom_titlebar = false;
    caps.beta = true;
    caps
}

/// A window cannot span displays while "Displays have separate Spaces" is on, which is the
/// default, so every screen gets its own surface.
pub fn plan_surfaces(app: &AppHandle) -> Vec<SurfaceSpec> {
    super::per_monitor(app)
}

pub fn attach(window: &WebviewWindow, _spec: &SurfaceSpec) -> Result<(), String> {
    let ns = window.ns_window().map_err(|e| e.to_string())? as *mut AnyObject;
    if ns.is_null() {
        return Err("This window has no native counterpart".into());
    }
    // Safety: `ns_window` hands back the NSWindow for this window, and `attach` runs on the main
    // thread (see the contract in mod.rs), which is where AppKit requires these calls.
    unsafe {
        let _: () = msg_send![ns, setLevel: DESKTOP_WINDOW_LEVEL];
        let _: () = msg_send![ns, setCollectionBehavior: COLLECTION_BEHAVIOR];
        let _: () = msg_send![ns, setIgnoresMouseEvents: Bool::YES];
        let _: () = msg_send![ns, setHasShadow: Bool::NO];
        let _: () = msg_send![ns, setMovable: Bool::NO];
        // Visible without activating Lumen — Tauri's show() would bring the app forward.
        let _: () = msg_send![ns, orderFrontRegardless];
    }
    Ok(())
}

pub fn refit(window: &WebviewWindow, spec: &SurfaceSpec) {
    let Ok(ns) = window.ns_window() else {
        return;
    };
    // Re-assert the level: switching Spaces or waking a display can raise the window.
    unsafe {
        let _: () = msg_send![ns as *mut AnyObject, setLevel: DESKTOP_WINDOW_LEVEL];
    }
    // Follow the display through resolution changes, using Tauri's geometry rather than AppKit's
    // (their coordinate systems differ, and Tauri already normalises it).
    if let Some((x, y, width, height)) = spec.bounds {
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = window.set_size(tauri::PhysicalSize::new(width, height));
    }
}

pub fn fullscreen_app_active() -> bool {
    false
}

pub fn on_battery() -> bool {
    // IOKit under the hood, no AppKit, so this is safe from the watchdog thread.
    let Ok(manager) = starship_battery::Manager::new() else {
        return false;
    };
    let Ok(batteries) = manager.batteries() else {
        return false;
    };
    batteries
        .flatten()
        .any(|battery| battery.state() == starship_battery::State::Discharging)
}

pub fn set_static_wallpaper(path: &str, _fit: &FitMode) -> Result<(), String> {
    // The `wallpaper` crate drives Finder over AppleScript, so the first call asks for Automation
    // permission. Moving to NSWorkspace would remove the prompt and bring real fit modes.
    ::wallpaper::set_from_path(path).map_err(|e| format!("Failed to set wallpaper: {}", e))
}

pub fn current_static_wallpaper() -> Option<String> {
    // Asking costs an AppleScript round trip and a permission prompt, so don't.
    None
}

/// Reports where the surface actually landed — see `SurfaceReport`. This is what CI checks on a
/// real window server, since nobody has a Mac to look at.
pub fn describe(window: &WebviewWindow) -> SurfaceReport {
    let mut report = SurfaceReport::default();
    let Ok(ns) = window.ns_window() else {
        report.detail("error", "this window has no NSWindow");
        return report;
    };
    let ns = ns as *mut AnyObject;
    if ns.is_null() {
        report.detail("error", "this window has no NSWindow");
        return report;
    }
    // Safety: same as `attach` — a real NSWindow, inspected on the main thread.
    unsafe {
        let level: isize = msg_send![ns, level];
        let ignores_mouse: Bool = msg_send![ns, ignoresMouseEvents];
        let visible: Bool = msg_send![ns, isVisible];
        let behavior: usize = msg_send![ns, collectionBehavior];
        report.visible = visible.as_bool();
        report.placed = level == DESKTOP_WINDOW_LEVEL && ignores_mouse.as_bool();
        report
            .detail("level", level)
            .detail("desktop_level", DESKTOP_WINDOW_LEVEL)
            .detail("ignores_mouse", ignores_mouse.as_bool())
            .detail("collection_behavior", behavior)
            .detail("expected_collection_behavior", COLLECTION_BEHAVIOR);
    }
    report
}
