use crate::models::*;
use crate::services::database::Database;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_history(limit: Option<u32>, db: State<'_, Arc<Database>>) -> Result<Vec<HistoryEntry>, String> {
    db.get_history(limit.unwrap_or(200))
}

#[tauri::command]
pub fn remove_history_entry(id: String, db: State<'_, Arc<Database>>) -> Result<(), String> {
    db.remove_history_entry(&id)
}

#[tauri::command]
pub fn clear_history(db: State<'_, Arc<Database>>) -> Result<(), String> {
    db.clear_history()
}
