import { create } from 'zustand';
import { api, errorText, isLive, type Wallpaper } from '../api';
import { useUIStore, type Toast } from './uiStore';

const toast = (t: Omit<Toast, 'id'>) => useUIStore.getState().addToast(t);

interface AppState {
  /** wallpaper id -> favorite row id */
  favIds: Record<string, string>;
  /** wallpaper url -> favorite row id (catches the same image found via another id) */
  favUrls: Record<string, string>;
  /** Bumped whenever favorites change so pages can refresh. */
  favVersion: number;
  /** wallpaper id -> download progress 0..1, or null when unknown. Present = busy. */
  busy: Record<string, number | null>;

  loadFavorites: () => Promise<void>;
  favoriteIdOf: (wp: Wallpaper) => string | undefined;
  toggleFavorite: (wp: Wallpaper) => Promise<void>;
  apply: (wp: Wallpaper) => Promise<void>;
  download: (wp: Wallpaper) => Promise<void>;
  setProgress: (id: string, received: number, total: number | null) => void;
}

export const useAppStore = create<AppState>((set, get) => {
  const run = async (wp: Wallpaper, task: () => Promise<void>) => {
    if (wp.id in get().busy) return;
    set((s) => ({ busy: { ...s.busy, [wp.id]: null } }));
    try {
      await task();
    } finally {
      set((s) => {
        const busy = { ...s.busy };
        delete busy[wp.id];
        return { busy };
      });
    }
  };

  return {
    favIds: {},
    favUrls: {},
    favVersion: 0,
    busy: {},

    loadFavorites: async () => {
      try {
        const favs = await api.getFavorites();
        const favIds: Record<string, string> = {};
        const favUrls: Record<string, string> = {};
        for (const f of favs) {
          favIds[f.id] = f.id;
          favUrls[f.url] = f.id;
        }
        set((s) => ({ favIds, favUrls, favVersion: s.favVersion + 1 }));
      } catch (e) {
        console.error('Failed to load favorites', e);
      }
    },

    favoriteIdOf: (wp) => get().favIds[wp.id] ?? get().favUrls[wp.url],

    toggleFavorite: async (wp) => {
      const existing = get().favoriteIdOf(wp);
      try {
        if (existing) {
          await api.removeFavorite(existing);
          toast({ type: 'info', title: 'Removed from favorites', duration: 2000 });
        } else {
          await api.addFavorite(wp);
          toast({ type: 'success', title: 'Added to favorites', duration: 2000 });
        }
      } catch (e) {
        toast({ type: 'error', title: 'Favorites error', message: errorText(e) });
      }
      await get().loadFavorites();
    },

    apply: (wp) =>
      run(wp, async () => {
        try {
          await api.setWallpaper(wp);
          toast({ type: 'success', title: isLive(wp) ? 'Live wallpaper set' : 'Wallpaper set', duration: 2500 });
        } catch (e) {
          toast({ type: 'error', title: "Couldn't set wallpaper", message: errorText(e), duration: 6000 });
        }
      }),

    download: (wp) =>
      run(wp, async () => {
        try {
          const path = await api.downloadWallpaper(wp);
          toast({ type: 'success', title: 'Saved', message: path, duration: 4000 });
        } catch (e) {
          toast({ type: 'error', title: 'Download failed', message: errorText(e), duration: 6000 });
        }
      }),

    setProgress: (id, received, total) =>
      set((s) => (id in s.busy ? { busy: { ...s.busy, [id]: total ? Math.min(1, received / total) : null } } : s)),
  };
});
