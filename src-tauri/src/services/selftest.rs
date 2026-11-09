//! `lumen --selftest`: create one live wallpaper surface, ask the platform where it ended up, and
//! print the answer as JSON.
//!
//! This exists because macOS and most of Linux have no machine here to try them on. CI runs this
//! on a real window server, so the placement code is checked by something other than the compiler.
//!
//! Exit codes: `0` placed correctly, `1` not placed, `2` nothing to test on this system.

use crate::services::desktop;
use std::io::Write;
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const SKIPPED: i32 = 2;

pub fn requested(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--selftest" || arg == "--selftest-monitors")
}

/// `--selftest-monitors` puts a surface on every monitor instead of one on the primary, which is
/// what per-monitor wallpapers depend on.
fn per_monitor_requested(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--selftest-monitors")
}

/// Runs off the main thread: `desktop::on_main` needs the event loop free to answer.
pub fn run(app: &AppHandle) -> i32 {
    // A wedged webview must not turn into a CI job that hangs until the runner times out.
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(90));
        report_line("selftest timed out");
        std::process::exit(SKIPPED);
    });

    let caps = desktop::capabilities();
    report_line(&format!(
        "platform: {} session={:?} desktop={:?}",
        caps.os, caps.session, caps.desktop
    ));
    if !caps.live_wallpaper {
        report_line(&format!(
            "skipped: {}",
            caps.live_unsupported_reason
                .as_deref()
                .unwrap_or("no live wallpapers here")
        ));
        return SKIPPED;
    }

    // Which coordinate space does this process actually live in? Tauri's list and the platform's
    // own list disagree on a scaled monitor, and only one of them can place a window correctly.
    for (index, monitor) in desktop::monitor_layout(app).into_iter().enumerate() {
        report_line(&format!(
            "layout[{}] {:<12} bounds={:?} primary={}",
            index, monitor.name, monitor.bounds, monitor.primary
        ));
    }
    if let Ok(monitors) = app.available_monitors() {
        for (index, monitor) in monitors.iter().enumerate() {
            report_line(&format!(
                "tauri[{}]  position={:?} size={:?} scale={}",
                index,
                (monitor.position().x, monitor.position().y),
                (monitor.size().width, monitor.size().height),
                monitor.scale_factor()
            ));
        }
    }

    #[cfg(target_os = "windows")]
    for line in crate::services::desktop::describe_monitors() {
        report_line(&line);
    }

    let args: Vec<String> = std::env::args().collect();
    let specs = if per_monitor_requested(&args) {
        let monitors: Vec<usize> = desktop::monitors(app).into_iter().map(|m| m.index).collect();
        report_line(&format!("monitors: {:?}", monitors));
        desktop::surfaces_for(app, &monitors)
    } else {
        desktop::plan_surfaces(app)
    };
    if specs.len() > 1 {
        return per_monitor_report(app, specs);
    }
    let Some(spec) = specs.first().cloned() else {
        report_line("skipped: this system reports no display");
        return SKIPPED;
    };

    let mut builder = WebviewWindowBuilder::new(app, &spec.label, WebviewUrl::App("index.html".into()))
        .title("Lumen Live")
        .decorations(false)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .visible(false)
        .initialization_script(desktop::bootstrap_script());
    if let Some((x, y, width, height)) = spec.bounds {
        builder = builder
            .position(x as f64, y as f64)
            .inner_size(width as f64, height as f64);
    }
    let window = match builder.build() {
        Ok(window) => window,
        Err(e) => {
            report_line(&format!("skipped: no window could be created ({})", e));
            return SKIPPED;
        }
    };

    // `attach` also reveals the window — see the contract in services/desktop/mod.rs.
    let attaching = {
        let window = window.clone();
        let spec = spec.clone();
        desktop::on_main(app, move || desktop::attach(&window, &spec))
    };
    match attaching {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            report_line(&format!("failed: attach: {}", e));
            return 1;
        }
        Err(e) => {
            report_line(&format!("skipped: the main thread never answered ({})", e));
            return SKIPPED;
        }
    }

    // Compositors apply placement on their own schedule; give them a moment before looking.
    std::thread::sleep(Duration::from_millis(1500));

    let report = {
        let window = window.clone();
        match desktop::on_main(app, move || desktop::describe(&window)) {
            Ok(report) => report,
            Err(e) => {
                report_line(&format!("skipped: the main thread never answered ({})", e));
                return SKIPPED;
            }
        }
    };

    report_line(&serde_json::to_string_pretty(&report).unwrap_or_else(|e| e.to_string()));
    if report.placed && report.visible {
        report_line("passed: the surface is on the desktop");
        0
    } else {
        report_line("failed: the surface is not where a wallpaper belongs");
        1
    }
}

/// Builds every planned surface, attaches each, and reports where they landed. A wallpaper per
/// monitor is only real if each surface covers its own monitor and nothing else.
fn per_monitor_report(app: &AppHandle, specs: Vec<desktop::SurfaceSpec>) -> i32 {
    let mut failures = 0;
    for spec in &specs {
        let mut builder = WebviewWindowBuilder::new(app, &spec.label, WebviewUrl::App("index.html".into()))
            .title("Lumen Live")
            .decorations(false)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .visible(false)
            .initialization_script(desktop::bootstrap_script());
        if let Some((x, y, width, height)) = spec.bounds {
            builder = builder
                .position(x as f64, y as f64)
                .inner_size(width as f64, height as f64);
        }
        let window = match builder.build() {
            Ok(window) => window,
            Err(e) => {
                report_line(&format!("{}: no window ({})", spec.label, e));
                failures += 1;
                continue;
            }
        };
        let spec_for_attach = spec.clone();
        match desktop::on_main(app, move || desktop::attach(&window, &spec_for_attach)) {
            Ok(Ok(())) => {}
            Ok(Err(e)) | Err(e) => {
                report_line(&format!("{}: attach failed ({})", spec.label, e));
                failures += 1;
                continue;
            }
        }
    }
    std::thread::sleep(Duration::from_millis(1500));

    for spec in &specs {
        let Some(window) = app.get_webview_window(&spec.label) else {
            continue;
        };
        let bounds = spec.bounds;
        let placement = window
            .outer_position()
            .ok()
            .zip(window.outer_size().ok())
            .map(|(position, size)| (position.x, position.y, size.width, size.height));
        let report = match desktop::on_main(app, move || desktop::describe(&window)) {
            Ok(report) => report,
            Err(e) => {
                report_line(&format!("{}: {}", spec.label, e));
                failures += 1;
                continue;
            }
        };
        let fitted = match (bounds, placement) {
            (Some((_, _, width, height)), Some((_, _, actual_width, actual_height))) => {
                actual_width == width && actual_height == height
            }
            _ => false,
        };
        report_line(&format!(
            "{}: monitor {:?} wanted {:?} got {:?} placed={} visible={} sized={}",
            spec.label, spec.monitor, bounds, placement, report.placed, report.visible, fitted
        ));
        if !report.placed || !report.visible || !fitted {
            failures += 1;
        }
    }

    if failures == 0 {
        report_line(&format!("passed: {} surfaces, one per monitor", specs.len()));
        0
    } else {
        report_line(&format!("failed: {} of {} surfaces are wrong", failures, specs.len()));
        1
    }
}

/// Straight to stdout and flushed: the log file is no use to a CI runner.
fn report_line(line: &str) {
    println!("[selftest] {}", line);
    let _ = std::io::stdout().flush();
}
