//! Starting Lumen with the session.
//!
//! macOS and Linux go through `tauri-plugin-autostart` (a LaunchAgent, an XDG `.desktop` entry).
//! Windows keeps its own Run-key code: the plugin's backend writes the value as
//! `path --minimized` with no quotes, which breaks as soon as the install path contains a space.
//! Deliberately nothing else on Windows — registering a scheduled task alongside the Run key made
//! Defender quarantine Lumen as `Behavior:Win32/Execution.A!ml`.

pub use imp::set;

#[cfg(target_os = "windows")]
mod imp {
    use tauri::AppHandle;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE};
    use winreg::RegKey;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "Lumen";

    fn desired_value() -> Result<String, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        Ok(format!("\"{}\" --minimized", exe.display()))
    }

    fn current_value() -> Option<String> {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(RUN_KEY, KEY_QUERY_VALUE)
            .ok()?
            .get_value::<String, _>(VALUE_NAME)
            .ok()
    }

    pub fn set(_app: &AppHandle, enable: bool) -> Result<(), String> {
        let run = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE | KEY_QUERY_VALUE)
            .map_err(|e| e.to_string())?;
        let current = current_value();
        if enable {
            let value = desired_value()?;
            // Write only when it actually changes — which still catches a reinstall moving the exe.
            // Rewriting an autostart entry on every launch is a pattern antivirus heuristics score
            // against.
            if current.as_deref() == Some(value.as_str()) {
                return Ok(());
            }
            run.set_value(VALUE_NAME, &value).map_err(|e| e.to_string())
        } else {
            if current.is_none() {
                return Ok(());
            }
            match run.delete_value(VALUE_NAME) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use tauri::AppHandle;
    use tauri_plugin_autostart::ManagerExt;

    fn is_enabled(app: &AppHandle) -> bool {
        app.autolaunch().is_enabled().unwrap_or(false)
    }

    pub fn set(app: &AppHandle, enable: bool) -> Result<(), String> {
        let manager = app.autolaunch();
        if enable {
            // Idempotent, and rewriting it repairs the entry after the app moved.
            manager.enable().map_err(|e| e.to_string())
        } else if is_enabled(app) {
            manager.disable().map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }
}
