use crate::models::*;
use crate::services::api_client::ApiClient;
use serde::Deserialize;

const BING_API: &str = "https://www.bing.com/HPImageArchive.aspx";

/// Bing's daily wallpapers (Bing only exposes the last ~16 days).
pub struct BingService;

#[derive(Deserialize)]
struct BingResponse {
    #[serde(default)]
    images: Vec<BingImage>,
}

#[derive(Deserialize)]
struct BingImage {
    urlbase: String,
    title: Option<String>,
    copyright: Option<String>,
    startdate: Option<String>,
    hsh: Option<String>,
}

impl BingService {
    pub async fn get_daily(params: &SearchParams) -> Result<SearchResult, String> {
        let page = params.page();
        if page > 2 {
            return Ok(SearchResult { wallpapers: vec![], total: Some(16), page, has_more: false, seed: None });
        }
        let url = format!("{}?format=js&idx={}&n=8&mkt=en-US", BING_API, (page - 1) * 8);
        let resp: BingResponse = ApiClient::get_json(&url, &[]).await?;

        let wallpapers = resp
            .images
            .into_iter()
            .map(|img| {
                let base = format!("https://www.bing.com{}", img.urlbase);
                let id = img.hsh.or(img.startdate.clone()).unwrap_or_else(|| img.urlbase.clone());
                WallpaperInfo {
                    id: format!("bing_{}", id),
                    source: "bing".into(),
                    source_id: img.startdate,
                    url: format!("{}_UHD.jpg", base),
                    thumbnail_url: Some(format!("{}_800x480.jpg", base)),
                    local_path: None,
                    width: Some(3840),
                    height: Some(2160),
                    colors: None,
                    tags: None,
                    title: img.title,
                    author: img.copyright,
                    media_type: MediaType::Image,
                }
            })
            .collect();

        Ok(SearchResult { wallpapers, total: Some(16), page, has_more: page < 2, seed: None })
    }
}
