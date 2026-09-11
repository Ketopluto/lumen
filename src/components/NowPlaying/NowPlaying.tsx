import React, { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { api, displayTitle, fromHistory, isLive, SOURCE_LABELS, type LiveStatus, type Wallpaper } from '../../api';
import { useThumbSrc } from '../../hooks';
import { useAppStore } from '../../store/appStore';
import './NowPlaying.css';

/** Bottom dock: what's on the desktop right now, with live wallpaper controls. */
export const NowPlaying: React.FC = () => {
  const [current, setCurrent] = useState<Wallpaper | null>(null);
  const [live, setLive] = useState<LiveStatus | null>(null);

  useEffect(() => {
    api
      .getHistory(1)
      .then((h) => h[0] && setCurrent(fromHistory(h[0])))
      .catch(() => {});
    api.liveStatus().then(setLive).catch(() => {});
    const unlisteners = [
      listen<Wallpaper>('wallpaper-changed', (e) => setCurrent(e.payload)),
      listen<LiveStatus>('live-state', (e) => setLive(e.payload)),
    ];
    return () => unlisteners.forEach((u) => u.then((f) => f()));
  }, []);

  if (!current) return null;
  return <Dock wallpaper={current} live={live} />;
};

const Dock: React.FC<{ wallpaper: Wallpaper; live: LiveStatus | null }> = ({ wallpaper, live }) => {
  const { src, video } = useThumbSrc(wallpaper, true);
  const apply = useAppStore((s) => s.apply);
  const source = SOURCE_LABELS[wallpaper.source];
  const title = displayTitle(wallpaper) ?? (source ? `${source} wallpaper` : 'Wallpaper');

  let status: React.ReactNode;
  let actions: React.ReactNode = null;
  if (isLive(wallpaper) && live?.path) {
    const autoPaused = live.paused && !live.manual_paused;
    status = (
      <span
        className="dock__status"
        title={autoPaused ? 'Resumes on its own when you leave the fullscreen app' : undefined}
      >
        <span className={`dock__dot ${live.paused ? 'dock__dot--paused' : ''}`} />
        {!live.paused ? 'Live' : autoPaused ? 'Paused automatically' : 'Paused'}
      </span>
    );
    actions = (
      <>
        <button className="btn btn--glass" onClick={() => api.setLivePaused(!live.manual_paused)}>
          {live.manual_paused ? 'Resume' : 'Pause'}
        </button>
        <button className="btn btn--glass" onClick={() => api.stopLive()}>
          Stop
        </button>
      </>
    );
  } else if (isLive(wallpaper)) {
    status = <span className="dock__status">Stopped</span>;
    actions = (
      <button className="btn btn--glass" onClick={() => apply(wallpaper)}>
        Play again
      </button>
    );
  } else {
    status = <span className="dock__status">Still image</span>;
  }

  return (
    <div className="dock" aria-label="Current wallpaper">
      {src && <div className="dock__ambient" style={{ backgroundImage: `url("${src}")` }} />}
      <div className="dock__thumb">
        {src ? (
          <img src={src} alt="" referrerPolicy="no-referrer" />
        ) : video ? (
          <video src={`${video}#t=1`} muted preload="metadata" />
        ) : null}
      </div>
      <div className="dock__text">
        <span className="dock__eyebrow">On your desktop</span>
        <span className="dock__title">{title}</span>
      </div>
      {status}
      {actions && <div className="dock__actions">{actions}</div>}
    </div>
  );
};
