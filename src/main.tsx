import React, { Suspense, lazy } from 'react';
import ReactDOM from 'react-dom/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import './index.css';

// The desktop-wallpaper window only needs the tiny player, not the whole app.
const isLiveWindow = (() => {
  try {
    return getCurrentWindow().label === 'live_wallpaper';
  } catch {
    return false;
  }
})();

const Root = lazy(() => (isLiveWindow ? import('./pages/LiveWallpaper/LiveWallpaper') : import('./App')));

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <Suspense fallback={null}>
      <Root />
    </Suspense>
  </React.StrictMode>,
);
