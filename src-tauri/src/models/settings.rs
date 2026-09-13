use super::FitMode;
use serde::{Deserialize, Serialize};

/// Application settings persisted in the database.
/// `#[serde(default)]` keeps older saved settings loadable when fields are added.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub fit_mode: FitMode,
    pub start_on_boot: bool,
    pub minimize_to_tray: bool,
    pub download_dir: String,
    pub max_history: u32,
    pub wallhaven_api_key: Option<String>,
    pub unsplash_access_key: Option<String>,
    pub pexels_api_key: Option<String>,
    pub theme: ThemePreference,
    /// Live wallpaper volume (0-100). 0 = muted.
    pub live_wallpaper_volume: u32,
    pub pause_on_battery: bool,
    pub pause_on_fullscreen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    Light,
    Dark,
    System,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            fit_mode: FitMode::Fill,
            // A live wallpaper should survive reboots, so Lumen starts with Windows by default.
            start_on_boot: true,
            minimize_to_tray: true,
            download_dir: crate::utils::default_download_dir().to_string_lossy().to_string(),
            max_history: 500,
            wallhaven_api_key: None,
            unsplash_access_key: None,
            pexels_api_key: None,
            theme: ThemePreference::Dark,
            live_wallpaper_volume: 0,
            pause_on_battery: false,
            pause_on_fullscreen: true,
        }
    }
}

impl AppSettings {
    pub fn key(value: &Option<String>) -> Option<&str> {
        value.as_deref().map(str::trim).filter(|k| !k.is_empty())
    }
}
