//! Still wallpapers on Linux, one desktop at a time.
//!
//! The `wallpaper` crate covers the common cases but misses the ones that matter most here: it
//! never writes GNOME's dark-mode key (so on a GNOME 42+ desktop in dark mode, which is the
//! default, nothing appears to happen), it drives Plasma through a `qdbus` script that Plasma 6
//! renamed out from under it, and on wlroots it spawns a fresh `swaybg` every time and leaves the
//! old ones running. Each desktop is handled directly, and anything unrecognised still falls back
//! to the crate.

use crate::models::FitMode;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;

pub fn set(desktop: &str, path: &str, fit: &FitMode) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("{} no longer exists", path));
    }
    match desktop {
        d if d.contains("gnome") || d.contains("unity") || d.contains("pop") || d.contains("budgie") => {
            gsettings_family("org.gnome.desktop.background", path, fit, true)
        }
        d if d.contains("cinnamon") => gsettings_family("org.cinnamon.desktop.background", path, fit, false),
        d if d.contains("mate") => mate(path, fit),
        d if d.contains("kde") || d.contains("plasma") => plasma(path),
        d if d.contains("xfce") => xfce(path, fit),
        d if d.contains("sway") || d.contains("hyprland") || d.contains("wlroots") || d.contains("river") => {
            swaybg(path, fit)
        }
        // LXQt, Deepin, i3 and friends: whatever the crate knows.
        _ => {
            let _ = ::wallpaper::set_mode(fit.to_mode());
            ::wallpaper::set_from_path(path).map_err(|e| format!("Failed to set wallpaper: {}", e))
        }
    }
}

pub fn current(desktop: &str) -> Option<String> {
    let uri = match desktop {
        d if d.contains("gnome") || d.contains("unity") || d.contains("pop") || d.contains("budgie") => {
            gsettings_get("org.gnome.desktop.background", "picture-uri")
        }
        d if d.contains("cinnamon") => gsettings_get("org.cinnamon.desktop.background", "picture-uri"),
        d if d.contains("mate") => gsettings_get("org.mate.background", "picture-filename"),
        d if d.contains("sway") || d.contains("hyprland") || d.contains("wlroots") || d.contains("river") => {
            SWAYBG.lock().ok()?.as_ref().map(|running| running.path.clone())
        }
        _ => ::wallpaper::get().ok(),
    }?;
    let path = uri.strip_prefix("file://").unwrap_or(&uri).to_string();
    Some(path).filter(|p| !p.is_empty())
}

/// Only the modes this desktop can really apply, so Settings never offers one that does nothing.
/// Plasma is set through `plasma-apply-wallpaperimage`, which has no say over the fit at all.
pub fn fit_modes(desktop: &str) -> Vec<FitMode> {
    if desktop.contains("kde") || desktop.contains("plasma") {
        return vec![FitMode::Fill];
    }
    let gnome_family = desktop.contains("gnome")
        || desktop.contains("unity")
        || desktop.contains("pop")
        || desktop.contains("budgie")
        || desktop.contains("cinnamon")
        || desktop.contains("mate");
    let mut modes = vec![FitMode::Fill, FitMode::Fit, FitMode::Stretch, FitMode::Center];
    if gnome_family || desktop.contains("xfce") || desktop.contains("sway") || desktop.contains("hyprland") {
        modes.push(FitMode::Tile);
    }
    if gnome_family {
        modes.push(FitMode::Span);
    }
    modes
}

// ── GNOME and its relatives ────────────────────────────────────────────────

/// GNOME 42 and later keep a second wallpaper for the dark theme; writing only `picture-uri`
/// looks like nothing happened at all to anyone using the default dark appearance.
fn gsettings_family(schema: &str, path: &str, fit: &FitMode, dark: bool) -> Result<(), String> {
    let uri = file_uri(path);
    gsettings_set(schema, "picture-uri", &uri)?;
    if dark {
        // Missing before GNOME 42; a failure here is not worth reporting.
        let _ = gsettings_set(schema, "picture-uri-dark", &uri);
    }
    let _ = gsettings_set(schema, "picture-options", gnome_option(fit));
    Ok(())
}

fn mate(path: &str, fit: &FitMode) -> Result<(), String> {
    // MATE stores a plain path rather than a URI.
    gsettings_set("org.mate.background", "picture-filename", path)?;
    let _ = gsettings_set("org.mate.background", "picture-options", gnome_option(fit));
    Ok(())
}

fn gnome_option(fit: &FitMode) -> &'static str {
    match fit {
        FitMode::Fill => "zoom",
        FitMode::Fit => "scaled",
        FitMode::Stretch => "stretched",
        FitMode::Center => "centered",
        FitMode::Tile => "wallpaper",
        FitMode::Span => "spanned",
    }
}

fn gsettings_set(schema: &str, key: &str, value: &str) -> Result<(), String> {
    run("gsettings", &["set", schema, key, value])
}

fn gsettings_get(schema: &str, key: &str) -> Option<String> {
    let out = Command::new("gsettings").args(["get", schema, key]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    // gsettings quotes its answer: 'file:///home/me/a.jpg'
    Some(
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .trim_matches('\'')
            .to_string(),
    )
}

// ── KDE Plasma ─────────────────────────────────────────────────────────────

/// `plasma-apply-wallpaperimage` ships with Plasma 5.18+ and is the only supported way in: the
/// old `evaluateScript` D-Bus call was renamed in Plasma 6 and silently does nothing there.
fn plasma(path: &str) -> Result<(), String> {
    run("plasma-apply-wallpaperimage", &[path]).map_err(|e| {
        format!(
            "{}. Plasma sets wallpapers through plasma-apply-wallpaperimage — it comes with \
             plasma-workspace, which should already be installed.",
            e
        )
    })
}

// ── XFCE ───────────────────────────────────────────────────────────────────

/// xfdesktop keeps one `last-image` property per monitor per workspace, so every one of them has
/// to be written or the wallpaper only changes on some screens.
fn xfce(path: &str, fit: &FitMode) -> Result<(), String> {
    let out = Command::new("xfconf-query")
        .args(["-c", "xfce4-desktop", "-l"])
        .output()
        .map_err(|e| format!("xfconf-query is not available: {}", e))?;
    let properties: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| line.ends_with("/last-image"))
        .collect();
    if properties.is_empty() {
        return Err("XFCE reported no desktop backdrop to change.".into());
    }
    let style = xfce_style(fit);
    for property in &properties {
        run("xfconf-query", &["-c", "xfce4-desktop", "-p", property, "-s", path])?;
        let style_property = property.replace("/last-image", "/image-style");
        let _ = run(
            "xfconf-query",
            &["-c", "xfce4-desktop", "-p", &style_property, "-s", style],
        );
    }
    Ok(())
}

fn xfce_style(fit: &FitMode) -> &'static str {
    match fit {
        FitMode::Center => "1",
        FitMode::Tile => "2",
        FitMode::Stretch => "3",
        FitMode::Fit => "4",
        // XFCE has no spanning mode; zoomed is the closest.
        FitMode::Fill | FitMode::Span => "5",
    }
}

// ── wlroots compositors ────────────────────────────────────────────────────

struct Swaybg {
    child: std::process::Child,
    path: String,
}

static SWAYBG: Mutex<Option<Swaybg>> = Mutex::new(None);

/// sway, Hyprland and river have no wallpaper setting of their own: a client paints the
/// background and keeps running. Only ours is replaced, and only one is ever alive — the crate
/// leaks another `swaybg` on every change until the desktop is a stack of dead wallpapers.
fn swaybg(path: &str, fit: &FitMode) -> Result<(), String> {
    let mode = match fit {
        FitMode::Fill | FitMode::Span => "fill",
        FitMode::Fit => "fit",
        FitMode::Stretch => "stretch",
        FitMode::Center => "center",
        FitMode::Tile => "tile",
    };
    let child = Command::new("swaybg")
        .args(["-i", path, "-m", mode])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            format!(
                "swaybg could not be started ({}). Install swaybg — this compositor has no \
                 wallpaper setting of its own.",
                e
            )
        })?;
    let mut slot = SWAYBG.lock().map_err(|_| "wallpaper process lock poisoned")?;
    if let Some(mut previous) = slot.take() {
        let _ = previous.child.kill();
        let _ = previous.child.wait();
    }
    *slot = Some(Swaybg {
        child,
        path: path.to_string(),
    });
    Ok(())
}

// ── Shared ─────────────────────────────────────────────────────────────────

fn file_uri(path: &str) -> String {
    // Spaces and '#' are the two that actually break gsettings in practice.
    let escaped = path.replace('%', "%25").replace(' ', "%20").replace('#', "%23");
    format!("file://{}", escaped)
}

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("Could not run {}: {}", program, e))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(if stderr.is_empty() {
        format!("{} failed", program)
    } else {
        format!("{} failed: {}", program, stderr)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_uri_escapes_what_gsettings_would_choke_on() {
        assert_eq!(file_uri("/home/me/a b#c.jpg"), "file:///home/me/a%20b%23c.jpg");
    }

    #[test]
    fn fit_modes_map_to_each_desktops_own_vocabulary() {
        assert_eq!(gnome_option(&FitMode::Fill), "zoom");
        assert_eq!(gnome_option(&FitMode::Tile), "wallpaper");
        assert_eq!(xfce_style(&FitMode::Center), "1");
        assert_eq!(xfce_style(&FitMode::Span), "5");
    }
}
