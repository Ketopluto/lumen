//! What this system can actually play.
//!
//! The webview is the only honest source of truth — WKWebView often has no WebM, and WebKitGTK
//! depends on which GStreamer plugins are installed — so the UI probes `canPlayType` and reports
//! it here. It is remembered in the database because Lumen can start straight into the tray, with
//! no UI to ask.

use crate::services::database::Database;
use serde::{Deserialize, Serialize};

const KEY: &str = "media_support";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct MediaSupport {
    pub webm: bool,
    pub mp4: bool,
}

impl Default for MediaSupport {
    /// Assume everything plays until the UI says otherwise: a wrong "no" is worse than a wrong
    /// "yes", which merely ends in a visible playback error.
    fn default() -> Self {
        Self { webm: true, mp4: true }
    }
}

pub fn get(db: &Database) -> MediaSupport {
    db.get_setting(KEY)
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

pub fn save(db: &Database, support: MediaSupport) {
    if let Ok(json) = serde_json::to_string(&support) {
        let _ = db.set_setting(KEY, &json);
    }
}

/// Refuses a file this system cannot decode, naming the way out. Without this the wallpaper would
/// simply be a black rectangle.
pub fn check_playable(db: &Database, path: &str) -> Result<(), String> {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let support = get(db);
    match extension.as_str() {
        "webm" if !support.webm => Err(webm_advice()),
        "mp4" | "m4v" | "mov" if !support.mp4 => Err(mp4_advice()),
        _ => Ok(()),
    }
}

pub fn webm_advice() -> String {
    if cfg!(target_os = "linux") {
        "This system cannot play WebM video. Install the GStreamer plugins (gstreamer1.0-plugins-good) \
         and restart Lumen."
            .into()
    } else {
        "This system cannot play WebM video. Pexels videos are MP4 and work here.".into()
    }
}

pub fn mp4_advice() -> String {
    if cfg!(target_os = "linux") {
        "This system cannot play MP4 video. Install the GStreamer libav plugin (gstreamer1.0-libav) \
         and restart Lumen."
            .into()
    } else {
        "This system cannot play MP4 video.".into()
    }
}
