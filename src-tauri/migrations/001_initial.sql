-- Lumen initial database schema

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS favorites (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    source_id TEXT,
    url TEXT NOT NULL,
    thumbnail_url TEXT,
    local_path TEXT,
    width INTEGER,
    height INTEGER,
    colors TEXT,
    tags TEXT,
    title TEXT,
    author TEXT,
    media_type TEXT NOT NULL DEFAULT 'image',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    cover_image TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS collection_items (
    id TEXT PRIMARY KEY,
    collection_id TEXT NOT NULL,
    favorite_id TEXT NOT NULL,
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    sort_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY (favorite_id) REFERENCES favorites(id) ON DELETE CASCADE,
    UNIQUE(collection_id, favorite_id)
);

CREATE TABLE IF NOT EXISTS history (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    source_id TEXT,
    url TEXT NOT NULL,
    thumbnail_url TEXT,
    local_path TEXT,
    width INTEGER,
    height INTEGER,
    title TEXT,
    media_type TEXT NOT NULL DEFAULT 'image',
    set_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS watched_folders (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    recursive INTEGER NOT NULL DEFAULT 1,
    added_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS cached_images (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    source_id TEXT NOT NULL,
    url TEXT NOT NULL,
    local_path TEXT NOT NULL,
    thumbnail_path TEXT,
    file_size INTEGER,
    downloaded_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(source, source_id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_favorites_source ON favorites(source);
CREATE INDEX IF NOT EXISTS idx_history_set_at ON history(set_at);
CREATE INDEX IF NOT EXISTS idx_collection_items_collection ON collection_items(collection_id);
CREATE INDEX IF NOT EXISTS idx_cached_images_source ON cached_images(source, source_id);
