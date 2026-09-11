use serde::{Deserialize, Serialize};

/// Parameters for a unified search across all online sources.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchParams {
    /// wallhaven | commons | live | nasa | bing | unsplash | pexels | pexels_video
    pub source: String,
    pub query: Option<String>,
    pub page: u32,
    /// Wallhaven: toplist | hot | date_added | random | views | favorites | relevance
    pub sorting: Option<String>,
    /// Wallhaven toplist range: 1d | 3d | 1w | 1M | 3M | 6M | 1y
    pub top_range: Option<String>,
    /// Wallhaven category bitmask: general/anime/people, e.g. "111"
    pub categories: Option<String>,
    /// Minimum resolution, e.g. "1920x1080"
    pub resolution: Option<String>,
    /// Wallhaven random seed so random pages don't repeat
    pub seed: Option<String>,
}

impl SearchParams {
    pub fn page(&self) -> u32 {
        self.page.max(1)
    }

    pub fn query(&self) -> Option<&str> {
        self.query.as_deref().map(str::trim).filter(|q| !q.is_empty())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub wallpapers: Vec<super::WallpaperInfo>,
    pub total: Option<u64>,
    pub page: u32,
    pub has_more: bool,
    pub seed: Option<String>,
}
