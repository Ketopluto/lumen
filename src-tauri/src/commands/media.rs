use crate::services::database::Database;
use crate::services::media::{self, MediaSupport};
use std::sync::Arc;
use tauri::State;

/// The UI reports what the webview can decode, once per start.
#[tauri::command]
pub fn set_media_support(support: MediaSupport, db: State<'_, Arc<Database>>) {
    media::save(&db, support);
}
