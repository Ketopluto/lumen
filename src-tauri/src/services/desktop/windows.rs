//! Windows: the live wallpaper window becomes a child of the desktop's WorkerW, behind the icons.

use super::{Capabilities, MonitorInfo, MonitorTarget, SurfaceReport, SurfaceSpec};
use crate::models::FitMode;
use std::collections::BTreeMap;
use std::ffi::c_void;
use tauri::{AppHandle, WebviewWindow};
use windows::core::{w, PCWSTR};
use windows::core::{HSTRING, PWSTR};
use windows::Win32::Foundation::{BOOL, COLORREF, FALSE, HWND, LPARAM, RECT, TRUE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
    MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::UI::Shell::{
    DesktopWallpaper, IDesktopWallpaper, DESKTOP_WALLPAPER_POSITION, DWPOS_CENTER, DWPOS_FILL, DWPOS_FIT, DWPOS_SPAN,
    DWPOS_STRETCH, DWPOS_TILE,
};
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn capabilities() -> Capabilities {
    let mut caps = Capabilities::full("windows");
    // IDesktopWallpaper sets a still image per monitor, and a live surface can be sized to one.
    caps.per_monitor_static = true;
    caps
}

/// One window spanning the virtual screen covers every monitor.
pub fn plan_surfaces(_app: &AppHandle) -> Vec<SurfaceSpec> {
    SurfaceSpec::spanning()
}

/// Every monitor in physical pixels, left to right, then top to bottom.
///
/// Tauri reports a scaled monitor in logical pixels, so a 2400x1350 screen at 125% comes back as
/// 1920x1080 at the wrong origin. Win32 is asked directly instead; this is also safe from any
/// thread, which the watchdog needs.
pub fn monitor_layout(_app: &AppHandle) -> Vec<MonitorInfo> {
    let mut monitors: Vec<MonitorInfo> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(collect_monitor),
            LPARAM(&mut monitors as *mut Vec<MonitorInfo> as isize),
        );
    }
    monitors.sort_by_key(|monitor| (monitor.bounds.0, monitor.bounds.1));
    monitors
}

unsafe extern "system" fn collect_monitor(monitor: HMONITOR, _dc: HDC, _rect: *mut RECT, out: LPARAM) -> BOOL {
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    if !GetMonitorInfoW(monitor, &mut info.monitorInfo as *mut MONITORINFO).as_bool() {
        return TRUE;
    }
    let rect = info.monitorInfo.rcMonitor;
    let name = String::from_utf16_lossy(&info.szDevice)
        .trim_end_matches('\0')
        .trim_start_matches(r"\\.\")
        .to_string();
    let monitors = &mut *(out.0 as *mut Vec<MonitorInfo>);
    monitors.push(MonitorInfo {
        name: if name.is_empty() {
            format!("Display {}", monitors.len() + 1)
        } else {
            name
        },
        bounds: (
            rect.left,
            rect.top,
            (rect.right - rect.left) as u32,
            (rect.bottom - rect.top) as u32,
        ),
        primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
    });
    TRUE
}

/// Read-only: what the shell thinks its monitors are, and what each is showing. The per-monitor
/// still image is chosen by matching these rectangles against `monitor_layout`, so when a
/// wallpaper lands on the wrong screen this is where the two views disagree.
pub fn describe_monitors() -> Vec<String> {
    let mut lines = Vec::new();
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let desktop: IDesktopWallpaper = match CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL) {
            Ok(desktop) => desktop,
            Err(e) => {
                lines.push(format!("shell: unavailable ({})", e));
                return lines;
            }
        };
        let count = desktop.GetMonitorDevicePathCount().unwrap_or(0);
        for index in 0..count {
            let Ok(id) = desktop.GetMonitorDevicePathAt(index) else {
                continue;
            };
            let rect = desktop
                .GetMonitorRECT(PCWSTR(id.0))
                .map(|r| (r.left, r.top, r.right - r.left, r.bottom - r.top));
            let current = desktop
                .GetWallpaper(PCWSTR(id.0))
                .ok()
                .and_then(|w| w.to_string().ok())
                .unwrap_or_default();
            let path = id.to_string().unwrap_or_default();
            let tail: String = path
                .chars()
                .rev()
                .take(28)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            lines.push(format!(
                "shell[{}] rect={:?} wallpaper={} id=...{}",
                index,
                rect,
                std::path::Path::new(&current)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("(none)"),
                tail
            ));
            CoTaskMemFree(Some(id.0 as *const c_void));
        }
    }
    lines
}

pub fn attach(window: &WebviewWindow, spec: &SurfaceSpec) -> Result<(), String> {
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    unsafe { attach_hwnd(hwnd.0 as isize, spec.bounds) }
}

pub fn refit(window: &WebviewWindow, spec: &SurfaceSpec) {
    if let Ok(hwnd) = window.hwnd() {
        unsafe { keep_fitted(hwnd.0 as isize, spec.bounds) };
    }
}

/// Parent `raw` behind the desktop icons so it renders as the wallpaper.
///
/// Windows 11 24H2+ keeps the icon view (SHELLDLL_DefView) and the wallpaper WorkerW side by side
/// inside Progman, so the window becomes a child of that WorkerW. (A layered sibling of the icons
/// is only a fallback: WebView2 renders black inside layered windows.) Older Windows moves the
/// icons into a new WorkerW, and the wallpaper goes into the WorkerW behind it.
/// Where a surface sits inside the WorkerW. Its client area starts at the top-left of the virtual
/// screen, which is not the origin once a monitor sits left of or above the primary one, so a
/// monitor rectangle has to be shifted into that space.
unsafe fn target_rect(bounds: Option<(i32, i32, u32, u32)>) -> (i32, i32, i32, i32) {
    match bounds {
        Some((x, y, width, height)) => (
            x - GetSystemMetrics(SM_XVIRTUALSCREEN),
            y - GetSystemMetrics(SM_YVIRTUALSCREEN),
            width as i32,
            height as i32,
        ),
        None => (
            0,
            0,
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        ),
    }
}

unsafe fn attach_hwnd(raw: isize, bounds: Option<(i32, i32, u32, u32)>) -> Result<(), String> {
    let hwnd = HWND(raw as *mut c_void);
    let progman = FindWindowW(w!("Progman"), PCWSTR::null())
        .map_err(|_| "Could not find the desktop window (is Explorer running?)".to_string())?;

    // Ask Progman to create the WorkerW that sits behind the icons.
    let mut result = 0usize;
    let _ = SendMessageTimeoutW(
        progman,
        0x052C,
        WPARAM(0xD),
        LPARAM(0x1),
        SMTO_NORMAL,
        1000,
        Some(&mut result),
    );

    let (x, y, cx, cy) = target_rect(bounds);

    let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
    let remove = (WS_POPUP | WS_CAPTION | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX).0 as isize;
    SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !remove) | WS_CHILD.0 as isize);

    let raised_desktop = FindWindowExW(progman, HWND::default(), w!("SHELLDLL_DefView"), PCWSTR::null());
    let raised_worker = FindWindowExW(progman, HWND::default(), w!("WorkerW"), PCWSTR::null());
    if let (Ok(_), Ok(worker)) = (&raised_desktop, &raised_worker) {
        // Windows 11 24H2+: become a child of the WorkerW that paints the wallpaper. It sits
        // below the icons, and needs no WS_EX_LAYERED (WebView2 renders black in layered windows).
        SetParent(hwnd, *worker).map_err(|e| format!("SetParent failed: {}", e))?;
        let _ = SetWindowPos(hwnd, HWND::default(), x, y, cx, cy, SWP_NOACTIVATE);
    } else if let Ok(icons) = raised_desktop {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_LAYERED.0 as isize);
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
        SetParent(hwnd, progman).map_err(|e| format!("SetParent failed: {}", e))?;
        // Directly below the icon view...
        let _ = SetWindowPos(hwnd, icons, x, y, cx, cy, SWP_NOACTIVATE);
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
        let _ = SetWindowPos(hwnd, HWND::default(), x, y, cx, cy, SWP_NOACTIVATE);
    }
    // Reveal it here rather than through Tauri's show(): see the contract in mod.rs.
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    Ok(())
}

/// Re-attaches the player if it lost its desktop parent, and keeps it covering every monitor
/// (the virtual screen changes with resolution or monitor changes).
unsafe fn keep_fitted(raw: isize, bounds: Option<(i32, i32, u32, u32)>) {
    let hwnd = HWND(raw as *mut c_void);
    let attached = matches!(GetParent(hwnd), Ok(p) if !p.0.is_null() && IsWindow(p).as_bool());
    if !attached {
        let _ = attach_hwnd(raw, bounds);
        return;
    }
    let (x, y, cx, cy) = target_rect(bounds);
    let mut rect = RECT::default();
    if GetClientRect(hwnd, &mut rect).is_ok() && (rect.right != cx || rect.bottom != cy) {
        let _ = SetWindowPos(hwnd, HWND::default(), x, y, cx, cy, SWP_NOACTIVATE | SWP_NOZORDER);
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
        if matches!(
            class.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        ) {
            return false;
        }
        let mut rect = RECT::default();
        if GetWindowRect(fg, &mut rect).is_err() {
            return false;
        }
        let monitor = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
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

pub fn set_static_wallpaper(path: &str, fit: &FitMode, target: Option<MonitorTarget>) -> Result<(), String> {
    if let Some(target) = target {
        return unsafe { set_monitor_wallpaper(path, fit, target) };
    }
    let _ = ::wallpaper::set_mode(fit.to_mode());
    ::wallpaper::set_from_path(path).map_err(|e| format!("Failed to set wallpaper: {}", e))
}

fn wallpaper_position(fit: &FitMode) -> DESKTOP_WALLPAPER_POSITION {
    match fit {
        FitMode::Fill => DWPOS_FILL,
        FitMode::Fit => DWPOS_FIT,
        FitMode::Stretch => DWPOS_STRETCH,
        FitMode::Center => DWPOS_CENTER,
        FitMode::Tile => DWPOS_TILE,
        FitMode::Span => DWPOS_SPAN,
    }
}

/// One monitor's still image, through the shell's own wallpaper interface.
///
/// Monitors are identified by device path, and that list is in no particular order, so the right
/// one is found by matching its rectangle against the monitor the caller asked for — falling back
/// to list position only when nothing matches.
unsafe fn set_monitor_wallpaper(path: &str, fit: &FitMode, target: MonitorTarget) -> Result<(), String> {
    // Already initialised on most threads; a second call on the same apartment is harmless.
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    let desktop: IDesktopWallpaper =
        CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL).map_err(|e| format!("Desktop wallpaper: {}", e))?;

    let count = desktop.GetMonitorDevicePathCount().map_err(|e| e.to_string())?;
    let (wanted_x, wanted_y, wanted_width, wanted_height) = target.bounds;
    let mut chosen: Option<PWSTR> = None;
    let mut fallback: Option<PWSTR> = None;
    for index in 0..count {
        let Ok(id) = desktop.GetMonitorDevicePathAt(index) else {
            continue;
        };
        let matches_bounds = desktop.GetMonitorRECT(PCWSTR(id.0)).is_ok_and(|rect| {
            rect.left == wanted_x
                && rect.top == wanted_y
                && rect.right - rect.left == wanted_width as i32
                && rect.bottom - rect.top == wanted_height as i32
        });
        if matches_bounds {
            chosen = Some(id);
            break;
        }
        if index as usize == target.index {
            fallback = Some(id);
        } else {
            CoTaskMemFree(Some(id.0 as *const c_void));
        }
    }
    let id = chosen.or(fallback).ok_or("That monitor is no longer connected")?;
    let result = desktop
        .SetWallpaper(PCWSTR(id.0), &HSTRING::from(path))
        .map_err(|e| format!("Failed to set wallpaper on that monitor: {}", e));
    CoTaskMemFree(Some(id.0 as *const c_void));
    result?;
    // The fit applies to the whole desktop: Windows keeps one position for every monitor.
    let _ = desktop.SetPosition(wallpaper_position(fit));
    Ok(())
}

/// Asks the shell to repaint by writing back the position it already has: every monitor redraws
/// and no image is replaced, which matters once the monitors hold different wallpapers.
pub fn redraw_wallpaper(_fit: &FitMode) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if let Ok(desktop) = CoCreateInstance::<_, IDesktopWallpaper>(&DesktopWallpaper, None, CLSCTX_ALL) {
            if let Ok(position) = desktop.GetPosition() {
                let _ = desktop.SetPosition(position);
            }
        }
    }
}

pub fn current_static_wallpaper() -> Option<String> {
    ::wallpaper::get().ok().filter(|p| !p.is_empty())
}

/// The still image on each of `targets`, read straight back from the shell.
///
/// Same identification problem as `set_monitor_wallpaper`: device paths come in no particular
/// order, so each monitor is found by its rectangle and only falls back to list position. A
/// monitor the shell has nothing for is simply absent from the map.
pub fn current_monitor_wallpapers(targets: &[MonitorTarget]) -> BTreeMap<usize, String> {
    let mut found = BTreeMap::new();
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let Ok(desktop) = CoCreateInstance::<_, IDesktopWallpaper>(&DesktopWallpaper, None, CLSCTX_ALL) else {
            return found;
        };
        let count = desktop.GetMonitorDevicePathCount().unwrap_or(0);
        for position in 0..count {
            let Ok(id) = desktop.GetMonitorDevicePathAt(position) else {
                continue;
            };
            let rect = desktop.GetMonitorRECT(PCWSTR(id.0)).ok();
            let index = targets
                .iter()
                .find(|target| {
                    rect.is_some_and(|r| {
                        let (x, y, width, height) = target.bounds;
                        r.left == x
                            && r.top == y
                            && r.right - r.left == width as i32
                            && r.bottom - r.top == height as i32
                    })
                })
                .map(|target| target.index)
                .or_else(|| {
                    let position = position as usize;
                    targets.iter().any(|t| t.index == position).then_some(position)
                });
            let path = desktop
                .GetWallpaper(PCWSTR(id.0))
                .ok()
                .and_then(|w| w.to_string().ok())
                .filter(|p| !p.is_empty());
            CoTaskMemFree(Some(id.0 as *const c_void));
            if let (Some(index), Some(path)) = (index, path) {
                found.entry(index).or_insert(path);
            }
        }
    }
    found
}

/// Reports where the surface actually landed — see `SurfaceReport`.
pub fn describe(window: &WebviewWindow) -> SurfaceReport {
    let mut report = SurfaceReport::default();
    let Ok(hwnd) = window.hwnd() else {
        report.detail("error", "this window has no HWND");
        return report;
    };
    unsafe {
        let hwnd = HWND(hwnd.0);
        let parent = GetAncestor(hwnd, GA_PARENT);
        let mut class = [0u16; 64];
        let read = GetClassNameW(parent, &mut class);
        let parent_class = String::from_utf16_lossy(&class[..read.max(0) as usize]);
        report.visible = IsWindowVisible(hwnd).as_bool();
        report.placed = parent_class == "WorkerW" || parent_class == "Progman";
        report
            .detail("parent_class", &parent_class)
            .detail("parent", format!("{:?}", parent.0))
            .detail(
                "child_style",
                format!("{}", GetWindowLongPtrW(hwnd, GWL_STYLE) & WS_CHILD.0 as isize != 0),
            );
    }
    report
}

#[cfg(test)]
mod monitor_tests {
    use super::*;

    /// Proves a still image can be set on one monitor without touching the other.
    ///
    /// Nothing visible changes: the image already on the second monitor is copied to a new path
    /// and that copy is set there, so the pixels are identical. What changes is the path the shell
    /// reports per monitor, which is exactly what per-monitor independence means. The original
    /// path is put back before anything is asserted.
    ///
    /// `cargo test monitor_tests::each_monitor -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn each_monitor_keeps_its_own_still_image() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let desktop: IDesktopWallpaper =
                CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL).expect("IDesktopWallpaper");
            let count = desktop.GetMonitorDevicePathCount().expect("monitor count");
            if count < 2 {
                println!("skipped: needs two monitors, found {}", count);
                return;
            }

            let read = |index: u32| -> (RECT, String) {
                let id = desktop.GetMonitorDevicePathAt(index).expect("device path");
                let rect = desktop.GetMonitorRECT(PCWSTR(id.0)).expect("monitor rect");
                let current = desktop
                    .GetWallpaper(PCWSTR(id.0))
                    .ok()
                    .and_then(|w| w.to_string().ok())
                    .unwrap_or_default();
                CoTaskMemFree(Some(id.0 as *const c_void));
                (rect, current)
            };

            let (_, first_before) = read(0);
            let (second_rect, second_before) = read(1);
            println!("monitor 0 shows {}", first_before);
            println!("monitor 1 shows {}", second_before);
            assert!(!second_before.is_empty(), "the second monitor has no wallpaper to copy");

            // A byte-identical copy at a different path: same pixels, different identity.
            let copy = std::env::temp_dir().join("lumen_monitor_probe.jpg");
            std::fs::copy(&second_before, &copy).expect("copy the current wallpaper");

            let target = MonitorTarget {
                index: 1,
                bounds: (
                    second_rect.left,
                    second_rect.top,
                    (second_rect.right - second_rect.left) as u32,
                    (second_rect.bottom - second_rect.top) as u32,
                ),
            };
            let applied = set_static_wallpaper(&copy.to_string_lossy(), &FitMode::Fill, Some(target));

            let (_, first_after) = read(0);
            let (_, second_after) = read(1);

            // Put the original back before judging anything, so a failure cannot leave it changed.
            let restored = set_static_wallpaper(&second_before, &FitMode::Fill, Some(target));
            let _ = std::fs::remove_file(&copy);

            println!("after setting monitor 1: monitor 0 = {}", first_after);
            println!("after setting monitor 1: monitor 1 = {}", second_after);
            println!("restore: {:?}", restored);

            applied.expect("setting one monitor's wallpaper");
            assert_eq!(
                first_after, first_before,
                "monitor 0's wallpaper changed; the monitors are not independent"
            );
            assert_eq!(
                second_after.to_lowercase(),
                copy.to_string_lossy().to_lowercase(),
                "monitor 1 did not take the image it was given"
            );
            restored.expect("restoring the original wallpaper");
        }
    }

    /// Per-monitor still images hinge on matching the shell's monitor list — which is in no
    /// documented order — against a rectangle. This reads that list without changing anything, so
    /// the mapping can be checked on a real multi-monitor desktop:
    /// `cargo test monitor_tests -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn the_shell_identifies_every_monitor_by_rectangle() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let desktop: IDesktopWallpaper =
                CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL).expect("IDesktopWallpaper");
            let count = desktop.GetMonitorDevicePathCount().expect("monitor count");
            println!("the shell reports {} monitor(s)", count);
            let mut rectangles = 0;
            for index in 0..count {
                let id = desktop.GetMonitorDevicePathAt(index).expect("device path");
                match desktop.GetMonitorRECT(PCWSTR(id.0)) {
                    Ok(rect) => {
                        rectangles += 1;
                        println!(
                            "  {}: {}x{} at ({}, {})",
                            index,
                            rect.right - rect.left,
                            rect.bottom - rect.top,
                            rect.left,
                            rect.top
                        );
                    }
                    // A device path can outlive the monitor it names; those are simply skipped.
                    Err(e) => println!("  {}: no rectangle ({})", index, e),
                }
                CoTaskMemFree(Some(id.0 as *const c_void));
            }
            assert!(rectangles > 0, "no monitor could be identified by rectangle");
        }
    }
}
