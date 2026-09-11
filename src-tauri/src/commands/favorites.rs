use crate::models::*;
use crate::services::database::Database;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn add_favorite(wallpaper: WallpaperInfo, db: State<'_, Arc<Database>>) -> Result<Favorite, String> {
    db.add_favorite(&wallpaper)
}

#[tauri::command]
pub fn remove_favorite(id: String, db: State<'_, Arc<Database>>) -> Result<(), String> {
    db.remove_favorite(&id)
}

#[tauri::command]
pub fn get_favorites(collection_id: Option<String>, db: State<'_, Arc<Database>>) -> Result<Vec<Favorite>, String> {
    db.get_favorites(collection_id.as_deref())
}

#[tauri::command]
pub fn create_collection(
    name: String,
    description: Option<String>,
    db: State<'_, Arc<Database>>,
) -> Result<Collection, String> {
    db.create_collection(&name, description.as_deref())
}

#[tauri::command]
pub fn delete_collection(id: String, db: State<'_, Arc<Database>>) -> Result<(), String> {
    db.delete_collection(&id)
}

#[tauri::command]
pub fn get_collections(db: State<'_, Arc<Database>>) -> Result<Vec<Collection>, String> {
    db.get_collections()
}

#[tauri::command]
pub fn add_to_collection(collection_id: String, favorite_id: String, db: State<'_, Arc<Database>>) -> Result<(), String> {
    db.add_to_collection(&collection_id, &favorite_id)
}

#[tauri::command]
pub fn remove_from_collection(
    collection_id: String,
    favorite_id: String,
    db: State<'_, Arc<Database>>,
) -> Result<(), String> {
    db.remove_from_collection(&collection_id, &favorite_id)
}
