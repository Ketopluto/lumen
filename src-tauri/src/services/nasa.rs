use crate::models::*;
use crate::services::api_client::ApiClient;
use crate::utils::url_encode;
use serde::Deserialize;

const NASA_API: &str = "https://images-api.nasa.gov/search";

/// NASA Image Library: 140k+ space images, no API key required.
pub struct NasaService;

#[derive(Deserialize)]
struct NasaResponse {
    collection: NasaCollection,
}

#[derive(Deserialize)]
struct NasaCollection {
    #[serde(default)]
    items: Vec<NasaItem>,
    #[serde(default)]
    links: Vec<NasaLink>,
    metadata: Option<NasaMeta>,
}

#[derive(Deserialize)]
struct NasaMeta {
    total_hits: Option<u64>,
}

#[derive(Deserialize)]
struct NasaItem {
    #[serde(default)]
    data: Vec<NasaData>,
    #[serde(default)]
    links: Vec<NasaLink>,
}

#[derive(Deserialize)]
struct NasaData {
    nasa_id: String,
    title: Option<String>,
    secondary_creator: Option<String>,
}

#[derive(Deserialize)]
struct NasaLink {
    href: String,
    rel: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

impl NasaService {
    pub async fn search(params: &SearchParams) -> Result<SearchResult, String> {
        let page = params.page();
        let q = params.query().unwrap_or("galaxy");
        let url = format!(
            "{}?q={}&media_type=image&page={}&page_size=40",
            NASA_API,
            url_encode(q),
            page
        );
        let resp: NasaResponse = ApiClient::get_json(&url, &[]).await?;

        let wallpapers = resp
            .collection
            .items
            .into_iter()
            .filter_map(|item| {
                let data = item.data.into_iter().next()?;
                let preview = item
                    .links
                    .iter()
                    .find(|l| l.href.contains("~medium"))
                    .or(item.links.first())?;
                // Skip portrait images where the preview reveals the aspect ratio.
                if let (Some(w), Some(h)) = (preview.width, preview.height) {
                    if w < h {
                        return None;
                    }
                }
                let id = url_encode(&data.nasa_id);
                Some(WallpaperInfo {
                    id: format!("nasa_{}", data.nasa_id),
                    source: "nasa".into(),
                    source_id: Some(data.nasa_id.clone()),
                    url: format!("https://images-assets.nasa.gov/image/{0}/{0}~orig.jpg", id),
                    thumbnail_url: Some(preview.href.clone()),
                    local_path: None,
                    width: None,
                    height: None,
                    colors: None,
                    tags: None,
                    title: data.title,
                    author: data.secondary_creator,
                    media_type: MediaType::Image,
                })
            })
            .collect();

        Ok(SearchResult {
            wallpapers,
            total: resp.collection.metadata.and_then(|m| m.total_hits),
            page,
            has_more: resp.collection.links.iter().any(|l| l.rel.as_deref() == Some("next")),
            seed: None,
        })
    }
}
