use crate::models::*;
use crate::services::api_client::ApiClient;
use crate::utils::url_encode;
use serde::Deserialize;

const KONACHAN_API: &str = "https://konachan.net/post.json";
const PER_PAGE: u32 = 40;

/// Konachan.net: anime wallpaper board (the safe-only mirror). No API key required.
pub struct KonachanService;

/// Animated posts are rarer than stills, so they are allowed to be a little smaller.
const MIN_WIDTH: u32 = 1000;

#[derive(Deserialize)]
struct KonachanPost {
    id: u64,
    width: Option<u32>,
    height: Option<u32>,
    rating: Option<String>,
    tags: Option<String>,
    file_url: Option<String>,
    jpeg_url: Option<String>,
    sample_url: Option<String>,
    preview_url: Option<String>,
    author: Option<String>,
}

/// Only the moving formats: a post can carry the `animated` tag and still be a JPEG.
fn animated_media_type(url: &str) -> Option<MediaType> {
    match url.rsplit('.').next().map(str::to_lowercase).as_deref() {
        Some("gif") => Some(MediaType::Gif),
        Some("webm") | Some("mp4") => Some(MediaType::Video),
        _ => None,
    }
}

fn absolute(url: String) -> String {
    if url.starts_with("//") {
        format!("https:{}", url)
    } else {
        url
    }
}

impl KonachanService {
    pub async fn search(params: &SearchParams) -> Result<SearchResult, String> {
        Self::search_board(params, false).await
    }

    /// The same board filtered to its animated posts: a small collection, but every one of them
    /// is a wallpaper rather than a reaction GIF, which is rare for moving anime art.
    pub async fn search_animated(params: &SearchParams) -> Result<SearchResult, String> {
        Self::search_board(params, true).await
    }

    async fn search_board(params: &SearchParams, animated: bool) -> Result<SearchResult, String> {
        let page = params.page();
        // Konachan searches by tags: "solo leveling" -> "solo_leveling".
        let mut tags = vec!["rating:safe".to_string()];
        if animated {
            tags.push("animated".into());
        }
        match params.query() {
            Some(q) => tags.push(q.to_lowercase().split_whitespace().collect::<Vec<_>>().join("_")),
            None => tags.push("order:score".into()),
        }
        let url = format!(
            "{}?tags={}&limit={}&page={}",
            KONACHAN_API,
            url_encode(&tags.join(" ")),
            PER_PAGE,
            page
        );
        let posts: Vec<KonachanPost> = ApiClient::get_json(&url, &[]).await?;
        let full_page = posts.len() as u32 >= PER_PAGE;

        let wallpapers = posts
            .into_iter()
            .filter(|p| p.rating.as_deref() == Some("s"))
            .filter(|p| {
                let floor = if animated { MIN_WIDTH } else { 1280 };
                matches!((p.width, p.height), (Some(w), Some(h)) if w >= floor && w >= h)
            })
            .filter_map(|p| {
                // Konachan's jpeg_url is a still frame, so an animated post has to use the
                // original file or the wallpaper would not move.
                let url = if animated {
                    absolute(p.file_url?)
                } else {
                    p.jpeg_url.or(p.file_url).map(absolute)?
                };
                let media_type = if animated {
                    animated_media_type(&url)?
                } else {
                    MediaType::Image
                };
                Some(WallpaperInfo {
                    id: format!("konachan_{}", p.id),
                    source: "konachan".into(),
                    source_id: Some(p.id.to_string()),
                    url,
                    thumbnail_url: p.sample_url.or(p.preview_url).map(absolute),
                    local_path: None,
                    width: p.width,
                    height: p.height,
                    colors: None,
                    tags: p
                        .tags
                        .map(|t| t.split_whitespace().take(8).map(|s| s.replace('_', " ")).collect()),
                    title: None,
                    author: p.author,
                    media_type,
                })
            })
            .collect();

        Ok(SearchResult {
            wallpapers,
            total: None,
            page,
            has_more: full_page,
            seed: None,
        })
    }
}
