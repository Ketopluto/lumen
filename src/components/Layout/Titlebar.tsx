import React from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { capabilities } from '../../platform';
import './Titlebar.css';

export const Titlebar: React.FC = () => {
  const appWindow = getCurrentWindow();

  return (
    <div className="titlebar" data-tauri-drag-region id="titlebar">
      <div className="titlebar__brand" data-tauri-drag-region>
        <div className="titlebar__logo">
          <svg width="18" height="18" viewBox="64 64 896 896">
            <defs>
              <linearGradient id="logo-grad" x1="0" y1="0" x2="1" y2="1">
                <stop offset="0" stopColor="#5b5ff0" />
                <stop offset="1" stopColor="#a24cf2" />
              </linearGradient>
              <clipPath id="logo-clip">
                <rect x="64" y="64" width="896" height="896" rx="224" />
              </clipPath>
            </defs>
            <rect x="64" y="64" width="896" height="896" rx="224" fill="url(#logo-grad)" />
            <g clipPath="url(#logo-clip)">
              <path d="M64 690 C 250 560, 430 565, 580 640 S 850 715, 960 610 V 960 H 64 Z" fill="#fff" fillOpacity="0.38" />
              <path d="M64 800 C 250 690, 430 695, 600 770 S 860 835, 960 745 V 960 H 64 Z" fill="#fff" />
            </g>
            <circle cx="680" cy="370" r="108" fill="#fff" />
          </svg>
        </div>
        <span className="titlebar__title" data-tauri-drag-region>Lumen</span>
      </div>
      {/* macOS draws its own traffic lights over the window. */}
      {capabilities().custom_titlebar && (
      <div className="titlebar__controls">
        <button
          className="titlebar__btn titlebar__btn--minimize"
          onClick={() => appWindow.minimize()}
          id="btn-minimize"
          aria-label="Minimize"
        >
          <svg width="12" height="12" viewBox="0 0 12 12"><rect y="5" width="12" height="1.5" rx="0.75" fill="currentColor"/></svg>
        </button>
        <button
          className="titlebar__btn titlebar__btn--maximize"
          onClick={() => appWindow.toggleMaximize()}
          id="btn-maximize"
          aria-label="Maximize"
        >
          <svg width="12" height="12" viewBox="0 0 12 12"><rect x="1" y="1" width="10" height="10" rx="1.5" stroke="currentColor" strokeWidth="1.5" fill="none"/></svg>
        </button>
        <button
          className="titlebar__btn titlebar__btn--close"
          onClick={() => appWindow.close()}
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
