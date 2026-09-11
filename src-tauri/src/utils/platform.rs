use std::path::PathBuf;

/// Platform data directory for Lumen (database lives here).
/// `LUMEN_DATA_DIR` overrides it, e.g. to test without touching real favorites.
pub fn app_data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("LUMEN_DATA_DIR") {
        return PathBuf::from(dir);
    }
    dirs::data_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".lumen"))
        .join("Lumen")
}

/// The app used to be called Vividwall: move its data over once so favorites and history carry on.
pub fn migrate_legacy_data() {
    if std::env::var_os("LUMEN_DATA_DIR").is_some() {
        return;
    }
    let Some(base) = dirs::data_dir() else { return };
    let (old, new) = (base.join("vividwall"), base.join("Lumen"));
    if old.is_dir() && !new.exists() {
        let _ = std::fs::rename(&old, &new);
    }
    // Rename the database together with its WAL/SHM files so no committed data is lost.
    if new.join("vividwall.db").is_file() && !new.join("lumen.db").exists() {
        for suffix in ["", "-wal", "-shm"] {
            let from = new.join(format!("vividwall.db{}", suffix));
            if from.exists() {
                let _ = std::fs::rename(from, new.join(format!("lumen.db{}", suffix)));
            }
        }
    }
}

/// Platform cache directory (thumbnails).
pub fn app_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".cache"))
        .join("Lumen")
}

pub fn thumbnails_dir() -> PathBuf {
    app_cache_dir().join("thumbnails")
}

/// Default folder that downloaded wallpapers are saved to.
pub fn default_download_dir() -> PathBuf {
    dirs::picture_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Pictures"))
        .join("Lumen")
}

pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "gif", "tif", "tiff"];
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "webm", "m4v", "mov"];

pub fn is_wallpaper_extension(ext: &str) -> bool {
    let ext = ext.to_lowercase();
    IMAGE_EXTENSIONS.contains(&ext.as_str()) || VIDEO_EXTENSIONS.contains(&ext.as_str())
}

/// Percent-encode a string for use in a URL query or path segment.
pub fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}
