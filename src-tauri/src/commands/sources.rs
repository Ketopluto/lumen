use crate::models::*;
use crate::services::bing::BingService;
use crate::services::commons::CommonsService;
use crate::services::database::Database;
use crate::services::konachan::KonachanService;
use crate::services::nasa::NasaService;
use crate::services::pexels::PexelsService;
use crate::services::unsplash::UnsplashService;
use crate::services::wallhaven::WallhavenService;
use std::sync::Arc;
use tauri::State;

/// Unified search across every online source. API keys come from saved settings.
#[tauri::command]
pub async fn search(params: SearchParams, db: State<'_, Arc<Database>>) -> Result<SearchResult, String> {
    let settings = db.settings();
    let need_key = |name: &str| format!("{} needs a free API key — add it in Settings", name);

    match params.source.as_str() {
        "wallhaven" => WallhavenService::search(&params, AppSettings::key(&settings.wallhaven_api_key)).await,
        "anime" => {
            let anime = SearchParams { categories: Some("010".into()), ..params.clone() };
            WallhavenService::search(&anime, AppSettings::key(&settings.wallhaven_api_key)).await
        }
        "konachan" => KonachanService::search(&params).await,
        "commons" => CommonsService::search_images(&params).await,
        "live" => CommonsService::search_videos(&params).await,
        "nasa" => NasaService::search(&params).await,
        "bing" => BingService::get_daily(&params).await,
        "unsplash" => match AppSettings::key(&settings.unsplash_access_key) {
            Some(key) => UnsplashService::search(&params, key).await,
            None => Err(need_key("Unsplash")),
        },
        "pexels" => match AppSettings::key(&settings.pexels_api_key) {
            Some(key) => PexelsService::search_photos(&params, key).await,
            None => Err(need_key("Pexels")),
        },
        "pexels_video" => match AppSettings::key(&settings.pexels_api_key) {
            Some(key) => PexelsService::search_videos(&params, key).await,
            None => Err(need_key("Pexels")),
        },
        other => Err(format!("Unknown source: {}", other)),
    }
}
