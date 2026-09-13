//! X11: the player becomes a desktop-type window, kept below everything and click-through.
//!
//! Every X11 desktop (GNOME on Xorg, KDE, XFCE, Cinnamon, MATE, tiling WMs) understands
//! `_NET_WM_WINDOW_TYPE_DESKTOP`, which puts the window behind normal windows and behind the icon
//! layer, while `keep_below` and an empty input shape keep it out of the way of clicks.

use super::SurfaceSpec;
use gtk::gdk;
use gtk::prelude::*;
use tauri::WebviewWindow;

/// The union of every monitor, i.e. the whole desktop, in logical pixels.
fn desktop_bounds() -> Option<gdk::Rectangle> {
    let display = gdk::Display::default()?;
    let mut union: Option<gdk::Rectangle> = None;
    for index in 0..display.n_monitors() {
        let Some(monitor) = display.monitor(index) else {
            continue;
        };
        let geometry = monitor.geometry();
        union = Some(match union {
            Some(current) => current.union(&geometry),
            None => geometry,
        });
    }
    union
}

pub fn attach(window: &WebviewWindow, _spec: &SurfaceSpec) -> Result<(), String> {
    let gtk_window = window.gtk_window().map_err(|e| e.to_string())?;

    // Must precede realize, which `.visible(false)` guarantees has not happened yet.
    gtk_window.set_type_hint(gdk::WindowTypeHint::Desktop);
    gtk_window.set_decorated(false);
    gtk_window.set_skip_taskbar_hint(true);
    gtk_window.set_skip_pager_hint(true);
    gtk_window.set_accept_focus(false);
    gtk_window.set_keep_below(true);
    gtk_window.stick(); // every workspace

    if let Some(bounds) = desktop_bounds() {
        gtk_window.move_(bounds.x(), bounds.y());
        gtk_window.resize(bounds.width(), bounds.height());
    }

    // The only place a live surface becomes visible on Linux — see the contract in mod.rs.
    gtk_window.show_all();
    if let Some(gdk_window) = gtk_window.window() {
        gdk_window.lower();
    }
    make_click_through(&gtk_window);
    Ok(())
}

/// Clicks, scrolls and the desktop's own right-click menu must pass straight through the wallpaper.
fn make_click_through(gtk_window: &gtk::ApplicationWindow) {
    let empty = gtk::cairo::Region::create();
    gtk_window.input_shape_combine_region(Some(&empty));
    if let Some(gdk_window) = gtk_window.window() {
        gdk_window.set_pass_through(true);
    }
}

pub fn refit(window: &WebviewWindow, _spec: &SurfaceSpec) {
    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };
    // Window managers reset these when they restart or when the layout changes.
    gtk_window.set_keep_below(true);
    let Some(bounds) = desktop_bounds() else {
        return;
    };
    let (width, height) = gtk_window.size();
    if width != bounds.width() || height != bounds.height() {
        gtk_window.move_(bounds.x(), bounds.y());
        gtk_window.resize(bounds.width(), bounds.height());
        make_click_through(&gtk_window);
    }
}
