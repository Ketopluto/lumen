mod commands;
mod models;
mod services;
mod utils;

use services::database::Database;
use services::scheduler::SlideshowScheduler;
use services::wallpaper_engine::{self, LiveState};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    // The UI is unloaded while in the tray; rebuild it off the event-loop thread.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = create_main(&app) {
            log::warn!("Could not open the main window: {}", e);
        }
    });
}

/// The main window isn't kept alive in the tray: closing it frees the whole UI webview,
/// so Lumen costs only a few MB in the background (plus the live wallpaper, if one is playing).
fn create_main(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window("main").is_some() {
        return Ok(());
    }
    let mut config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .cloned()
        .ok_or("missing main window config")?;
    config.visible = true;
    let window = tauri::WebviewWindowBuilder::from_config(app, &config)
        .and_then(|builder| builder.build())
        .map_err(|e| e.to_string())?;
    let _ = window.set_focus();
    Ok(())
}

/// `lumen.exe <image-or-video>` sets that file as the wallpaper — also when Lumen is already
/// running (the second instance forwards its arguments here). Returns true if a file was given.
fn apply_from_args(app: &AppHandle, args: &[String], cwd: &str) -> bool {
    let Some(path) = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .map(|a| std::path::Path::new(cwd).join(a))
        .find(|p| p.is_file())
    else {
        return false;
    };
    let path = path.to_string_lossy().to_string();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let db = app.state::<Arc<Database>>().inner().clone();
        let fit = db.settings().fit_mode;
        let info = models::WallpaperInfo::from_local(&path);
        if let Err(e) = wallpaper_engine::apply(&app, &db, &info, &fit).await {
            log::warn!("Could not set {} as wallpaper: {}", path, e);
        }
    });
    true
}

fn quit(app: &AppHandle) {
    wallpaper_engine::stop_live(app, true);
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    utils::migrate_legacy_data();
    services::logging::init();
    log::info!(
        "Lumen {} starting (args: {})",
        env!("CARGO_PKG_VERSION"),
        std::env::args().skip(1).collect::<Vec<_>>().join(" ")
    );
    let db = Arc::new(
        Database::new(&utils::app_data_dir().join("lumen.db")).expect("Failed to open the Lumen database"),
    );
    let scheduler = Arc::new(SlideshowScheduler::new(db.clone()));
    let start_minimized = std::env::args().any(|a| a == "--minimized");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            if !apply_from_args(app, &args, &cwd) {
                show_main(app);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(db.clone())
        .manage(scheduler.clone())
        .manage(LiveState::default())
        .invoke_handler(tauri::generate_handler![
            commands::wallpaper::set_wallpaper,
            commands::wallpaper::download_wallpaper,
            commands::wallpaper::stop_live_wallpaper,
            commands::wallpaper::set_live_paused,
            commands::wallpaper::get_live_status,
            commands::wallpaper::get_current_wallpaper,
            commands::sources::search,
            commands::favorites::add_favorite,
            commands::favorites::remove_favorite,
            commands::favorites::get_favorites,
            commands::favorites::create_collection,
            commands::favorites::delete_collection,
            commands::favorites::get_collections,
            commands::favorites::add_to_collection,
            commands::favorites::remove_from_collection,
            commands::history::get_history,
            commands::history::remove_history_entry,
            commands::history::clear_history,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::scheduler::start_slideshow,
            commands::scheduler::stop_slideshow,
            commands::scheduler::get_slideshow_status,
            commands::filesystem::add_watched_folder,
            commands::filesystem::remove_watched_folder,
            commands::filesystem::get_watched_folders,
            commands::filesystem::get_local_images,
            commands::filesystem::get_thumbnail,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // With close-to-tray the window simply closes (freeing the UI) and the tray keeps
                // Lumen running — see RunEvent::ExitRequested below.
                if window.label() == "main" && !window.state::<Arc<Database>>().settings().minimize_to_tray {
                    api.prevent_close();
                    quit(window.app_handle());
                }
            }
        })
        .setup(move |app| {
            use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            // One-time move to the persistent defaults (start with Windows, keep playing on battery)
            // for settings saved before they existed. Later changes in Settings are respected.
            if db.get_setting("persistence_defaults").ok().flatten().is_none() {
                let mut s = db.settings();
                s.start_on_boot = true;
                s.pause_on_battery = false;
                let _ = db.save_settings(&s);
                let _ = db.set_setting("persistence_defaults", "1");
            }
            let settings = db.settings();
            // Keep the startup entry pointing at this exe (it moves when Lumen is reinstalled).
            // Debug builds skip this so a dev build never registers itself.
            #[cfg(not(debug_assertions))]
            match wallpaper_engine::set_autostart(settings.start_on_boot) {
                Ok(()) => log::info!("start with Windows: {}", settings.start_on_boot),
                Err(e) => log::warn!("could not set start-with-Windows: {}", e),
            }

            // Local files the UI may display: thumbnails, downloads and watched folders.
            let scope = app.asset_protocol_scope();
            let _ = scope.allow_directory(utils::thumbnails_dir(), false);
            let _ = std::fs::create_dir_all(&settings.download_dir);
            let _ = scope.allow_directory(&settings.download_dir, true);
            for folder in db.get_watched_folders().unwrap_or_default() {
                let _ = scope.allow_directory(&folder.path, folder.recursive);
            }

            let show = MenuItem::with_id(app, "show", "Open Lumen", true, None::<&str>)?;
            let pause_live = MenuItem::with_id(app, "pause_live", "Pause / resume live wallpaper", true, None::<&str>)?;
            let stop_live = MenuItem::with_id(app, "stop_live", "Stop live wallpaper", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit (stops live wallpaper)", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &pause_live, &stop_live, &separator, &quit_item])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().cloned().ok_or("missing app icon")?)
                .tooltip("Lumen")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "pause_live" => wallpaper_engine::toggle_manual_pause(app),
                    "stop_live" => wallpaper_engine::stop_live(app, true),
                    "quit" => quit(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            let args: Vec<String> = std::env::args().collect();
            let cwd = std::env::current_dir().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
            let opened_file = apply_from_args(app.handle(), &args, &cwd);
            if !start_minimized && !opened_file {
                create_main(app.handle())?;
            }

            // Bring back whatever was running last session — unless Lumen was started to open a
            // file, which the old wallpaper would otherwise replace a moment later.
            if !opened_file {
                let handle = app.handle().clone();
                let scheduler = scheduler.clone();
                tauri::async_runtime::spawn(async move {
                    wallpaper_engine::restore_live(&handle);
                    scheduler.restore(handle.clone()).await;
                });
            }
            wallpaper_engine::spawn_pause_monitor(app.handle().clone());

            log::info!("Lumen started");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Lumen")
        .run(|_app, event| {
            // Closing the last window keeps Lumen in the tray; only Quit (which sets an exit code) exits.
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
