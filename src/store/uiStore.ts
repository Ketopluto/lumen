import { create } from 'zustand';
import type { Wallpaper as WallpaperInfo } from '../api';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface Toast {
  id: string;
  type: ToastType;
  title: string;
  message?: string;
  duration?: number;
}

interface UIState {
  sidebarCollapsed: boolean;
  activePreview: WallpaperInfo | null;
  toasts: Toast[];
  filterPanelOpen: boolean;
  isLoading: boolean;
  globalError: string | null;

  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setActivePreview: (wallpaper: WallpaperInfo | null) => void;
  addToast: (toast: Omit<Toast, 'id'>) => void;
  removeToast: (id: string) => void;
  setFilterPanelOpen: (open: boolean) => void;
  setIsLoading: (loading: boolean) => void;
  setGlobalError: (error: string | null) => void;
}

let toastId = 0;

export const useUIStore = create<UIState>((set) => ({
  sidebarCollapsed: false,
  activePreview: null,
  toasts: [],
  filterPanelOpen: false,
  isLoading: false,
  globalError: null,

  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),
  setActivePreview: (wallpaper) => set({ activePreview: wallpaper }),
  addToast: (toast) => {
    const id = String(++toastId);
    set((s) => ({ toasts: [...s.toasts, { ...toast, id }] }));
    const duration = toast.duration ?? 4000;
    if (duration > 0) {
      setTimeout(() => {
        set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
      }, duration);
    }
  },
  removeToast: (id) =>
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
  setFilterPanelOpen: (open) => set({ filterPanelOpen: open }),
  setIsLoading: (loading) => set({ isLoading: loading }),
  setGlobalError: (error) => set({ globalError: error }),
}));
