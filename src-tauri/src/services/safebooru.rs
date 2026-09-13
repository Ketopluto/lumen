//! Safebooru: animated anime wallpapers, no API key.
//!
//! This is the only large, keyless source of moving anime art I could find that actually answers:
//! Danbooru sits behind a Cloudflare challenge, Gelbooru wants credentials, and Reddit no longer
//! serves its JSON to apps. Safebooru is the all-ages board, so everything here is `rating:safe`,
//! and it is checked again on the way out.
//!
//! Nearly all of it is GIF, which is the happy accident that makes it work everywhere: no codec,
//! no GStreamer plugin, no WebM on WKWebView problem. The handful of real videos are filtered
//! against what this system can decode.

use crate::models::*;
use crate::services::api_client::ApiClient;
use crate::services::media::MediaSupport;
use crate::utils::url_encode;
use serde::Deserialize;

const SAFEBOORU_API: &str = "https://safebooru.org/index.php?page=dapi&s=post&q=index&json=1";
const PER_PAGE: u32 = 40;

/// Below this, a post is a reaction GIF or a sticker rather than a wallpaper. It costs most of the
/// catalogue (17k animated posts, ~2.6k of them this wide) and is worth it.
const MIN_WIDTH: u32 = 1000;

pub struct SafebooruService;

#[derive(Deserialize)]
struct SafebooruPost {
    id: u64,
    width: Option<u32>,
    height: Option<u32>,
    rating: Option<String>,
    tags: Option<String>,
    file_url: Option<String>,
    preview_url: Option<String>,
    sample_url: Option<String>,
}

/// What the file actually is, from its extension — a post tagged `animated` can still be a PNG.
fn media_type(url: &str) -> Option<MediaType> {
    let extension = url.rsplit('.').next()?.split('?').next()?.to_lowercase();
    match extension.as_str() {
        "gif" => Some(MediaType::Gif),
        "webm" | "mp4" => Some(MediaType::Video),
        _ => None,
    }
}

fn playable(url: &str, support: MediaSupport) -> bool {
    match url.rsplit('.').next().map(str::to_lowercase).as_deref() {
        Some("webm") => support.webm,
        Some("mp4") => support.mp4,
        _ => true,
    }
}

impl SafebooruService {
    pub async fn search(params: &SearchParams, support: MediaSupport) -> Result<SearchResult, String> {
        let page = params.page();
        // Booru tags are underscored: "genshin impact" -> "genshin_impact".
        let mut tags = vec!["animated".to_string(), format!("width:>={}", MIN_WIDTH)];
        if let Some(query) = params.query() {
            tags.push(query.to_lowercase().split_whitespace().collect::<Vec<_>>().join("_"));
        }
        let url = format!(
            "{}&tags={}&limit={}&pid={}",
            SAFEBOORU_API,
            url_encode(&tags.join(" ")),
            PER_PAGE,
            page - 1 // Safebooru counts pages from zero.
        );

        // A search with no matches answers with an empty body, which no deserializer accepts.
        let body = ApiClient::get_text(&url, &[]).await?;
        let posts: Vec<SafebooruPost> = if body.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&body).map_err(|e| format!("Unexpected response from Safebooru: {}", e))?
        };
        let full_page = posts.len() as u32 >= PER_PAGE;

        let wallpapers = posts
            .into_iter()
            // Safebooru renamed "safe" to "general" when it adopted Danbooru's rating names; both
            // mean all-ages. Anything else ("sensitive", "questionable") never reaches the grid.
            .filter(|p| matches!(p.rating.as_deref(), Some("general") | Some("safe")))
            .filter(|p| p.width.unwrap_or(0) >= MIN_WIDTH)
            .filter_map(|p| {
                let url = p.file_url?;
                let media_type = media_type(&url)?;
                if !playable(&url, support) {
                    return None;
                }
                Some(WallpaperInfo {
                    id: format!("safebooru_{}", p.id),
                    source: "safebooru".into(),
                    source_id: Some(p.id.to_string()),
                    url,
                    // A still JPEG thumbnail: the grid stays cheap even with 40 animations on it.
                    thumbnail_url: p.preview_url.or(p.sample_url),
                    local_path: None,
                    width: p.width,
                    height: p.height,
                    colors: None,
                    tags: p
                        .tags
                        .map(|t| t.split_whitespace().take(8).map(|s| s.replace('_', " ")).collect()),
                    title: None,
                    author: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_moving_files_are_offered() {
        assert!(matches!(
            media_type("https://safebooru.org/images/1/a.gif"),
            Some(MediaType::Gif)
        ));
        assert!(matches!(
            media_type("https://safebooru.org/images/1/a.mp4"),
            Some(MediaType::Video)
        ));
        // Tagged "animated", but a still image as far as the wallpaper engine is concerned.
        assert!(media_type("https://safebooru.org/images/1/a.png").is_none());
    }

    #[test]
    fn a_system_without_webm_is_not_offered_webm() {
        let no_webm = MediaSupport { webm: false, mp4: true };
        assert!(!playable("https://safebooru.org/images/1/a.webm", no_webm));
        assert!(playable("https://safebooru.org/images/1/a.mp4", no_webm));
        // GIF needs no codec at all, which is why this source works everywhere.
        assert!(playable(
            "https://safebooru.org/images/1/a.gif",
            MediaSupport {
                webm: false,
                mp4: false
            }
        ));
    }
}

/// Hit the real boards: `cargo test -- --ignored`. Not in the default run because CI should not
/// fail when a third-party site has a bad afternoon.
#[cfg(test)]
mod live_api {
    use super::*;
    use crate::services::konachan::KonachanService;

    fn params(query: Option<&str>) -> SearchParams {
        SearchParams {
            source: "anime_live".into(),
            query: query.map(String::from),
            page: 1,
            sorting: None,
            top_range: None,
            categories: None,
            resolution: None,
            seed: None,
        }
    }

    #[test]
    #[ignore]
    fn safebooru_returns_moving_wallpapers() {
        let support = MediaSupport { webm: true, mp4: true };
        let result = tauri::async_runtime::block_on(SafebooruService::search(&params(None), support)).unwrap();
        println!(
            "safebooru: {} wallpapers, more: {}",
            result.wallpapers.len(),
            result.has_more
        );
        assert!(!result.wallpapers.is_empty());
        for w in &result.wallpapers {
            assert!(w.width.unwrap_or(0) >= MIN_WIDTH, "{} is too small", w.url);
            assert!(!matches!(w.media_type, MediaType::Image), "{} does not move", w.url);
        }
        let first = &result.wallpapers[0];
        println!(
            "  e.g. {:?} {}x{} {}",
            first.media_type,
            first.width.unwrap(),
            first.height.unwrap(),
            first.url
        );
    }

    #[test]
    #[ignore]
    fn safebooru_search_with_no_matches_is_empty_not_an_error() {
        let support = MediaSupport { webm: true, mp4: true };
        let result =
            tauri::async_runtime::block_on(SafebooruService::search(&params(Some("zzqq no such tag xx")), support))
                .unwrap();
        assert!(result.wallpapers.is_empty());
        assert!(!result.has_more);
    }

    #[test]
    #[ignore]
    fn konachan_animated_returns_moving_wallpapers() {
        let result = tauri::async_runtime::block_on(KonachanService::search_animated(&params(None))).unwrap();
        println!("konachan animated: {} wallpapers", result.wallpapers.len());
        assert!(!result.wallpapers.is_empty());
        for w in &result.wallpapers {
            assert!(!matches!(w.media_type, MediaType::Image), "{} does not move", w.url);
            assert!(!w.url.contains("/jpeg/"), "{} is Konachan's still frame", w.url);
        }
        println!("  e.g. {}", result.wallpapers[0].url);
    }
}
