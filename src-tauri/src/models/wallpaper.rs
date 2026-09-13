use serde::{Deserialize, Serialize};

/// How a static wallpaper is scaled on the monitor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum FitMode {
    #[default]
    Fill,
    Fit,
    Stretch,
    Center,
    Tile,
    Span,
}

impl FitMode {
    /// Maps to the `wallpaper` crate mode. macOS sets the fit through NSWorkspace instead, so this
    /// is unused there.
    #[allow(dead_code)]
    pub fn to_mode(&self) -> ::wallpaper::Mode {
        match self {
            Self::Fill => ::wallpaper::Mode::Crop,
            Self::Fit => ::wallpaper::Mode::Fit,
            Self::Stretch => ::wallpaper::Mode::Stretch,
            Self::Center => ::wallpaper::Mode::Center,
            Self::Tile => ::wallpaper::Mode::Tile,
            Self::Span => ::wallpaper::Mode::Span,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    #[default]
    Image,
    Video,
    Gif,
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Image => write!(f, "image"),
            Self::Video => write!(f, "video"),
            Self::Gif => write!(f, "gif"),
        }
    }
}

impl MediaType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "mp4" | "webm" | "mkv" | "mov" | "m4v" => Self::Video,
            "gif" => Self::Gif,
            _ => Self::Image,
        }
    }

    pub fn from_path(path: &str) -> Self {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        Self::from_extension(ext)
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "video" => Self::Video,
            "gif" => Self::Gif,
            _ => Self::Image,
        }
    }

    pub fn is_live(&self) -> bool {
        matches!(self, Self::Video | Self::Gif)
    }
}

/// Core wallpaper information used across search results, favorites and history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperInfo {
    pub id: String,
    pub source: String,
    #[serde(default)]
    pub source_id: Option<String>,
    pub url: String,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub local_path: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub colors: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub media_type: MediaType,
}

impl WallpaperInfo {
    pub fn from_local(path: &str) -> Self {
        Self {
            id: format!("local_{}", path),
            source: "local".into(),
            source_id: None,
            url: path.into(),
            thumbnail_url: None,
            local_path: Some(path.into()),
            width: None,
            height: None,
            colors: None,
            tags: None,
            title: std::path::Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string()),
            author: None,
            media_type: MediaType::from_path(path),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Favorite {
    #[serde(flatten)]
    pub wallpaper: WallpaperInfo,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub cover_image: Option<String>,
    pub count: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub source: String,
    pub source_id: Option<String>,
    pub url: String,
    pub thumbnail_url: Option<String>,
    pub local_path: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub title: Option<String>,
    pub media_type: MediaType,
    pub set_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalImage {
    pub path: String,
    pub filename: String,
    pub extension: String,
    pub size_bytes: u64,
    pub media_type: MediaType,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedFolder {
    pub id: String,
    pub path: String,
    pub recursive: bool,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideshowConfig {
    pub source: SlideshowSource,
    pub interval_secs: u64,
    pub shuffle: bool,
    #[serde(default)]
    pub fit_mode: FitMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SlideshowSource {
    Favorites,
    Collection(String),
    Folder(String),
}
