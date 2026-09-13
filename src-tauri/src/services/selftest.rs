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
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

pub const SKIPPED: i32 = 2;

pub fn requested(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--selftest")
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

    let specs = desktop::plan_surfaces(app);
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

/// Straight to stdout and flushed: the log file is no use to a CI runner.
fn report_line(line: &str) {
    println!("[selftest] {}", line);
    let _ = std::io::stdout().flush();
}
