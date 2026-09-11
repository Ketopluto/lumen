use crate::models::*;
use crate::services::api_client::ApiClient;
use crate::utils::url_encode;
use serde::Deserialize;

const COMMONS_API: &str = "https://commons.wikimedia.org/w/api.php";
const PER_PAGE: u32 = 30;

/// Wikimedia Commons: featured / quality photos and freely-licensed videos (used for live wallpapers).
/// No API key required.
pub struct CommonsService;

#[derive(Deserialize)]
struct CommonsResponse {
    #[serde(rename = "continue")]
    cont: Option<serde_json::Value>,
    query: Option<CommonsQuery>,
}

#[derive(Deserialize)]
struct CommonsQuery {
    #[serde(default)]
    pages: Vec<CommonsPage>,
}

#[derive(Deserialize)]
struct CommonsPage {
    pageid: u64,
    title: String,
    #[serde(default)]
    index: u32,
    #[serde(default)]
    imageinfo: Vec<CommonsInfo>,
    #[serde(default)]
    videoinfo: Vec<CommonsInfo>,
}

#[derive(Deserialize)]
struct CommonsInfo {
    url: Option<String>,
    thumburl: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    mime: Option<String>,
    #[serde(default)]
    derivatives: Vec<CommonsDerivative>,
}

#[derive(Deserialize)]
struct CommonsDerivative {
    src: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    width: u32,
}

fn clean_title(title: &str) -> String {
    let t = title.trim_start_matches("File:");
    match t.rfind('.') {
        Some(i) => t[..i].replace('_', " "),
        None => t.replace('_', " "),
    }
}

/// Strip Commons tracking parameters from media URLs.
fn clean_url(url: &str) -> String {
    url.split('?').next().unwrap_or(url).to_string()
}

impl CommonsService {
    pub async fn search_images(params: &SearchParams) -> Result<SearchResult, String> {
        let mut search = String::from(
            "filetype:bitmap filew:>1919 incategory:Featured_pictures_on_Wikimedia_Commons|Quality_images",
        );
        if let Some(q) = params.query() {
            search = format!("{} {}", q, search);
        }
        Self::run(params, &search, "imageinfo", "iiprop=url|size|mime&iiurlwidth=640").await
    }

    pub async fn search_videos(params: &SearchParams) -> Result<SearchResult, String> {
        let q = params.query().unwrap_or("timelapse");
        let search = format!("{} filetype:video filew:>1279", q);
        Self::run(params, &search, "videoinfo", "viprop=url|size|mime|derivatives&viurlwidth=640").await
    }

    async fn run(params: &SearchParams, search: &str, prop: &str, prop_args: &str) -> Result<SearchResult, String> {
        let page = params.page();
        let sort = if params.query().is_some() { "relevance" } else { "create_timestamp_desc" };
        let url = format!(
            "{}?action=query&format=json&formatversion=2&generator=search&gsrnamespace=6&gsrlimit={}&gsroffset={}&gsrsort={}&gsrsearch={}&prop={}&{}",
            COMMONS_API,
            PER_PAGE,
            (page - 1) * PER_PAGE,
            sort,
            url_encode(search),
            prop,
            prop_args
        );

        let resp: CommonsResponse = ApiClient::get_json(&url, &[]).await?;
        let mut pages = resp.query.map(|q| q.pages).unwrap_or_default();
        pages.sort_by_key(|p| p.index);

        let wallpapers = pages
            .into_iter()
            .filter_map(|p| {
                let is_video = !p.videoinfo.is_empty();
                let info = p.imageinfo.into_iter().chain(p.videoinfo).next()?;
                let (w, h) = (info.width.unwrap_or(0), info.height.unwrap_or(0));
                // Wallpapers only: skip portrait media.
                if w > 0 && h > 0 && w < h {
                    return None;
                }
                let url = if is_video {
                    pick_video(&info)?
                } else {
                    let mime = info.mime.as_deref().unwrap_or("");
                    if mime == "image/tiff" {
                        // Desktop wallpaper APIs dislike TIFF; use a large JPEG rendition instead.
                        info.thumburl.as_deref().map(|t| t.replacen("/640px-", "/3840px-", 1))?
                    } else {
                        info.url.clone()?
                    }
                };
                Some(WallpaperInfo {
                    id: format!("commons_{}", p.pageid),
                    source: "commons".into(),
                    source_id: Some(p.pageid.to_string()),
                    url: clean_url(&url),
                    thumbnail_url: info.thumburl.as_deref().map(clean_url),
                    local_path: None,
                    width: info.width,
                    height: info.height,
                    colors: None,
                    tags: None,
                    title: Some(clean_title(&p.title)),
                    author: None,
                    media_type: if is_video { MediaType::Video } else { MediaType::Image },
                })
            })
            .collect();

        Ok(SearchResult {
            wallpapers,
            total: None,
            page,
            has_more: resp.cont.is_some(),
            seed: None,
        })
    }
}

/// Pick the best WebView-playable rendition up to 1080p (originals can be huge 4K files or Theora).
fn pick_video(info: &CommonsInfo) -> Option<String> {
    let playable = |d: &&CommonsDerivative| {
        d.kind.starts_with("video/webm") && (d.kind.contains("vp9") || d.kind.contains("vp8"))
    };
    let best = info
        .derivatives
        .iter()
        .filter(playable)
        .filter(|d| d.height.min(d.width) <= 1080)
        .max_by_key(|d| d.height);
    best.map(|d| d.src.clone()).or_else(|| {
        let mime = info.mime.as_deref().unwrap_or("");
        if mime == "video/webm" || mime == "video/mp4" {
            info.url.clone()
        } else {
            None
        }
    })
}
