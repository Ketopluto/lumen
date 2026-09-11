use crate::models::*;
use crate::utils::{is_wallpaper_extension, thumbnails_dir};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::sync::Semaphore;

/// Upper bound on files returned for one folder scan (keeps the UI responsive).
const MAX_FILES: usize = 5000;

/// Scan a folder (optionally recursively) for images and videos, newest first.
pub fn scan_folder(root: &Path, recursive: bool) -> Vec<LocalImage> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else { continue };
            let path = entry.path();
            if file_type.is_dir() {
                if recursive {
                    stack.push(path);
                }
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if !is_wallpaper_extension(&ext) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            out.push(LocalImage {
                path: path.to_string_lossy().to_string(),
                filename: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                media_type: MediaType::from_extension(&ext),
                extension: ext,
                size_bytes: meta.len(),
                modified_at: meta
                    .modified()
                    .ok()
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
            });
            if out.len() >= MAX_FILES {
                break;
            }
        }
        if out.len() >= MAX_FILES {
            break;
        }
    }
    out.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    out
}

fn thumb_semaphore() -> &'static Semaphore {
    // Decoding big photos is memory-hungry: only a few at a time.
    static SEM: OnceLock<Semaphore> = OnceLock::new();
    SEM.get_or_init(|| Semaphore::new(3))
}

fn thumb_path(src: &Path) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    if let Ok(meta) = std::fs::metadata(src) {
        meta.len().hash(&mut hasher);
        if let Ok(m) = meta.modified() {
            m.hash(&mut hasher);
        }
    }
    thumbnails_dir().join(format!("{:016x}.jpg", hasher.finish()))
}

/// Returns a cached 480px JPEG thumbnail for a local image, generating it if needed.
pub async fn thumbnail(src: PathBuf) -> Result<PathBuf, String> {
    let dest = thumb_path(&src);
    if dest.exists() {
        return Ok(dest);
    }
    let _permit = thumb_semaphore().acquire().await.map_err(|e| e.to_string())?;
    if dest.exists() {
        return Ok(dest);
    }
    tokio::task::spawn_blocking(move || {
        let img = image::open(&src).map_err(|e| format!("Cannot read image: {}", e))?;
        let thumb = img.thumbnail(480, 480).to_rgb8();
        std::fs::create_dir_all(thumbnails_dir()).map_err(|e| e.to_string())?;
        let tmp = dest.with_extension("tmp");
        thumb
            .save_with_format(&tmp, image::ImageFormat::Jpeg)
            .map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
        Ok(dest)
    })
    .await
    .map_err(|e| e.to_string())?
}
