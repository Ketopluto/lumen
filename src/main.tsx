import React, { Suspense, lazy } from 'react';
import ReactDOM from 'react-dom/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { applyPlatformAttributes, loadCapabilities } from './platform';
import './index.css';

// The desktop-wallpaper windows only need the tiny player, not the whole app. There is one per
// display on platforms where a window cannot span displays: live_wallpaper, live_wallpaper_1, …
const isLiveWindow = (() => {
  try {
    return getCurrentWindow().label.startsWith('live_wallpaper');
  } catch {
    return false;
  }
})();

// Set data-os before the first paint so platform CSS never flashes the wrong style.
applyPlatformAttributes();
void loadCapabilities();

const Root = lazy(() => (isLiveWindow ? import('./pages/LiveWallpaper/LiveWallpaper') : import('./App')));

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <Suspense fallback={null}>
      <Root />
    </Suspense>
  </React.StrictMode>,
);
