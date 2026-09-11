use crate::models::*;
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::sync::Mutex;

/// Thread-safe SQLite wrapper.
pub struct Database {
    conn: Mutex<Connection>,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

const FAVORITE_COLUMNS: &str = "f.id, f.source, f.source_id, f.url, f.thumbnail_url, f.local_path, \
     f.width, f.height, f.colors, f.tags, f.title, f.author, f.media_type, f.created_at";

fn favorite_from_row(row: &Row) -> rusqlite::Result<Favorite> {
    let colors: Option<String> = row.get(8)?;
    let tags: Option<String> = row.get(9)?;
    let media: String = row.get(12)?;
    Ok(Favorite {
        wallpaper: WallpaperInfo {
            id: row.get(0)?,
            source: row.get(1)?,
            source_id: row.get(2)?,
            url: row.get(3)?,
            thumbnail_url: row.get(4)?,
            local_path: row.get(5)?,
            width: row.get(6)?,
            height: row.get(7)?,
            colors: colors.and_then(|s| serde_json::from_str(&s).ok()),
            tags: tags.and_then(|s| serde_json::from_str(&s).ok()),
            title: row.get(10)?,
            author: row.get(11)?,
            media_type: MediaType::parse(&media),
        },
        created_at: row.get(13)?,
    })
}

impl Database {
    pub fn new(db_path: &std::path::Path) -> Result<Self, String> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(err)?;
        }
        let conn = Connection::open(db_path).map_err(err)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").map_err(err)?;
        conn.execute_batch(include_str!("../../migrations/001_initial.sql"))
            .map_err(|e| format!("Migration failed: {}", e))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        // A panic while holding the lock shouldn't brick the database for the rest of the session.
        Ok(self.conn.lock().unwrap_or_else(|p| p.into_inner()))
    }

    // ── Favorites ──────────────────────────────────────────────────────

    /// Adds a favorite, or returns the existing one if the wallpaper is already favorited.
    pub fn add_favorite(&self, info: &WallpaperInfo) -> Result<Favorite, String> {
        let conn = self.conn()?;
        let existing = conn
            .query_row(
                &format!("SELECT {} FROM favorites f WHERE f.id = ?1 OR f.url = ?2", FAVORITE_COLUMNS),
                params![info.id, info.url],
                favorite_from_row,
            )
            .optional()
            .map_err(err)?;
        if let Some(fav) = existing {
            return Ok(fav);
        }

        let colors = info.colors.as_ref().and_then(|c| serde_json::to_string(c).ok());
        let tags = info.tags.as_ref().and_then(|t| serde_json::to_string(t).ok());
        conn.execute(
            "INSERT INTO favorites (id, source, source_id, url, thumbnail_url, local_path, width, height, colors, tags, title, author, media_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![info.id, info.source, info.source_id, info.url, info.thumbnail_url, info.local_path,
                    info.width, info.height, colors, tags, info.title, info.author, info.media_type.to_string()],
        ).map_err(err)?;

        conn.query_row(
            &format!("SELECT {} FROM favorites f WHERE f.id = ?1", FAVORITE_COLUMNS),
            params![info.id],
            favorite_from_row,
        )
        .map_err(err)
    }

    pub fn remove_favorite(&self, id: &str) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM collection_items WHERE favorite_id = ?1", params![id]).map_err(err)?;
        conn.execute("DELETE FROM favorites WHERE id = ?1", params![id]).map_err(err)?;
        Ok(())
    }

    /// Remember where a favorite was downloaded so slideshows don't re-download it.
    pub fn set_favorite_local_path(&self, id: &str, path: &str) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute("UPDATE favorites SET local_path = ?2 WHERE id = ?1", params![id, path]).map_err(err)?;
        Ok(())
    }

    pub fn get_favorites(&self, collection_id: Option<&str>) -> Result<Vec<Favorite>, String> {
        let conn = self.conn()?;
        let rows = match collection_id {
            Some(cid) => {
                let mut stmt = conn
                    .prepare(&format!(
                        "SELECT {} FROM favorites f INNER JOIN collection_items ci ON f.id = ci.favorite_id \
                         WHERE ci.collection_id = ?1 ORDER BY ci.sort_order DESC",
                        FAVORITE_COLUMNS
                    ))
                    .map_err(err)?;
                let rows = stmt.query_map(params![cid], favorite_from_row).map_err(err)?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            None => {
                let mut stmt = conn
                    .prepare(&format!("SELECT {} FROM favorites f ORDER BY f.created_at DESC", FAVORITE_COLUMNS))
                    .map_err(err)?;
                let rows = stmt.query_map([], favorite_from_row).map_err(err)?;
                rows.collect::<Result<Vec<_>, _>>()
            }
        };
        rows.map_err(err)
    }

    // ── Collections ────────────────────────────────────────────────────

    pub fn create_collection(&self, name: &str, description: Option<&str>) -> Result<Collection, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Collection name can't be empty".into());
        }
        let conn = self.conn()?;
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO collections (id, name, description) VALUES (?1, ?2, ?3)",
            params![id, name, description],
        )
        .map_err(err)?;
        let now = chrono::Utc::now().to_rfc3339();
        Ok(Collection {
            id,
            name: name.to_string(),
            description: description.map(str::to_string),
            cover_image: None,
            count: 0,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn delete_collection(&self, id: &str) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM collection_items WHERE collection_id = ?1", params![id]).map_err(err)?;
        conn.execute("DELETE FROM collections WHERE id = ?1", params![id]).map_err(err)?;
        Ok(())
    }

    pub fn get_collections(&self) -> Result<Vec<Collection>, String> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT c.id, c.name, c.description, c.cover_image, c.created_at, c.updated_at, \
                 (SELECT COUNT(*) FROM collection_items WHERE collection_id = c.id) \
                 FROM collections c ORDER BY c.name COLLATE NOCASE",
            )
            .map_err(err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Collection {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    cover_image: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    count: row.get(6)?,
                })
            })
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }

    pub fn add_to_collection(&self, collection_id: &str, favorite_id: &str) -> Result<(), String> {
        let conn = self.conn()?;
        let sort_order: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM collection_items WHERE collection_id = ?1",
                params![collection_id],
                |row| row.get(0),
            )
            .map_err(err)?;
        conn.execute(
            "INSERT OR IGNORE INTO collection_items (id, collection_id, favorite_id, sort_order) VALUES (?1, ?2, ?3, ?4)",
            params![uuid::Uuid::new_v4().to_string(), collection_id, favorite_id, sort_order],
        )
        .map_err(err)?;
        conn.execute(
            "UPDATE collections SET updated_at = datetime('now'), \
             cover_image = COALESCE(cover_image, (SELECT thumbnail_url FROM favorites WHERE id = ?2)) WHERE id = ?1",
            params![collection_id, favorite_id],
        )
        .map_err(err)?;
        Ok(())
    }

    pub fn remove_from_collection(&self, collection_id: &str, favorite_id: &str) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM collection_items WHERE collection_id = ?1 AND favorite_id = ?2",
            params![collection_id, favorite_id],
        )
        .map_err(err)?;
        Ok(())
    }

    // ── History ────────────────────────────────────────────────────────

    pub fn add_history(&self, info: &WallpaperInfo, max_entries: u32) -> Result<(), String> {
        let conn = self.conn()?;
        // Keep one entry per wallpaper: re-setting it just moves it to the top.
        conn.execute("DELETE FROM history WHERE url = ?1", params![info.url]).map_err(err)?;
        conn.execute(
            "INSERT INTO history (id, source, source_id, url, thumbnail_url, local_path, width, height, title, media_type, set_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params![uuid::Uuid::new_v4().to_string(), info.source, info.source_id, info.url, info.thumbnail_url,
                    info.local_path, info.width, info.height, info.title, info.media_type.to_string()],
        )
        .map_err(err)?;
        conn.execute(
            "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY set_at DESC LIMIT ?1)",
            params![max_entries.max(1)],
        )
        .map_err(err)?;
        Ok(())
    }

    pub fn get_history(&self, limit: u32) -> Result<Vec<HistoryEntry>, String> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, source, source_id, url, thumbnail_url, local_path, width, height, title, media_type, set_at \
                 FROM history ORDER BY set_at DESC LIMIT ?1",
            )
            .map_err(err)?;
        let rows = stmt
            .query_map(params![limit], |row| {
                let media: String = row.get(9)?;
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    source_id: row.get(2)?,
                    url: row.get(3)?,
                    thumbnail_url: row.get(4)?,
                    local_path: row.get(5)?,
                    width: row.get(6)?,
                    height: row.get(7)?,
                    title: row.get(8)?,
                    media_type: MediaType::parse(&media),
                    set_at: row.get(10)?,
                })
            })
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }

    pub fn remove_history_entry(&self, id: &str) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM history WHERE id = ?1", params![id]).map_err(err)?;
        Ok(())
    }

    pub fn clear_history(&self) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM history", []).map_err(err)?;
        Ok(())
    }

    // ── Key/value settings ─────────────────────────────────────────────

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn()?;
        conn.query_row("SELECT value FROM settings WHERE key = ?1", params![key], |row| row.get(0))
            .optional()
            .map_err(err)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)", params![key, value])
            .map_err(err)?;
        Ok(())
    }

    pub fn delete_setting(&self, key: &str) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM settings WHERE key = ?1", params![key]).map_err(err)?;
        Ok(())
    }

    /// Never fails: corrupt or missing settings fall back to defaults.
    pub fn settings(&self) -> AppSettings {
        self.get_setting("app_settings")
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), String> {
        let json = serde_json::to_string(settings).map_err(err)?;
        self.set_setting("app_settings", &json)
    }

    // ── Watched folders ────────────────────────────────────────────────

    pub fn add_watched_folder(&self, path: &str, recursive: bool) -> Result<WatchedFolder, String> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO watched_folders (id, path, recursive) VALUES (?1, ?2, ?3)",
            params![uuid::Uuid::new_v4().to_string(), path, recursive as i32],
        )
        .map_err(err)?;
        conn.query_row(
            "SELECT id, path, recursive, added_at FROM watched_folders WHERE path = ?1",
            params![path],
            |row| {
                Ok(WatchedFolder {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    recursive: row.get::<_, i32>(2)? != 0,
                    added_at: row.get(3)?,
                })
            },
        )
        .map_err(err)
    }

    pub fn remove_watched_folder(&self, path: &str) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM watched_folders WHERE path = ?1", params![path]).map_err(err)?;
        Ok(())
    }

    pub fn get_watched_folders(&self) -> Result<Vec<WatchedFolder>, String> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT id, path, recursive, added_at FROM watched_folders ORDER BY added_at")
            .map_err(err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(WatchedFolder {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    recursive: row.get::<_, i32>(2)? != 0,
                    added_at: row.get(3)?,
                })
            })
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> Database {
        let path = std::env::temp_dir().join(format!("lumen-test-{}.db", uuid::Uuid::new_v4()));
        Database::new(&path).unwrap()
    }

    fn wp(id: &str) -> WallpaperInfo {
        WallpaperInfo {
            id: id.into(),
            source: "wallhaven".into(),
            source_id: Some(id.into()),
            url: format!("https://example.com/{}.jpg", id),
            thumbnail_url: Some(format!("https://example.com/{}_t.jpg", id)),
            local_path: None,
            width: Some(1920),
            height: Some(1080),
            colors: Some(vec!["#000000".into()]),
            tags: None,
            title: Some("t".into()),
            author: None,
            media_type: MediaType::Image,
        }
    }

    #[test]
    fn favorites_are_deduplicated_and_removable() {
        let db = temp_db();
        db.add_favorite(&wp("a")).unwrap();
        db.add_favorite(&wp("a")).unwrap();
        db.add_favorite(&wp("b")).unwrap();
        assert_eq!(db.get_favorites(None).unwrap().len(), 2);
        db.remove_favorite("a").unwrap();
        let favs = db.get_favorites(None).unwrap();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].wallpaper.id, "b");
        assert_eq!(favs[0].wallpaper.colors.as_deref(), Some(&["#000000".to_string()][..]));
    }

    #[test]
    fn collections_track_members() {
        let db = temp_db();
        db.add_favorite(&wp("a")).unwrap();
        let c = db.create_collection("Space", None).unwrap();
        db.add_to_collection(&c.id, "a").unwrap();
        db.add_to_collection(&c.id, "a").unwrap();
        let cols = db.get_collections().unwrap();
        assert_eq!(cols[0].count, 1);
        assert!(cols[0].cover_image.is_some());
        assert_eq!(db.get_favorites(Some(&c.id)).unwrap().len(), 1);
        db.remove_favorite("a").unwrap();
        assert_eq!(db.get_collections().unwrap()[0].count, 0);
        assert!(db.create_collection("  ", None).is_err());
    }

    #[test]
    fn history_dedupes_and_prunes() {
        let db = temp_db();
        for id in ["a", "b", "a", "c", "d"] {
            db.add_history(&wp(id), 3).unwrap();
        }
        let h = db.get_history(10).unwrap();
        assert_eq!(h.len(), 3);
        assert_eq!(h[0].url, wp("d").url);
        assert!(h.iter().filter(|e| e.url == wp("a").url).count() <= 1);
    }

    #[test]
    fn settings_round_trip_and_tolerate_old_json() {
        let db = temp_db();
        assert!(db.settings().minimize_to_tray);
        db.set_setting("app_settings", r#"{"theme":"light","unknown_field":1}"#).unwrap();
        let s = db.settings();
        assert_eq!(s.theme, ThemePreference::Light);
        assert!(s.pause_on_fullscreen);
        db.set_setting("app_settings", "not json").unwrap();
        assert_eq!(db.settings().theme, ThemePreference::Dark);
    }

    #[test]
    fn watched_folders_are_unique() {
        let db = temp_db();
        let a = db.add_watched_folder("C:/x", true).unwrap();
        let b = db.add_watched_folder("C:/x", true).unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(db.get_watched_folders().unwrap().len(), 1);
    }
}
