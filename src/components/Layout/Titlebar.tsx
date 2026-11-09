import React from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { capabilities } from '../../platform';
import './Titlebar.css';

export const Titlebar: React.FC = () => {
  // Resolved on use, not on render: outside Tauri (vite dev in a plain browser) this throws, and
  // a titlebar that cannot find its window should not take the whole app down with it.
  const window = () => {
    try {
      return getCurrentWindow();
    } catch {
      return null;
    }
  };

  return (
    <div className="titlebar" data-tauri-drag-region id="titlebar">
      <div className="titlebar__brand" data-tauri-drag-region>
        <div className="titlebar__logo">
          {/* A screen with a sun over a horizon: the wallpaper, drawn in the one colour this
              window has. currentColor, so it is paper on the dark theme and ink on the light. */}
          <svg width="19" height="19" viewBox="0 0 20 20" aria-hidden="true">
            <defs>
              <clipPath id="logo-screen">
                <rect x="2.9" y="3.9" width="14.2" height="12.2" rx="2.1" />
              </clipPath>
            </defs>
            <rect
              x="2.9"
              y="3.9"
              width="14.2"
              height="12.2"
              rx="2.1"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.3"
            />
            <g clipPath="url(#logo-screen)" fill="currentColor">
              <circle cx="13.2" cy="7.6" r="1.7" />
              <path d="M2.9 13.1c2.1-1.9 4-1.6 5.6-0.6s3.3 1.7 5 0.7 2.5-1 3.6-0.2V16.1H2.9Z" />
            </g>
          </svg>
        </div>
        <span className="titlebar__title" data-tauri-drag-region>Lumen</span>
      </div>
      {/* macOS draws its own traffic lights over the window. */}
      {capabilities().custom_titlebar && (
      <div className="titlebar__controls">
        <button
          className="titlebar__btn titlebar__btn--minimize"
          onClick={() => window()?.minimize()}
          id="btn-minimize"
          aria-label="Minimize"
        >
          <svg width="12" height="12" viewBox="0 0 12 12"><rect y="5" width="12" height="1.5" rx="0.75" fill="currentColor"/></svg>
        </button>
        <button
          className="titlebar__btn titlebar__btn--maximize"
          onClick={() => window()?.toggleMaximize()}
          id="btn-maximize"
          aria-label="Maximize"
        >
          <svg width="12" height="12" viewBox="0 0 12 12"><rect x="1" y="1" width="10" height="10" rx="1.5" stroke="currentColor" strokeWidth="1.5" fill="none"/></svg>
        </button>
        <button
          className="titlebar__btn titlebar__btn--close"
          onClick={() => window()?.close()}
          id="btn-close"
          aria-label="Close"
        >
          <svg width="12" height="12" viewBox="0 0 12 12"><path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/></svg>
        </button>
      </div>
      )}
    </div>
  );
};
