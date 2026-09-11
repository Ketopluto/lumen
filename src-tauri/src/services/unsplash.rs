use crate::models::*;
use crate::services::api_client::ApiClient;
use crate::utils::url_encode;
use serde::Deserialize;

const UNSPLASH_API: &str = "https://api.unsplash.com";

/// Unsplash (requires a free access key).
pub struct UnsplashService;

#[derive(Deserialize)]
struct UnsplashSearchResponse {
    results: Vec<UnsplashPhoto>,
    total: Option<u64>,
    total_pages: Option<u32>,
}

#[derive(Deserialize)]
struct UnsplashPhoto {
    id: String,
    width: Option<u32>,
    height: Option<u32>,
    color: Option<String>,
    description: Option<String>,
    alt_description: Option<String>,
    urls: UnsplashUrls,
    user: Option<UnsplashUser>,
}

#[derive(Deserialize)]
struct UnsplashUrls {
    raw: Option<String>,
    full: Option<String>,
    small: Option<String>,
    regular: Option<String>,
}

#[derive(Deserialize)]
struct UnsplashUser {
    name: Option<String>,
}

impl UnsplashService {
    pub async fn search(params: &SearchParams, access_key: &str) -> Result<SearchResult, String> {
        let page = params.page();
        let url = format!(
            "{}/search/photos?query={}&page={}&per_page=30&orientation=landscape",
            UNSPLASH_API,
            url_encode(params.query().unwrap_or("wallpaper")),
            page
        );
        let auth = format!("Client-ID {}", access_key);
        let resp: UnsplashSearchResponse = ApiClient::get_json(&url, &[("Authorization", auth.as_str())]).await?;

        let total_pages = resp.total_pages.unwrap_or(1);
        let wallpapers = resp
            .results
            .into_iter()
            .map(|p| WallpaperInfo {
                id: format!("unsplash_{}", p.id),
                source: "unsplash".into(),
                source_id: Some(p.id),
                // `raw` + params gives a 4K JPEG without downloading the (often 20MB+) original.
                url: p
                    .urls
                    .raw
                    .map(|r| format!("{}&w=3840&fm=jpg&q=90", r))
                    .or(p.urls.full)
                    .unwrap_or_default(),
                thumbnail_url: p.urls.small.or(p.urls.regular),
                local_path: None,
                width: p.width,
                height: p.height,
                colors: p.color.map(|c| vec![c]),
                tags: None,
                title: p.description.or(p.alt_description),
                author: p.user.and_then(|u| u.name),
                media_type: MediaType::Image,
            })
            .collect();

        Ok(SearchResult { wallpapers, total: resp.total, page, has_more: page < total_pages, seed: None })
    }
}
