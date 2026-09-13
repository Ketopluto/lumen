import { invoke, convertFileSrc } from '@tauri-apps/api/core';

export type MediaType = 'image' | 'video' | 'gif';
export type FitMode = 'fill' | 'fit' | 'stretch' | 'center' | 'tile' | 'span';
export type Theme = 'dark' | 'light' | 'system';

export interface Wallpaper {
  id: string;
  source: string;
  source_id?: string | null;
  url: string;
  thumbnail_url?: string | null;
  local_path?: string | null;
  width?: number | null;
  height?: number | null;
  colors?: string[] | null;
  tags?: string[] | null;
  title?: string | null;
  author?: string | null;
  media_type: MediaType;
}

export interface Favorite extends Wallpaper {
  created_at: string;
}

export interface Collection {
  id: string;
  name: string;
  description: string | null;
  cover_image: string | null;
  count: number;
  created_at: string;
  updated_at: string;
}

export interface HistoryEntry {
  id: string;
  source: string;
  source_id: string | null;
  url: string;
  thumbnail_url: string | null;
  local_path: string | null;
  width: number | null;
  height: number | null;
  title: string | null;
  media_type: MediaType;
  set_at: string;
}

export interface LocalImage {
  path: string;
  filename: string;
  extension: string;
  size_bytes: number;
  media_type: MediaType;
  modified_at: string | null;
}

export interface WatchedFolder {
  id: string;
  path: string;
  recursive: boolean;
  added_at: string;
}

export interface Settings {
  fit_mode: FitMode;
  start_on_boot: boolean;
  minimize_to_tray: boolean;
  download_dir: string;
  max_history: number;
  wallhaven_api_key: string | null;
  unsplash_access_key: string | null;
  pexels_api_key: string | null;
  theme: Theme;
  live_wallpaper_volume: number;
  pause_on_battery: boolean;
  pause_on_fullscreen: boolean;
}

export interface SearchParams {
  source: string;
  query?: string;
  page: number;
  sorting?: string;
  top_range?: string;
  categories?: string;
  resolution?: string;
  seed?: string | null;
}

export interface SearchResult {
  wallpapers: Wallpaper[];
  total: number | null;
  page: number;
  has_more: boolean;
  seed: string | null;
}

export type SlideshowSource =
  | { type: 'Favorites' }
  | { type: 'Collection'; value: string }
  | { type: 'Folder'; value: string };

export interface SlideshowConfig {
  source: SlideshowSource;
  interval_secs: number;
  shuffle: boolean;
  fit_mode: FitMode;
}

export interface LiveStatus {
  path: string | null;
  /** Effective state: paused by the user or automatically (fullscreen app / battery). */
  paused: boolean;
  manual_paused: boolean;
  volume: number;
}

export const api = {
  search: (params: SearchParams) => invoke<SearchResult>('search', { params }),

  setWallpaper: (wallpaper: Wallpaper, fitMode?: FitMode) => invoke<string>('set_wallpaper', { wallpaper, fitMode }),
  downloadWallpaper: (wallpaper: Wallpaper) => invoke<string>('download_wallpaper', { wallpaper }),
  stopLive: () => invoke<void>('stop_live_wallpaper'),
  setLivePaused: (paused: boolean) => invoke<void>('set_live_paused', { paused }),
  liveStatus: () => invoke<LiveStatus>('get_live_status'),

  getFavorites: (collectionId?: string | null) => invoke<Favorite[]>('get_favorites', { collectionId: collectionId ?? null }),
  addFavorite: (wallpaper: Wallpaper) => invoke<Favorite>('add_favorite', { wallpaper }),
  removeFavorite: (id: string) => invoke<void>('remove_favorite', { id }),
  getCollections: () => invoke<Collection[]>('get_collections'),
  createCollection: (name: string) => invoke<Collection>('create_collection', { name, description: null }),
  deleteCollection: (id: string) => invoke<void>('delete_collection', { id }),
  addToCollection: (collectionId: string, favoriteId: string) =>
    invoke<void>('add_to_collection', { collectionId, favoriteId }),
  removeFromCollection: (collectionId: string, favoriteId: string) =>
    invoke<void>('remove_from_collection', { collectionId, favoriteId }),

  getHistory: (limit = 200) => invoke<HistoryEntry[]>('get_history', { limit }),
  removeHistoryEntry: (id: string) => invoke<void>('remove_history_entry', { id }),
  clearHistory: () => invoke<void>('clear_history'),

  getSettings: () => invoke<Settings>('get_settings'),
  updateSettings: (settings: Settings) => invoke<Settings>('update_settings', { settings }),

  startSlideshow: (config: SlideshowConfig) => invoke<void>('start_slideshow', { config }),
  stopSlideshow: () => invoke<void>('stop_slideshow'),
  slideshowStatus: () => invoke<SlideshowConfig | null>('get_slideshow_status'),

  getWatchedFolders: () => invoke<WatchedFolder[]>('get_watched_folders'),
  addWatchedFolder: (path: string) => invoke<WatchedFolder>('add_watched_folder', { path, recursive: true }),
  removeWatchedFolder: (path: string) => invoke<void>('remove_watched_folder', { path }),
  getLocalImages: (folderPath: string) => invoke<LocalImage[]>('get_local_images', { folderPath, recursive: true }),
  getThumbnail: (path: string) => invoke<string>('get_thumbnail', { path }),

  setMediaSupport: (support: { webm: boolean; mp4: boolean }) => invoke<void>('set_media_support', { support }),
};

export const isRemote = (s?: string | null): s is string => !!s && /^https?:\/\//i.test(s);
export const fileSrc = (path: string) => convertFileSrc(path);
export const isLive = (w: { media_type: MediaType }) => w.media_type === 'video' || w.media_type === 'gif';

export function errorText(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return String(e);
}

export function fromLocal(img: LocalImage): Wallpaper {
  return {
    id: `local_${img.path}`,
    source: 'local',
    url: img.path,
    local_path: img.path,
    title: img.filename,
    media_type: img.media_type,
  };
}

export function fromHistory(h: HistoryEntry): Wallpaper {
  return {
    id: h.source_id ? `${h.source}_${h.source_id}` : `local_${h.local_path ?? h.url}`,
    source: h.source,
    source_id: h.source_id,
    url: h.url,
    thumbnail_url: h.thumbnail_url,
    local_path: h.local_path,
    width: h.width,
    height: h.height,
    title: h.title,
    media_type: h.media_type,
  };
}

/** Human-friendly title: local files lose their extension and underscores. */
export function displayTitle(w: Pick<Wallpaper, 'title' | 'source'>): string | null {
  if (!w.title) return null;
  if (w.source !== 'local') return w.title;
  return w.title.replace(/\.[a-z0-9]{2,5}$/i, '').replace(/[_-]+/g, ' ').trim() || w.title;
}

export function applyTheme(theme: Theme) {
  const resolved =
    theme === 'system' ? (window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark') : theme;
  document.documentElement.setAttribute('data-theme', resolved);
}

export const SOURCE_LABELS: Record<string, string> = {
  wallhaven: 'Wallhaven',
  konachan: 'Konachan',
  commons: 'Wikimedia Commons',
  nasa: 'NASA',
  bing: 'Bing',
  unsplash: 'Unsplash',
  pexels: 'Pexels',
  local: 'Local file',
};
