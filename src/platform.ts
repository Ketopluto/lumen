import { invoke } from '@tauri-apps/api/core';
import type { FitMode } from './api';

export type OsName = 'windows' | 'macos' | 'linux' | 'unknown';

/** Mirrors `Capabilities` in src-tauri/src/services/desktop/types.rs. */
export interface Capabilities {
  os: OsName;
  session: string | null;
  desktop: string | null;
  live_wallpaper: boolean;
  live_unsupported_reason: string | null;
  live_all_monitors: boolean;
  autostart: boolean;
  pause_on_fullscreen: boolean;
  pause_on_battery: boolean;
  fit_modes: FitMode[];
  static_wallpaper: boolean;
  custom_titlebar: boolean;
  beta: boolean;
}

/** Only reached by `vite dev` in a plain browser tab, where the backend injected nothing. */
const FALLBACK: Capabilities = {
  os: 'unknown',
  session: null,
  desktop: null,
  live_wallpaper: false,
  live_unsupported_reason: 'Lumen is not running on a desktop right now.',
  live_all_monitors: false,
  autostart: false,
  pause_on_fullscreen: false,
  pause_on_battery: false,
  fit_modes: ['fill', 'fit', 'stretch', 'center'],
  static_wallpaper: false,
  custom_titlebar: true,
  beta: false,
};

declare global {
  interface Window {
    __LUMEN__?: { capabilities: Capabilities };
  }
}

// The backend injects this before the first paint, so the very first render is already correct.
let caps: Capabilities = window.__LUMEN__?.capabilities ?? FALLBACK;

export function capabilities(): Capabilities {
  return caps;
}

/** Fallback path for the browser preview: ask the backend instead of reading the injection. */
export async function loadCapabilities(): Promise<Capabilities> {
  if (window.__LUMEN__?.capabilities) return caps;
  try {
    caps = await invoke<Capabilities>('get_capabilities');
    applyPlatformAttributes();
  } catch {
    // Running outside Tauri; keep the fallback.
  }
  return caps;
}

/** Lets CSS branch per platform: `:root[data-os="macos"] { … }`. */
export function applyPlatformAttributes() {
  const root = document.documentElement;
  root.dataset.os = caps.os;
  if (caps.session) root.dataset.session = caps.session;
}

const OS_NAMES: Record<OsName, string> = {
  windows: 'Windows',
  macos: 'macOS',
  linux: 'Linux',
  unknown: 'your system',
};

export const osName = () => OS_NAMES[caps.os];

/** Each platform calls this something different. */
export const startupLabel = () =>
  caps.os === 'macos' ? 'Open at login' : caps.os === 'windows' ? 'Start with Windows' : 'Start on login';

/** Where a background app lives: the Windows tray, the macOS menu bar. */
export const trayName = () => (caps.os === 'macos' ? 'menu bar' : 'tray');

/**
 * What this webview can actually decode. WKWebView often has no WebM, and WebKitGTK depends on
 * which GStreamer plugins are installed, so sources are filtered by this rather than by platform.
 */
export const mediaSupport = (() => {
  try {
    const probe = document.createElement('video');
    const can = (type: string) => probe.canPlayType(type) !== '';
    return {
      webm: can('video/webm; codecs="vp9"') || can('video/webm; codecs="vp8"') || can('video/webm'),
      mp4: can('video/mp4; codecs="avc1.42E01E"') || can('video/mp4'),
    };
  } catch {
    // No DOM (server-side render or a test): assume it plays and let playback report the truth.
    return { webm: true, mp4: true };
  }
})();
