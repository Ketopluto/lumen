use crate::models::*;
use crate::services::api_client::ApiClient;
use crate::utils::url_encode;
use serde::Deserialize;

const PEXELS_API: &str = "https://api.pexels.com";

/// Pexels photos and videos (requires a free API key).
pub struct PexelsService;

#[derive(Deserialize)]
struct PhotosResponse {
    #[serde(default)]
    photos: Vec<PexelsPhoto>,
    total_results: Option<u64>,
    next_page: Option<String>,
}

#[derive(Deserialize)]
struct PexelsPhoto {
    id: u64,
    width: Option<u32>,
    height: Option<u32>,
    photographer: Option<String>,
    avg_color: Option<String>,
    alt: Option<String>,
    src: PexelsSrc,
}

#[derive(Deserialize)]
struct PexelsSrc {
    original: Option<String>,
    large2x: Option<String>,
    medium: Option<String>,
    large: Option<String>,
}

#[derive(Deserialize)]
struct VideosResponse {
    #[serde(default)]
    videos: Vec<PexelsVideo>,
    total_results: Option<u64>,
    next_page: Option<String>,
}

#[derive(Deserialize)]
struct PexelsVideo {
    id: u64,
    width: Option<u32>,
    height: Option<u32>,
    image: Option<String>,
    user: Option<PexelsUser>,
    #[serde(default)]
    video_files: Vec<PexelsVideoFile>,
}

#[derive(Deserialize)]
struct PexelsUser {
    name: Option<String>,
}

#[derive(Deserialize)]
struct PexelsVideoFile {
    file_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    link: String,
}

impl PexelsService {
    pub async fn search_photos(params: &SearchParams, api_key: &str) -> Result<SearchResult, String> {
        let page = params.page();
        let url = match params.query() {
            Some(q) => format!(
                "{}/v1/search?query={}&page={}&per_page=40&orientation=landscape",
                PEXELS_API,
                url_encode(q),
                page
            ),
            None => format!("{}/v1/curated?page={}&per_page=40", PEXELS_API, page),
        };
        let resp: PhotosResponse = ApiClient::get_json(&url, &[("Authorization", api_key)]).await?;

        let wallpapers = resp
            .photos
            .into_iter()
            .map(|p| WallpaperInfo {
                id: format!("pexels_{}", p.id),
                source: "pexels".into(),
                source_id: Some(p.id.to_string()),
                url: p.src.original.or(p.src.large2x).unwrap_or_default(),
                thumbnail_url: p.src.large.or(p.src.medium),
                local_path: None,
                width: p.width,
                height: p.height,
                colors: p.avg_color.map(|c| vec![c]),
                tags: None,
                title: p.alt.filter(|a| !a.is_empty()),
                author: p.photographer,
                media_type: MediaType::Image,
            })
            .collect();

        Ok(SearchResult {
            wallpapers,
            total: resp.total_results,
            page,
            has_more: resp.next_page.is_some(),
            seed: None,
        })
    }

    pub async fn search_videos(params: &SearchParams, api_key: &str) -> Result<SearchResult, String> {
        let page = params.page();
        let url = match params.query() {
            Some(q) => format!(
                "{}/videos/search?query={}&page={}&per_page=30&orientation=landscape",
                PEXELS_API,
                url_encode(q),
                page
            ),
            None => format!("{}/videos/popular?page={}&per_page=30&min_width=1920", PEXELS_API, page),
        };
        let resp: VideosResponse = ApiClient::get_json(&url, &[("Authorization", api_key)]).await?;

        let wallpapers = resp
            .videos
            .into_iter()
            .filter_map(|v| {
                // Best mp4 rendition up to 1080p keeps downloads and GPU decode light.
                let file = v
                    .video_files
                    .iter()
                    .filter(|f| f.file_type.as_deref() == Some("video/mp4"))
                    .filter(|f| f.height.unwrap_or(0).min(f.width.unwrap_or(0)) <= 1080)
                    .max_by_key(|f| f.height.unwrap_or(0))
                    .or_else(|| v.video_files.first())?;
                Some(WallpaperInfo {
                    id: format!("pexelsvideo_{}", v.id),
                    source: "pexels".into(),
                    source_id: Some(v.id.to_string()),
                    url: file.link.clone(),
                    thumbnail_url: v.image,
                    local_path: None,
                    width: v.width,
                    height: v.height,
                    colors: None,
                    tags: None,
                    title: None,
                    author: v.user.and_then(|u| u.name),
                    media_type: MediaType::Video,
                })
            })
            .collect();

        Ok(SearchResult {
            wallpapers,
            total: resp.total_results,
            page,
            has_more: resp.next_page.is_some(),
            seed: None,
        })
    }
}
