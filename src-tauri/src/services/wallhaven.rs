use crate::models::*;
use crate::services::api_client::ApiClient;
use crate::utils::url_encode;
use serde::Deserialize;

const WALLHAVEN_API: &str = "https://wallhaven.cc/api/v1";

/// Wallhaven: ~600k+ SFW wallpapers, no API key required.
pub struct WallhavenService;

#[derive(Deserialize)]
struct WallhavenResponse {
    data: Vec<WallhavenWallpaper>,
    meta: Option<WallhavenMeta>,
}

#[derive(Deserialize)]
struct WallhavenWallpaper {
    id: String,
    path: String,
    #[serde(default)]
    thumbs: WallhavenThumbs,
    dimension_x: Option<u32>,
    dimension_y: Option<u32>,
    #[serde(default)]
    colors: Vec<String>,
    category: Option<String>,
    file_type: Option<String>,
}

#[derive(Deserialize, Default)]
struct WallhavenThumbs {
    large: Option<String>,
    original: Option<String>,
    small: Option<String>,
}

#[derive(Deserialize)]
struct WallhavenMeta {
    total: Option<u64>,
    current_page: Option<u32>,
    last_page: Option<u32>,
    seed: Option<String>,
}

impl WallhavenService {
    pub async fn search(params: &SearchParams, api_key: Option<&str>) -> Result<SearchResult, String> {
        let page = params.page();
        let sorting = params.sorting.as_deref().unwrap_or(if params.query().is_some() {
            "relevance"
        } else {
            "toplist"
        });
        let categories = params
            .categories
            .as_deref()
            .filter(|c| c.len() == 3 && c.chars().all(|ch| ch == '0' || ch == '1') && *c != "000")
            .unwrap_or("111");

        let mut url = format!(
            // Landscape only: portrait art doesn't fit a desktop.
            "{}/search?purity=100&ratios=landscape&categories={}&sorting={}&page={}",
            WALLHAVEN_API, categories, sorting, page
        );
        if let Some(q) = params.query() {
            url.push_str(&format!("&q={}", url_encode(q)));
        }
        if sorting == "toplist" {
            url.push_str(&format!("&topRange={}", params.top_range.as_deref().unwrap_or("1M")));
        }
        if let Some(res) = params.resolution.as_deref().filter(|r| r.contains('x')) {
            url.push_str(&format!("&atleast={}", url_encode(res)));
        }
        if sorting == "random" {
            if let Some(seed) = params.seed.as_deref() {
                url.push_str(&format!("&seed={}", url_encode(seed)));
            }
        }

        let headers: Vec<(&str, &str)> = api_key.map(|k| vec![("X-API-Key", k)]).unwrap_or_default();
        let resp: WallhavenResponse = ApiClient::get_json(&url, &headers).await?;

        let wallpapers = resp
            .data
            .into_iter()
            .map(|w| WallpaperInfo {
                id: format!("wallhaven_{}", w.id),
                source: "wallhaven".into(),
                source_id: Some(w.id),
                url: w.path,
                thumbnail_url: w.thumbs.large.or(w.thumbs.small).or(w.thumbs.original),
                local_path: None,
                width: w.dimension_x,
                height: w.dimension_y,
                colors: if w.colors.is_empty() { None } else { Some(w.colors) },
                tags: w.category.map(|c| vec![c]),
                title: None,
                author: None,
                media_type: if w.file_type.as_deref() == Some("image/gif") {
                    MediaType::Gif
                } else {
                    MediaType::Image
                },
            })
            .collect();

        let meta = resp.meta;
        let current = meta.as_ref().and_then(|m| m.current_page).unwrap_or(page);
        let last = meta.as_ref().and_then(|m| m.last_page).unwrap_or(current);
        Ok(SearchResult {
            wallpapers,
            total: meta.as_ref().and_then(|m| m.total),
            page: current,
            has_more: current < last,
            seed: meta.and_then(|m| m.seed),
        })
    }
}
