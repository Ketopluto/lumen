//! Windows: the live wallpaper window becomes a child of the desktop's WorkerW, behind the icons.

use super::{Capabilities, SurfaceSpec};
use crate::models::FitMode;
use std::ffi::c_void;
use tauri::{AppHandle, WebviewWindow};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{BOOL, COLORREF, FALSE, HWND, LPARAM, RECT, TRUE, WPARAM};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn capabilities() -> Capabilities {
    Capabilities::full("windows")
}

/// One window spanning the virtual screen covers every monitor.
pub fn plan_surfaces(_app: &AppHandle) -> Vec<SurfaceSpec> {
    SurfaceSpec::spanning()
}

pub fn attach(window: &WebviewWindow, _spec: &SurfaceSpec) -> Result<(), String> {
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    unsafe { attach_hwnd(hwnd.0 as isize) }
}

pub fn refit(window: &WebviewWindow, _spec: &SurfaceSpec) {
    if let Ok(hwnd) = window.hwnd() {
        unsafe { keep_fitted(hwnd.0 as isize) };
    }
}

/// Parent `raw` behind the desktop icons so it renders as the wallpaper.
///
/// Windows 11 24H2+ keeps the icon view (SHELLDLL_DefView) and the wallpaper WorkerW side by side
/// inside Progman, so the window becomes a child of that WorkerW. (A layered sibling of the icons
/// is only a fallback: WebView2 renders black inside layered windows.) Older Windows moves the
/// icons into a new WorkerW, and the wallpaper goes into the WorkerW behind it.
unsafe fn attach_hwnd(raw: isize) -> Result<(), String> {
    let hwnd = HWND(raw as *mut c_void);
    let progman = FindWindowW(w!("Progman"), PCWSTR::null())
        .map_err(|_| "Could not find the desktop window (is Explorer running?)".to_string())?;

    // Ask Progman to create the WorkerW that sits behind the icons.
    let mut result = 0usize;
    let _ = SendMessageTimeoutW(progman, 0x052C, WPARAM(0xD), LPARAM(0x1), SMTO_NORMAL, 1000, Some(&mut result));

    let cx = GetSystemMetrics(SM_CXVIRTUALSCREEN);
    let cy = GetSystemMetrics(SM_CYVIRTUALSCREEN);

    let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
    let remove = (WS_POPUP | WS_CAPTION | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX).0 as isize;
    SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !remove) | WS_CHILD.0 as isize);

    let raised_desktop = FindWindowExW(progman, HWND::default(), w!("SHELLDLL_DefView"), PCWSTR::null());
    let raised_worker = FindWindowExW(progman, HWND::default(), w!("WorkerW"), PCWSTR::null());
    if let (Ok(_), Ok(worker)) = (&raised_desktop, &raised_worker) {
        // Windows 11 24H2+: become a child of the WorkerW that paints the wallpaper. It sits
        // below the icons, and needs no WS_EX_LAYERED (WebView2 renders black in layered windows).
        SetParent(hwnd, *worker).map_err(|e| format!("SetParent failed: {}", e))?;
        let _ = SetWindowPos(hwnd, HWND::default(), 0, 0, cx, cy, SWP_NOACTIVATE);
    } else if let Ok(icons) = raised_desktop {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_LAYERED.0 as isize);
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
        SetParent(hwnd, progman).map_err(|e| format!("SetParent failed: {}", e))?;
        // Directly below the icon view...
        let _ = SetWindowPos(hwnd, icons, 0, 0, cx, cy, SWP_NOACTIVATE);
        // ...and above the WorkerW that paints the static wallpaper.
        if let Ok(worker) = FindWindowExW(progman, HWND::default(), w!("WorkerW"), PCWSTR::null()) {
            let _ = SetWindowPos(worker, hwnd, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    } else {
        let mut worker = HWND::default();
        let _ = EnumWindows(Some(find_worker), LPARAM(&mut worker as *mut HWND as isize));
        if worker.0.is_null() {
            return Err("Could not find the desktop WorkerW window".into());
        }
        SetParent(hwnd, worker).map_err(|e| format!("SetParent failed: {}", e))?;
        let _ = SetWindowPos(hwnd, HWND::default(), 0, 0, cx, cy, SWP_NOACTIVATE);
    }
    // Reveal it here rather than through Tauri's show(): see the contract in mod.rs.
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    Ok(())
}

/// Re-attaches the player if it lost its desktop parent, and keeps it covering every monitor
/// (the virtual screen changes with resolution or monitor changes).
unsafe fn keep_fitted(raw: isize) {
    let hwnd = HWND(raw as *mut c_void);
    let attached = matches!(GetParent(hwnd), Ok(p) if !p.0.is_null() && IsWindow(p).as_bool());
    if !attached {
        let _ = attach_hwnd(raw);
        return;
    }
    let cx = GetSystemMetrics(SM_CXVIRTUALSCREEN);
    let cy = GetSystemMetrics(SM_CYVIRTUALSCREEN);
    let mut rect = RECT::default();
    if GetClientRect(hwnd, &mut rect).is_ok() && (rect.right != cx || rect.bottom != cy) {
        let _ = SetWindowPos(hwnd, HWND::default(), 0, 0, cx, cy, SWP_NOACTIVATE | SWP_NOZORDER);
    }
}

/// Finds the WorkerW that follows the top-level window hosting SHELLDLL_DefView.
unsafe extern "system" fn find_worker(top: HWND, out: LPARAM) -> BOOL {
    if FindWindowExW(top, HWND::default(), w!("SHELLDLL_DefView"), PCWSTR::null()).is_ok() {
        if let Ok(worker) = FindWindowExW(HWND::default(), top, w!("WorkerW"), PCWSTR::null()) {
            *(out.0 as *mut HWND) = worker;
            return FALSE;
        }
    }
    TRUE
}

pub fn fullscreen_app_active() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return false;
        }
        let mut class = [0u16; 64];
        let len = GetClassNameW(fg, &mut class).max(0) as usize;
        let class = String::from_utf16_lossy(&class[..len]);
        if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
            return false;
        }
        let mut rect = RECT::default();
        if GetWindowRect(fg, &mut rect).is_err() {
            return false;
        }
        let monitor = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return false;
        }
        let m = info.rcMonitor;
        rect.left <= m.left && rect.top <= m.top && rect.right >= m.right && rect.bottom >= m.bottom
    }
}

pub fn on_battery() -> bool {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status).is_ok() && status.ACLineStatus == 0 }
}

pub fn set_autostart(enable: bool) -> Result<(), String> {
    // Only the Run key. Registering a scheduled task as well made Windows Defender's behaviour
    // heuristics flag Lumen as malware (Behavior:Win32/Execution.A!ml) and quarantine it.
    use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE};
    use winreg::RegKey;
    let run = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_SET_VALUE | KEY_QUERY_VALUE)
        .map_err(|e| e.to_string())?;
    let current = run.get_value::<String, _>("Lumen").ok();
    if enable {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let value = format!("\"{}\" --minimized", exe.display());
        // Write only when it actually changes: rewriting an autostart entry on every launch is
        // another pattern antivirus heuristics score against.
        if current.as_deref() == Some(value.as_str()) {
            return Ok(());
        }
        run.set_value("Lumen", &value).map_err(|e| e.to_string())
    } else {
        if current.is_none() {
            return Ok(());
        }
        match run.delete_value("Lumen") {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

pub fn set_static_wallpaper(path: &str, fit: &FitMode) -> Result<(), String> {
    let _ = ::wallpaper::set_mode(fit.to_mode());
    ::wallpaper::set_from_path(path).map_err(|e| format!("Failed to set wallpaper: {}", e))
}

pub fn current_static_wallpaper() -> Option<String> {
    ::wallpaper::get().ok().filter(|p| !p.is_empty())
}
