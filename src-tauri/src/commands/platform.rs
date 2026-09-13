use crate::services::desktop::{self, Capabilities};

/// What this desktop supports. Also injected into every window as `window.__LUMEN__` before the
/// first paint; this command is the fallback for `vite dev` in a plain browser tab.
#[tauri::command]
pub fn get_capabilities() -> Capabilities {
    desktop::capabilities().clone()
}
