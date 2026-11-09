use crate::services::desktop::{self, Capabilities};

/// What this desktop supports. Also injected into every window as `window.__LUMEN__` before the
/// first paint; this command is the fallback for `vite dev` in a plain browser tab.
#[tauri::command]
pub fn get_capabilities() -> Capabilities {
    desktop::capabilities().clone()
}

/// A monitor as the UI shows it: numbered, named, and with the size people recognise it by.
#[derive(serde::Serialize)]
pub struct MonitorRow {
    pub index: usize,
    pub name: String,
    /// Physical position, so the UI can draw the monitors where they really are.
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}

/// Every monitor Lumen can see, straight from the same list the wallpaper engine indexes into, so
/// monitor 2 in the picker is monitor 2 on the desktop.
#[tauri::command]
pub fn get_monitors(app: tauri::AppHandle) -> Vec<MonitorRow> {
    desktop::monitor_layout(&app)
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| MonitorRow {
            index,
            name: monitor.name,
            x: monitor.bounds.0,
            y: monitor.bounds.1,
            width: monitor.bounds.2,
            height: monitor.bounds.3,
            primary: monitor.primary,
        })
        .collect()
}
