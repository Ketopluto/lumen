use crate::models::*;
use crate::services::api_client::ApiClient;
use crate::utils::url_encode;
use serde::Deserialize;

const KONACHAN_API: &str = "https://konachan.net/post.json";
const PER_PAGE: u32 = 40;

/// Konachan.net: anime wallpaper board (the safe-only mirror). No API key required.
pub struct KonachanService;

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

fn absolute(url: String) -> String {
    if url.starts_with("//") {
        format!("https:{}", url)
    } else {
        url
    }
}

impl KonachanService {
    pub async fn search(params: &SearchParams) -> Result<SearchResult, String> {
        let page = params.page();
        // Konachan searches by tags: "solo leveling" -> "solo_leveling".
        let mut tags = vec!["rating:safe".to_string()];
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
            .filter(|p| matches!((p.width, p.height), (Some(w), Some(h)) if w >= 1280 && w >= h))
            .filter_map(|p| {
                let url = p.jpeg_url.or(p.file_url).map(absolute)?;
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
                    tags: p.tags.map(|t| t.split_whitespace().take(8).map(|s| s.replace('_', " ")).collect()),
                    title: None,
                    author: p.author,
                    media_type: MediaType::Image,
                })
            })
            .collect();

        Ok(SearchResult { wallpapers, total: None, page, has_more: full_page, seed: None })
    }
}
