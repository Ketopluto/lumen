//! Wayland: the player becomes a `wlr-layer-shell` surface on the background layer.
//!
//! `libgtk-layer-shell` is loaded at runtime rather than linked, so it stays an optional
//! dependency: on a compositor without it (or on GNOME, which has no such protocol at all) the
//! capability simply turns off instead of the app failing to start.

use super::{SurfaceReport, SurfaceSpec};
use gtk::glib::translate::ToGlibPtr;
use gtk::prelude::*;
use std::ffi::{c_char, c_int, c_void};
use std::sync::OnceLock;
use tauri::WebviewWindow;

const LAYER_BACKGROUND: c_int = 0;
const KEYBOARD_MODE_NONE: c_int = 0;
const EDGES: [c_int; 4] = [0, 1, 2, 3]; // left, right, top, bottom

struct LayerShell {
    _library: libloading::Library,
    is_supported: unsafe extern "C" fn() -> c_int,
    is_layer_window: unsafe extern "C" fn(*mut c_void) -> c_int,
    init_for_window: unsafe extern "C" fn(*mut c_void),
    set_layer: unsafe extern "C" fn(*mut c_void, c_int),
    set_anchor: unsafe extern "C" fn(*mut c_void, c_int, c_int),
    set_exclusive_zone: unsafe extern "C" fn(*mut c_void, c_int),
    set_monitor: unsafe extern "C" fn(*mut c_void, *mut c_void),
    set_namespace: unsafe extern "C" fn(*mut c_void, *const c_char),
    set_keyboard_mode: unsafe extern "C" fn(*mut c_void, c_int),
}

/// Loaded once; `None` when the library isn't installed.
fn layer_shell() -> Option<&'static LayerShell> {
    static SHELL: OnceLock<Option<LayerShell>> = OnceLock::new();
    SHELL
        .get_or_init(|| unsafe {
            let library = ["libgtk-layer-shell.so.0", "libgtk-layer-shell.so"]
                .iter()
                .find_map(|name| libloading::Library::new(name).ok())?;
            let shell = LayerShell {
                is_supported: *library.get(b"gtk_layer_is_supported\0").ok()?,
                is_layer_window: *library.get(b"gtk_layer_is_layer_window\0").ok()?,
                init_for_window: *library.get(b"gtk_layer_init_for_window\0").ok()?,
                set_layer: *library.get(b"gtk_layer_set_layer\0").ok()?,
                set_anchor: *library.get(b"gtk_layer_set_anchor\0").ok()?,
                set_exclusive_zone: *library.get(b"gtk_layer_set_exclusive_zone\0").ok()?,
                set_monitor: *library.get(b"gtk_layer_set_monitor\0").ok()?,
                set_namespace: *library.get(b"gtk_layer_set_namespace\0").ok()?,
                set_keyboard_mode: *library.get(b"gtk_layer_set_keyboard_mode\0").ok()?,
                _library: library,
            };
            Some(shell)
        })
        .as_ref()
}

/// Whether this system could show a layer-shell wallpaper. Only checks that the library is
/// installed: asking the compositor needs a GDK connection, which does not exist yet at startup.
pub fn library_available() -> bool {
    layer_shell().is_some()
}

const MISSING_LIBRARY: &str = "Live wallpapers on Wayland need gtk-layer-shell. Install it \
     (gtk-layer-shell on Fedora and Arch, libgtk-layer-shell0 on Debian and Ubuntu) and restart Lumen.";

pub fn attach(window: &WebviewWindow, spec: &SurfaceSpec) -> Result<(), String> {
    let shell = layer_shell().ok_or(MISSING_LIBRARY)?;
    if unsafe { (shell.is_supported)() } == 0 {
        return Err("This Wayland compositor has no background layer for apps to draw on.".into());
    }

    let gtk_window = window.gtk_window().map_err(|e| e.to_string())?;
    // The C API takes a GtkWindow*, so upcast before handing over the pointer.
    let window_ptr: *mut gtk::ffi::GtkWindow = gtk_window.upcast_ref::<gtk::Window>().to_glib_none().0;
    let raw = window_ptr as *mut c_void;

    unsafe {
        // Everything below must happen before the window is realized, which `.visible(false)`
        // guarantees: tao and wry both skip show_all() for an invisible window.
        (shell.init_for_window)(raw);
        (shell.set_namespace)(raw, c"lumen-wallpaper".as_ptr());
        (shell.set_layer)(raw, LAYER_BACKGROUND);
        for edge in EDGES {
            // Anchored to all four edges means "fill this output".
            (shell.set_anchor)(raw, edge, 1);
        }
        // Ignore panel exclusive zones: a wallpaper covers the whole output.
        (shell.set_exclusive_zone)(raw, -1);
        (shell.set_keyboard_mode)(raw, KEYBOARD_MODE_NONE);

        // Layer-shell surfaces belong to one output, so pin each surface to its monitor.
        if let Some(index) = spec.monitor {
            if let Some(monitor) = gtk::gdk::Display::default().and_then(|d| d.monitor(index as i32)) {
                let monitor_ptr: *mut gtk::gdk::ffi::GdkMonitor = monitor.to_glib_none().0;
                (shell.set_monitor)(raw, monitor_ptr as *mut c_void);
            }
        }
    }

    // The only place a live surface becomes visible on Linux — see the contract in mod.rs.
    gtk_window.show_all();
    super::make_click_through(&gtk_window);
    Ok(())
}

pub fn refit(_window: &WebviewWindow, _spec: &SurfaceSpec) {
    // The compositor keeps a layer surface anchored to its output across resolution changes, so
    // there is nothing to re-assert. A monitor appearing or disappearing changes the surface plan,
    // which the watchdog notices on its own.
}

/// Reports where the surface actually landed — see `SurfaceReport`.
pub fn describe(window: &WebviewWindow) -> SurfaceReport {
    let mut report = SurfaceReport::default();
    let Some(shell) = layer_shell() else {
        report.detail("error", MISSING_LIBRARY);
        return report;
    };
    let gtk_window = match window.gtk_window() {
        Ok(gtk_window) => gtk_window,
        Err(e) => {
            report.detail("error", e);
            return report;
        }
    };
    let raw: *mut gtk::ffi::GtkWindow = gtk_window.to_glib_none().0;
    // Safety: a live GtkWindow pointer, and the library was loaded from the same process.
    unsafe {
        report.placed = (shell.is_layer_window)(raw as *mut c_void) != 0;
        report.detail("compositor_supports_layer_shell", (shell.is_supported)() != 0);
    }
    report.visible = gtk_window.is_visible();
    report.detail("mapped", gtk_window.is_mapped());
    report
}
