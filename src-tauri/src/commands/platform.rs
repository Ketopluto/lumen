use crate::services::desktop::{self, Capabilities};

/// What this desktop supports. Also injected into every window as `window.__LUMEN__` before the
/// first paint; this command is the fallback for `vite dev` in a plain browser tab.
#[tauri::command]
pub fn get_capabilities() -> Capabilities {
    desktop::capabilities().clone()
}

/// A monitor as the UI shows it: numbered, named, and with the size people recognise it by.
#[derive(serde::Serialize)]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}

/// Every monitor Lumen can see, in the order it numbers them — the same order the wallpaper
/// engine uses, so an index means the same thing on both sides.
#[tauri::command]
pub fn get_monitors(app: tauri::AppHandle) -> Vec<MonitorInfo> {
    let primary = app.primary_monitor().ok().flatten().map(|monitor| *monitor.position());
    app.available_monitors()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| MonitorInfo {
            index,
            name: monitor
                .name()
                .cloned()
                .unwrap_or_else(|| format!("Display {}", index + 1)),
            width: monitor.size().width,
            height: monitor.size().height,
            primary: primary.is_some_and(|position| position == *monitor.position()),
        })
        .collect()
}
